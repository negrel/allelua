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
    aio: *aio.AIO,

    pub fn init(options: Options) !*Self {
        var self = try options.lua.allocator.*.create(Self);
        errdefer options.lua.allocator.*.destroy(self);

        self.mode = options.mode;

        // Setup Lua.
        self.L = try zluajit.State.init(options.lua);
        errdefer self.L.deinit();

        // Load libs.
        self.L.openLibs();

        // Setup event loop.
        self.aio = try aio.AIO.init(self.L);
        const aio_vref = zluajit.ValueRef.init(self.L, -1);
        defer self.L.pop(1);

        // Setup runtime.
        self.setupRuntime(aio_vref) catch {
            self.L.dumpStack();
            @panic("error setting up the runtime");
        };

        return self;
    }

    fn setupRuntime(self: *Self, aio_vref: zluajit.ValueRef) !void {
        const z = self.L.newTableRef();

        z.set("io", aio_vref);
        z.set("mode", self.mode);
        z.set(
            "path_max",
            @as(usize, @intCast(std.posix.PATH_MAX)),
        );
        z.set("path_separator", std.fs.path.sep_str);
        z.set(
            "at_fdcwd",
            @as(zluajit.Integer, std.posix.AT.FDCWD),
        );

        z.set("dump", struct {
            fn dump(L: zluajit.State) void {
                L.dumpStack();
            }
        }.dump);
        z.set("raise", zluajit.c.lua_error);

        z.set(
            "resolve_path",
            struct {
                fn resolvePath(L: zluajit.State, path: []const u8) !c_int {
                    var splitter = std.mem.splitScalar(
                        u8,
                        path,
                        std.fs.path.delimiter,
                    );
                    var paths: std.ArrayList([]const u8) = .{};
                    defer paths.deinit(L.allocator().*);

                    while (splitter.next()) |p| {
                        try paths.append(L.allocator().*, p);
                    }

                    const real = try std.fs.path.resolve(
                        L.allocator().*,
                        paths.items,
                    );
                    defer L.allocator().free(real);

                    L.pushString(real);
                    return 1;
                }
            }.resolvePath,
        );

        z.set(
            "real_path",
            struct {
                fn realPath(L: zluajit.State, path: []const u8) !c_int {
                    const real = try std.fs.cwd().realpathAlloc(L.allocator().*, path);
                    L.pushString(real);
                    L.allocator().free(real);
                    return 1;
                }
            }.realPath,
        );
        z.set("raw_getmetatable", struct {
            fn rawGetMetaTable(L: zluajit.State) c_int {
                if (L.getMetaTable(-1)) {
                    return 1;
                }
                return 0;
            }
        }.rawGetMetaTable);

        self.L.setGlobal("_z");

        try self.L.doString(
            @embedFile("./embed/00_debug.lua"),
            "allelua.debug",
        );
        try self.L.doString(
            @embedFile("./embed/00_error.lua"),
            "allelua.error",
        );
        try self.L.doString(
            @embedFile("./embed/00_nursery.lua"),
            "allelua.nursery",
        );
        try self.L.doString(
            @embedFile("./embed/00_string.lua"),
            "string",
        );
        try self.L.doString(
            @embedFile("./embed/00_table.lua"),
            "table",
        );
        try self.L.doString(
            @embedFile("./embed/00_time.lua"),
            "time",
        );
        try self.L.doString(
            @embedFile("./embed/10_fs.lua"),
            "allelua.fs",
        );
        try self.L.doString(
            @embedFile("./embed/10_process.lua"),
            "allelua.process",
        );
        try self.L.doString(
            @embedFile("./embed/20_sh.lua"),
            "allelua.sh",
        );
        try self.L.doString(
            @embedFile("./embed/98_import.lua"),
            "allelua.__import",
        );
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
        fpath: ?[]const u8,
        args: []const []const u8,
    ) !void {
        // __start is our Lua entrypoint. This will setup environments and
        // execute user code.
        const z = self.L.globalRef().getField(
            "_z",
            zluajit.TableRef,
        ).?;
        _ = z.get("__start", zluajit.ValueRef);

        // Arg 1 is CLI args as a Lua table.
        self.L.newTable();
        for (args, 0..args.len) |arg, i| {
            self.L.pushInteger(@intCast(i));
            self.L.pushString(arg);
            self.L.setTable(-3);
        }

        // Arg 2 is file.
        if (fpath) |p| self.L.pushString(p) else self.L.pushNil();

        // Start executing code.
        var status = try self.L.@"resume"(2);

        // Event loop.
        while (status == .yield) {
            // Poll event loop for I/O completions.
            const done = try self.aio.io.poll(.all);

            // Resume work.
            self.L.pushInteger(@intCast(done));
            status = try self.L.@"resume"(1);
        }

        // Poll remaining I/O operations.
        _ = try self.aio.io.poll(.all);
    }
};
