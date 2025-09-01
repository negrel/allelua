const std = @import("std");
pub const zluajit = @import("zluajit");
pub const xev = @import("xev");

const print = std.debug.print;

pub const Allelua = struct {
    const Self = @This();

    L: zluajit.State,
    xev: xev.Loop,

    pub const RegistryKey = "allelua";

    pub fn init(options: zluajit.State.Options) !*Self {
        var self = try options.allocator.*.create(Self);
        errdefer options.allocator.*.destroy(self);

        // Setup Lua.
        self.L = try zluajit.State.init(options);
        errdefer self.L.deinit();

        // Setup xev event loop.
        self.xev = try xev.Loop.init(.{});
        errdefer self.xev.deinit();

        // Load libs.
        self.L.openLibs();
        self.L.doString(@embedFile("./embed/compat.lua"), "compat") catch unreachable;
        self.L.doString(@embedFile("./embed/table.lua"), "table") catch unreachable;
        self.L.doString(@embedFile("./embed/coroutine.lua"), "coroutine") catch unreachable;

        // Store reference on registry.
        self.L.pushLightUserData(self);
        self.L.setField(zluajit.Registry, Self.RegistryKey);

        self.L.pushAnyType(struct {
            fn sleep(L: zluajit.State) !c_int {
                const secs = L.checkNumber(1);
                const ms: u64 = @intFromFloat(secs * 1000);

                L.getField(zluajit.Registry, Allelua.RegistryKey);
                var al: *Allelua = @constCast(@ptrCast(@alignCast(L.toPointer(-1).?)));
                var alloc = al.L.allocator();

                const SleepTask = struct {
                    L: zluajit.State,
                    c: xev.Completion,
                    timer: xev.Timer,
                    nursery_ref: c_int,

                    fn callback(
                        t: ?*@This(),
                        _: *xev.Loop,
                        _: *xev.Completion,
                        err: anyerror!void,
                    ) xev.CallbackAction {
                        const task = t.?;
                        defer task.L.allocator().destroy(task);

                        // Retrieve nursery from registry.
                        task.L.pushAnyType(task.nursery_ref);
                        task.L.getTable(zluajit.Registry);
                        task.L.unref(zluajit.Registry, task.nursery_ref);

                        // TODO: mark routine as dead.
                        err catch unreachable;

                        // Mark routine as ready.
                        task.L.getField(-1, "_wake");
                        task.L.pushValue(-2);
                        _ = task.L.pushState();
                        task.L.call(2, 0);

                        return .disarm;
                    }
                };

                var task = try alloc.create(SleepTask);
                task.L = L;
                task.c = undefined;
                task.timer = try xev.Timer.init();

                // Async yield.
                L.getGlobal("coroutine");
                L.getField(-1, "_nursery");

                // Create reference to nursery.
                L.pushValue(-1);
                task.nursery_ref = try L.ref(zluajit.Registry);

                task.timer.run(
                    &al.xev,
                    &task.c,
                    ms,
                    SleepTask,
                    task,
                    SleepTask.callback,
                );

                print("sleeping {}ms\n", .{ms});
                L.dumpStack();

                // Async yield.
                L.getGlobal("coroutine");
                L.getField(-1, "_nursery");
                return L.yield(1);
            }
        }.sleep);

        return self;
    }

    pub fn deinit(self: *Self) void {
        const alloc = self.L.allocator();
        self.L.deinit();
        self.xev.deinit();
        alloc.destroy(self);
    }

    pub fn doFile(
        self: *Self,
        filename: [*c]const u8,
    ) !void {
        const th = self.L.newThread();
        try th.loadFile(filename);
        while (true) {
            // Run thread until it yields.
            switch (try th.@"resume"(0)) {
                .ok => break,
                .yield => {},
            }

            print("polling xev\n", .{});
            // Poll event loop until there is at least one completion.
            try self.xev.run(.once);
        }
    }
};
