const std = @import("std");
pub const zluajit = @import("zluajit");
pub const xev = @import("xev");

const print = std.debug.print;

pub const Allelua = struct {
    const Self = @This();

    const registry_key = "allelua";

    L: zluajit.State,
    xev: xev.Loop,

    pub fn init(options: zluajit.State.Options) !*Self {
        var self = try options.allocator.*.create(Self);
        errdefer options.allocator.*.destroy(self);

        // Setup Lua.
        self.L = try zluajit.State.init(options);
        errdefer self.L.deinit();

        // Setup xev event loop.
        self.xev = try xev.Loop.init(.{});
        errdefer self.xev.deinit();

        self.openLibs();

        // Store reference on registry.
        self.L.pushLightUserData(self);
        self.L.setField(zluajit.Registry, Self.registry_key);

        return self;
    }

    /// Retrieves reference from Lua registry.
    // pub fn initFromLuaRegistry(L: zluajit.State) *Self {
    // L.getField(index: c_int, k: [*c]const u8)
    // }

    pub fn deinit(self: *Self) void {
        const alloc = self.L.allocator();
        self.L.deinit();
        self.xev.deinit();
        alloc.destroy(self);
    }

    pub fn openLibs(self: *Self) void {
        self.L.openLibs();

        Flag.newTable(self.L);
        self.L.setGlobal("refs");
        defer {
            self.L.pushNil();
            self.L.setGlobal("refs");
        }

        // Layer to improve compatibility with other Lua versions.
        self.L.doString(@embedFile("./embed/compat.lua"), "compat") catch unreachable;

        // Extensions.
        {
            self.L.doString(@embedFile("./embed/table.lua"), "table") catch unreachable;
            self.L.doString(@embedFile("./embed/coroutine.lua"), "coroutine") catch unreachable;
        }

        self.L.setGlobalAnyType("sleep", struct {
            fn sleep(L: zluajit.State) !c_int {
                const secs = L.checkNumber(1);
                const ms: u64 = @intFromFloat(secs * 1000);

                L.getField(zluajit.Registry, Allelua.registry_key);
                var al: *Allelua = @constCast(@ptrCast(@alignCast(L.toPointer(-1).?)));

                const SleepTask = Future(struct {
                    c: xev.Completion,
                    timer: xev.Timer,
                });

                const callback = struct {
                    fn callback(
                        t: ?*SleepTask,
                        _: *xev.Loop,
                        _: *xev.Completion,
                        err: anyerror!void,
                    ) xev.CallbackAction {
                        const task = t.?;
                        defer task.deinit();

                        task.wake(.{});

                        // TODO: mark routine as dead or raise error from coroutine.
                        err catch unreachable;

                        return .disarm;
                    }
                }.callback;

                const task = SleepTask.init(L);
                task.data.timer = try xev.Timer.init();

                task.data.timer.run(
                    &al.xev,
                    &task.data.c,
                    ms,
                    SleepTask,
                    task,
                    callback,
                );

                return task.yield();
            }
        }.sleep);
    }

    pub fn doFile(
        self: *Self,
        filename: ?[]const u8,
    ) !void {
        const th = self.L.newThread();
        th.doString(
            @embedFile("./embed/entrypoint.lua"),
            "entrypoint",
        ) catch unreachable;
        _ = filename;

        while (true) {
            // Run thread until it yields.
            switch (try th.@"resume"(0)) {
                .ok => break,
                .yield => {},
            }

            // Poll event loop until there is at least one completion.
            try self.xev.run(.once);
        }
    }
};

/// Future represents an asynchronous Zig computation called from Lua
/// thread.
pub fn Future(comptime T: type) type {
    return struct {
        const Self = @This();

        const zluajitTName = "_Future";

        L: zluajit.State,
        data: T,
        ref: c_int,
        nursery: c_int,

        /// Creates a new Future and pushes it on top of the stack.
        pub fn init(L: zluajit.State) *Self {
            const self = L.newUserData(Self);
            self.L = L;

            if (L.newMetaTableRef(Self)) |mt| {
                mt.setField("__gc", Self.deinit);
                mt.setField("__metatable", false);
            }
            L.setMetaTable(-2);

            // Prevent garbage collection of self.
            self.ref = L.ref(zluajit.Registry) catch unreachable;

            // Retrieve nursery if any.
            self.L.getGlobal("coroutine");
            self.L.getField(-1, "_nursery");
            defer self.L.pop(2);
            self.nursery = L.ref(zluajit.Registry) catch unreachable;

            return self;
        }

        /// Yields future's thread. This will move the thread into a suspended
        /// state until the Future is dropped (typically once async work is
        /// done)
        pub fn yield(self: *const Self) c_int {
            self.L.pushLightUserData(Flag.async_yield.toLightUserData());
            return self.L.yield(1);
        }

        /// Push provided data onto stack of future's thread.
        pub fn wake(self: *const Self, data: anytype) void {
            // Retrieve nursery from registy.
            self.L.pushAnyType(self.nursery);
            self.L.getTable(zluajit.Registry);

            // Prepare to call nursery:_wake().
            self.L.getField(-1, "_wake");
            self.L.pushValue(-2);
            _ = self.L.pushState();

            // Push wake args.
            const info = @typeInfo(@TypeOf(data)).@"struct";
            inline for (info.fields) |f| {
                self.L.pushAnyType(@field(data, f.name));
            }

            self.L.pCall(2 + info.fields.len, 0, 0) catch @panic("logic error");
        }

        /// Clean up future state and mark it as ready to be resumed.
        pub fn deinit(self: *const Self) void {
            // Remove references so data can be GC.
            self.L.unref(zluajit.Registry, self.ref);
            self.L.unref(zluajit.Registry, self.nursery);
        }
    };
}

/// Flag defines unique integer / pointer shared between Zig and Lua
/// runtime (as light user data).
const Flag = enum {
    async_yield,

    /// Creates a table containing all variants on top of the stack.
    fn newTable(L: zluajit.State) void {
        L.newTable();
        const tab = L.toAnyType(-1, zluajit.TableRef).?;
        inline for (@typeInfo(Flag).@"enum".fields) |d| {
            tab.setField(d.name, Flag.toLightUserData(@enumFromInt(d.value)));
        }
    }

    inline fn toLightUserData(self: Flag) *anyopaque {
        return @ptrFromInt(@as(usize, @intFromEnum(self)) + 1);
    }
};
