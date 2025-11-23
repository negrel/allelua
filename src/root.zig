const std = @import("std");
pub const zluajit = @import("zluajit");

const aio = @import("aio.zig");

pub const Allelua = struct {
    const Self = @This();

    const registry_key = "__allelua.Allelua";

    L: zluajit.State,

    pub fn init(options: zluajit.State.Options) !*Self {
        var self = try options.allocator.*.create(Self);
        errdefer options.allocator.*.destroy(self);

        // Setup Lua.
        self.L = try zluajit.State.init(options);
        errdefer self.L.deinit();

        // Load libs.
        self.L.openLibs();

        // Setup runtime.
        self.setupRuntime() catch {
            self.L.dumpStack();
            @panic("error setting up the runtime");
        };

        // Store reference on registry.
        self.L.pushLightUserData(self);
        self.L.setField(zluajit.Registry, Self.registry_key);

        return self;
    }

    fn setupRuntime(self: *Self) !void {
        self.L.pushZFunction(struct {
            fn print(L: zluajit.State) void {
                L.dumpStack();
            }
        }.print);
        self.L.setGlobal("print");

        try self.L.doString(
            @embedFile("./embed/99_start.lua"),
            "allelua.__start",
        );
    }

    pub fn deinit(self: *Self) void {
        const alloc = self.L.allocator();
        self.L.deinit();
        alloc.destroy(self);
    }

    pub fn doFile(
        self: *Self,
        r: *std.Io.Reader,
        chunkName: [*c]const u8,
        args: []const []const u8,
    ) !void {
        const Reader = struct {
            r: *std.Io.Reader,
            err: std.Io.Reader.Error!void = undefined,

            fn luaReader(
                _: ?*zluajit.c.lua_State,
                ptr: ?*anyopaque,
                size: [*c]usize,
            ) callconv(.c) [*c]const u8 {
                const reader: *@This() = @ptrCast(@alignCast(ptr));

                _ = reader.r.tossBuffered();
                reader.r.fillMore() catch |err| {
                    if (err == error.EndOfStream) return null;
                };

                const buf = reader.r.buffered();
                size.* = buf.len;
                return buf.ptr;
            }
        };

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

        // Arg 3 is user code function.
        {
            // Load file.
            var reader = Reader{ .r = r };
            try self.L.load(Reader.luaReader, @ptrCast(&reader), chunkName);

            // Check for I/O error reading the file.
            try reader.err;
        }

        // Start executing code.
        var status = try self.L.@"resume"(3);

        // Event loop.
        while (status != .ok) {
            const done = try io.io.poll(.all);
            self.L.pushInteger(@intCast(done));
            status = try self.L.@"resume"(1);
        }

        // Poll remaining I/O operations.
        _ = try io.io.poll(.all);
    }
};
