//! This file contains logic and types behind asynchronous Lua I/O.

const std = @import("std");

const zev = @import("zev");
const zluajit = @import("zluajit");

/// I/O handle Lua userdata.
pub const AIO = struct {
    const Self = @This();
    const registry_key = "__allelua.AIO";

    L: zluajit.State,
    allocator: std.mem.Allocator,
    ref: c_int,
    io: zev.Io,

    pub fn init(L: zluajit.State) !*Self {
        const alloc = L.allocator().*;
        const self = L.newUserData(Self);

        self.L = L;
        self.allocator = alloc;
        try self.io.init(.{});
        errdefer self.io.deinit();

        // Prevent GC of AIO.
        L.pushValue(-1);
        self.ref = try L.ref(zluajit.Registry);
        errdefer L.unref(zluajit.Registry, self.ref);

        // Store in registry table.
        const registry = L.registryRef();
        registry.set(registry_key, @as(*anyopaque, @ptrCast(self)));
        errdefer registry.set(registry_key, null);

        if (L.newMetaTableRef(Self)) |mt| {
            mt.set("__gc", Self.deinit);
            mt.set("__metatable", "allelua.AIO");

            const index = L.newTableRef();
            defer L.pop(1);
            mt.set("__index", index);

            index.set("poll", struct {
                fn poll(aio: *Self, mode: zev.PollMode) !zluajit.Integer {
                    const completed = try aio.io.poll(mode);
                    return std.math.cast(zluajit.Integer, completed) orelse
                        std.math.maxInt(zluajit.Integer);
                }
            }.poll);

            // I/O operations.
            index.set("sleep", luaSubmit(zev.Sleep));
            index.set("openat", luaSubmit(zev.OpenAt));
            index.set("close", luaSubmit(zev.Close));
            index.set("pread", luaSubmit(zev.PRead));
            index.set("pwrite", luaSubmit(zev.PWrite));
            index.set("unlinkat", luaSubmit(zev.UnlinkAt));
            index.set("fsync", luaSubmit(zev.FSync));
            index.set("fstat", luaSubmit(zev.FStat));
            index.set("getcwd", luaSubmit(zev.GetCwd));
            index.set("chdir", luaSubmit(zev.ChDir));
            index.set("spawn", luaSubmit(zev.Spawn));
            index.set("waitpid", luaSubmit(zev.WaitPid));
        }
        L.setMetaTable(-2);

        return self;
    }

    fn deinit(self: *Self) void {
        self.io.deinit();
    }

    fn fromLuaRegistry(L: zluajit.State) ?*Self {
        return L.registryRef().get(registry_key, *Self);
    }
};

/// Generate Lua CFunction to submit I/O operation of given type.
inline fn luaSubmit(OpData: type) zluajit.CFunction {
    const Future = struct {
        L: zluajit.State,
        nursery_ref: c_int,
        op: zev.Io.Op(OpData),

        // Ref to prevent Lua's GC to collect L.
        ref: c_int,
    };

    const info = @typeInfo(OpData).@"struct";
    const Static = struct {
        fn callback(_: *zev.Io, op_h: *zev.OpHeader) void {
            const op = zev.Io.Op(OpData).fromHeader(op_h);
            const lua: *zluajit.c.lua_State = @ptrCast(op.header.user_data.?);
            const L = zluajit.State.initFromCPointer(lua);
            const fut: *Future = @fieldParentPtr("op", op);

            // Retrieve associated nursery.
            const nursery = L.registryRef().rawGet(
                fut.nursery_ref,
                zluajit.TableRef,
            ).?;
            defer L.pop(1);

            // Extract data from zev.Io operation.
            const args = L.newTableRef();
            pushResultT(L, @TypeOf(op.data.result), op.data.result);
            args.rawSet(
                @as(c_int, @intCast(1)),
                zluajit.ValueRef.init(L, args.ref.idx + 1),
            );
            args.rawSet(
                @as(c_int, @intCast(2)),
                zluajit.ValueRef.init(L, args.ref.idx + 2),
            );
            defer L.pop(3);

            // Mark future as ready in nursery.
            nurseryReady(nursery, fut.L, args);

            // Remove reference to allow Lua's GC to collect L.
            L.unref(zluajit.Registry, fut.ref);

            // Free future.
            L.allocator().destroy(fut);
        }
    };

    return zluajit.wrapFn(struct {
        fn submit(L: zluajit.State, aio: *AIO) !void {
            const fut = try L.allocator().create(Future);
            errdefer L.allocator().destroy(fut);
            fut.L = L;

            // Prepare zev.Io operation.
            var i: c_int = 2; // 2 as arg 1 is IO handle.
            inline for (info.fields) |f| {
                // OpData output fields has a default value.
                if (f.defaultValue() == null) {
                    // Check value.
                    @field(&fut.op.data, f.name) = checkT(f.type, L, &i);
                }
            }

            // Create reference to future to prevent GC.
            _ = L.pushState();
            fut.ref = try L.ref(zluajit.Registry);
            errdefer L.unref(zluajit.Registry, fut.ref);

            // Retrieve Nursery class.
            const nursery_class = L.globalRef().get(
                "_z",
                zluajit.TableRef,
            ).?.rawGet(
                "Nursery",
                zluajit.TableRef,
            ).?;

            defer L.pop(1);

            // Extract Nursery.running.
            _ = nursery_class.rawGet("running", zluajit.TableRef).?;
            fut.nursery_ref = try L.ref(zluajit.Registry);
            errdefer L.unref(zluajit.Registry, fut.nursery_ref);

            fut.op.header.code = OpData.op_code;
            fut.op.header.user_data = L.lua;
            fut.op.header.callback = @ptrCast(&Static.callback);
            fut.op.private = zev.Io.OpPrivateData(OpData).init(.{});

            // Submit it.
            _ = aio.io.submit(&fut.op) catch |err| switch (err) {
                error.SubmissionQueueFull => {
                    _ = try aio.io.poll(.one);
                    try aio.io.submit(&fut.op);
                },
                else => return err,
            };
        }
    }.submit);
}

