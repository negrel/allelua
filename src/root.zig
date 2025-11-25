const std = @import("std");
pub const zluajit = @import("zluajit");

const aio = @import("aio.zig");

pub const Allelua = struct {
    const Self = @This();

    pub const Mode = enum {
        debug,
        production,
    };

    pub const Options = struct {
        mode: Mode = .debug,
        lua: zluajit.State.Options = .{},
    };

    L: zluajit.State,
    mode: Mode,

    pub fn init(options: Options) !*Self {
        var self = try options.lua.allocator.*.create(Self);
        errdefer options.lua.allocator.*.destroy(self);

        self.mode = options.mode;

        // Setup Lua.
        self.L = try zluajit.State.init(options.lua);
        errdefer self.L.deinit();

        // Load libs.
        self.L.openLibs();

        // Setup runtime.
        self.setupRuntime() catch {
            self.L.dumpStack();
            @panic("error setting up the runtime");
        };

        return self;
    }

    fn setupRuntime(self: *Self) !void {
        self.L.globalRef().set("mode", self.mode);

        // dump() is noop in prod.
        if (self.mode == .production) {
            self.L.pushZFunction(struct {
                fn dump(_: zluajit.State) void {}
            }.dump);
        } else {
            self.L.pushZFunction(struct {
                fn dump(L: zluajit.State) void {
                    L.dumpStack();
                }
            }.dump);
        }
        self.L.setGlobal("dump");

        self.L.pushZFunction(struct {
            fn resolvePath(L: zluajit.State, path: []const u8) !c_int {
                var splitter = std.mem.splitScalar(u8, path, std.fs.path.delimiter);
                var paths: std.ArrayList([]const u8) = .{};
                defer paths.deinit(L.allocator().*);

                while (splitter.next()) |p| {
                    try paths.append(L.allocator().*, p);
                }

                const real = try std.fs.path.resolve(L.allocator().*, paths.items);
                defer L.allocator().free(real);

                L.pushString(real);
                return 1;
            }
        }.resolvePath);
        self.L.setGlobal("resolve_path");

        self.L.pushZFunction(struct {
            fn realPath(L: zluajit.State, path: []const u8) !c_int {
                const real = try std.fs.cwd().realpathAlloc(L.allocator().*, path);
                L.pushString(real);
                L.allocator().free(real);
                return 1;
            }
        }.realPath);
        self.L.setGlobal("real_path");

        try self.L.doString(
            @embedFile("./embed/00_table.lua"),
            "table",
        );
        try self.L.doString(
            @embedFile("./embed/00_string.lua"),
            "string",
        );
        try self.L.doString(
            @embedFile("./embed/00_time.lua"),
            "time",
        );
        try self.L.doString(
            @embedFile("./embed/00_nursery.lua"),
            "nursery",
        );
        try self.L.doString(
            @embedFile("./embed/98_import.lua"),
            "allelua.__import",
        );
        try self.L.doString(
            @embedFile("./embed/99_start.lua"),
            "allelua.__start",
        );

        // Set Lua strings metatable to string module.
        self.L.pushString("");
        self.L.getGlobal("string");
        self.L.setMetaTable(-2);
    }

    pub fn deinit(self: *Self) void {
        const alloc = self.L.allocator();
        self.L.deinit();
        alloc.destroy(self);
    }

    pub fn doFile(
        self: *Self,
        fpath: ?[]const u8,
        args: []const []const u8,
    ) !void {
        // __start is our Lua entrypoint. This will setup environments and
        // execute user code.
        self.L.getGlobal("__start");

        // Arg 1 is aio handle.
        const io = try aio.AIO.init(self.L);

        // Arg 2 is CLI args as a Lua table.
        self.L.newTable();
        for (args, 0..args.len) |arg, i| {
            self.L.pushInteger(@intCast(i));
            self.L.pushString(arg);
            self.L.setTable(-3);
        }

        // Arg 3 is file.
        if (fpath) |p| self.L.pushString(p) else self.L.pushNil();

        // Start executing code.
        var status = try self.L.@"resume"(3);

        // Event loop.
        while (status == .yield) {
            // Poll event loop for I/O completions.
            const done = try io.io.poll(.all);

            // Resume work.
            self.L.pushInteger(@intCast(done));
            status = try self.L.@"resume"(1);
        }

        // Poll remaining I/O operations.
        _ = try io.io.poll(.all);
    }
};
