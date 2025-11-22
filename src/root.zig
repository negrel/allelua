const std = @import("std");
pub const zluajit = @import("zluajit");

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

        self.L.openLibs();

        // Store reference on registry.
        self.L.pushLightUserData(self);
        self.L.setField(zluajit.Registry, Self.registry_key);

        return self;
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

        // Load file.
        var reader = Reader{ .r = r };
        try self.L.load(Reader.luaReader, @ptrCast(&reader), chunkName);

        // Check for I/O error.
        try reader.err;

        // Execute file.
        try self.L.pCall(0, 0, 0);
    }
};
