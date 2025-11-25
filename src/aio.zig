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

    const Static = struct {
        fn callback(_: *zev.Io, op: *zev.Io.Op(OpData)) void {
            const lua: *zluajit.c.lua_State = @ptrCast(op.header.user_data.?);
            const L = zluajit.State.initFromCPointer(lua);
            const fut: *Future = @fieldParentPtr("op", op);

            // Retrieve associated nursery.
            const nursery = L.registryRef().rawGet(
                fut.nursery_ref,
                zluajit.TableRef,
            ).?;
            defer L.pop(1);

            // nursery.pending_count -= 1
            nursery.set(
                "pending_count",
                nursery.get("pending_count", zluajit.Integer).? - 1,
            );

            const ready = nursery.get("ready", zluajit.TableRef).?;
            defer L.pop(1);

            // nursery.ready[co] = io_results
            L.pushAnyType(fut.L);
            // TODO: construct result table.
            L.newTable();
            L.setTable(ready.ref.idx);

            // Remove reference to allow Lua's GC to collect L.
            L.unref(zluajit.Registry, fut.ref);

            // Free future.
            L.allocator().destroy(fut);
        }
    };

    const info = @typeInfo(OpData).@"struct";
    return zluajit.wrapFn(struct {
        fn submit(L: zluajit.State, aio: *AIO) !c_int {
            const fut = try L.allocator().create(Future);
            errdefer L.allocator().destroy(fut);
            fut.L = L;

            // Create reference to future to prevent GC.
            _ = L.pushState();
            fut.ref = try L.ref(zluajit.Registry);
            errdefer L.unref(zluajit.Registry, fut.ref);

            // Create reference to nursery.
            L.getGlobal("__allelua_nursery");
            fut.nursery_ref = try L.ref(zluajit.Registry);
            errdefer L.unref(zluajit.Registry, fut.nursery_ref);

            // TableRef to nursery.
            const nursery = L.globalRef().get(
                "__allelua_nursery",
                zluajit.TableRef,
            ).?;

            // nursery.pending_count += 1
            nursery.set(
                "pending_count",
                nursery.get("pending_count", zluajit.Integer).? + 1,
            );

            comptime var i = 2; // 2 as arg 1 is IO handle.
            inline for (info.fields) |f| {
                // OpData output fields has a default value.
                if (f.defaultValue() != null) break;

                // Check value.
                @field(&fut.op.data, f.name) = checkT(f.type, L, &i);
            }

            fut.op.header.code = OpData.op_code;
            fut.op.header.user_data = L.lua;
            fut.op.header.callback = @ptrCast(&Static.callback);
            fut.op.private = zev.Io.OpPrivateData(OpData).init(.{});

            // Submit.
            _ = aio.io.submit(&fut.op) catch |err| switch (err) {
                error.SubmissionQueueFull => {
                    _ = try aio.io.poll(.one);
                    try aio.io.submit(&fut.op);
                },
                else => return err,
            };

            // Yield.
            return L.yield(0);
        }
    }.submit);
}

inline fn checkT(T: type, L: zluajit.State, comptime narg: *comptime_int) T {
    const t: T = switch (T) {
        usize => @intCast(L.checkInteger(narg.*)),
        else => unreachable,
    };

    narg.* += 1;

    return t;
}