inline fn checkT(T: type, L: zluajit.State, narg: *c_int) T {
    const t: T = switch (T) {
        usize => @intCast(L.checkInteger(narg.*)),
        std.fs.File => return .{
            .handle = checkT(std.fs.File.Handle, L, narg),
        },
        []u8 => {
            // Retrieve ptr.
            const ptr = L.checkCData(narg.*);
            narg.* += 1;

            // Retrieve buffer length.
            const len = checkT(usize, L, narg);

            return ptr[0..len];
        },
        []const u8 => {
            if (L.valueType(narg.*)) |vtype| {
                switch (vtype) {
                    .string => {
                        const str = L.checkString(narg.*);
                        narg.* += 1;
                        const len = checkT(usize, L, narg);
                        return str[0..len];
                    },
                    .cdata => return @constCast(checkT([]u8, L, narg)),
                    else => {},
                }
            }

            L.argError(narg.*, "expected string or string.Buffer");
        },
        [*:null]const ?[*:0]const u8 => T: {
            const allocator = L.allocator().*;
            var result: std.ArrayList(?[*:0]const u8) = .{};

            const t = narg.*;
            L.checkValueType(t, .table);

            L.pushNil(); // first key
            while (L.next(t)) {
                // Value is a string.
                if (L.toString(-1)) |envvar| {
                    result.append(
                        allocator,
                        @ptrCast(envvar.ptr),
                    ) catch |err| L.raiseError(err);
                }

                // removes 'value'; keeps 'key' for next iteration
                L.pop(1);
            }

            // Sentinel null.
            result.append(allocator, null) catch |err| L.raiseError(err);
            break :T @ptrCast(result.items.ptr);
        },
        std.fs.Dir => return .{ .fd = checkT(std.fs.Dir.Handle, L, narg) },
        std.posix.fd_t => @intCast(L.checkInteger(narg.*)),
        zev.OpenAt.Options => {
            const options = checkT(zluajit.TableRef, L, narg);
            return .{
                .read = options.get("read", bool) orelse false,
                .write = options.get("write", bool) orelse false,
                .append = options.get("append", bool) orelse false,
                .truncate = options.get("truncate", bool) orelse false,
                .create = options.get("create", bool) orelse false,
                .create_new = options.get("create_new", bool) orelse false,
            };
        },
        else => L.checkAnyType(narg.*, T),
    };

    narg.* += 1;

    return t;
}

inline fn pushResultT(L: zluajit.State, comptime T: type, v: T) void {
    const info = @typeInfo(T);
    switch (info) {
        .error_union => |i| {
            const payload = v catch |err| {
                L.pushBool(false);
                L.pushAnyType(@errorName(err));
                return;
            };
            return pushResultT(L, i.payload, payload);
        },
        else => {},
    }

    L.pushBool(true);
    switch (T) {
        void => L.pushNil(),
        std.fs.File => L.pushAnyType(v.handle),
        std.fs.File.Stat => {
            const stat = L.newTableRef();
            stat.set("inode", v.inode);
            stat.set("size", v.size);
            stat.set("mode", v.mode);
            stat.set("kind", v.kind);
            stat.set("atime", v.atime);
            stat.set("mtime", v.mtime);
            stat.set("ctime", v.ctime);
        },
        zev.Spawn.Result => {
            const r = L.newTableRef();
            if (@TypeOf(v.pid) != void) {
                r.set("pid", v.pid);
            }
            r.set("stdin", v.stdin.handle);
            r.set("stdout", v.stdout.handle);
            r.set("stderr", v.stderr.handle);
        },
        []u8 => L.pushString(v),
        else => L.pushAnyType(v),
    }
}

/// Prepare L to be resumed on next poll of nursery n.
/// This function implementation MUST be kept in sync with the Lua version.
fn nurseryReady(
    n: zluajit.TableRef,
    L: zluajit.State,
    args: zluajit.TableRef,
) void {
    var nursery: ?zluajit.TableRef = n;
    var co: ?zluajit.State = L;

    while (nursery) |nur| {
        const pending = nur.rawGet("pending", zluajit.TableRef).?;
        pending.rawSet(co.?, zluajit.nil);
        const ready = nur.rawGet("ready", zluajit.TableRef).?;
        ready.rawSet(co.?, args);
        nur.ref.L.pop(2);

        nursery = nur.rawGet("parent", zluajit.TableRef);
        co = nur.rawGet("co", zluajit.State);
    }
}
