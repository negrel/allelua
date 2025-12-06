const std = @import("std");
const Allelua = @import("allelua").Allelua;
const utils = @import("utils.zig");

/// Run a Lua program.
pub fn run(
    a: std.mem.Allocator,
    stdio: utils.StdIo,
    args: []const [:0]const u8,
) !void {
    var allocator = a;

    var lua_fpath: ?[]const u8 = null;
    var lua_args: []const []const u8 = &.{};
    var mode: Allelua.Mode = .production;

    for (0..args.len) |i| {
        if (utils.strEql(args[i], "--debug")) {
            mode = .debug;
        } else if (utils.hasPrefix(args[i], "--")) {
            try utils.cliError(stdio.err, "unrecognized flag {s}", .{args[i]});
        } else {
            if (!utils.strEql(args[i], "-")) {
                lua_fpath = args[i];
            }
            lua_args = args[i + 1 ..];
            break;
        }
    }

    // Setup runtime.
    const rt = try Allelua.init(.{
        .mode = mode,
        .lua = .{
            .allocator = &allocator,
        },
    });
    defer rt.deinit();

    // Execute Lua file.
    rt.doFile(lua_fpath, lua_args) catch |err| switch (err) {
        error.OutOfMemory => return err,
        error.Runtime => {
            rt.L.dumpStack();
            try utils.fatalError(stdio.err, "runtime error", .{});
        },
        else => try utils.fatalError(
            stdio.err,
            "read error: {s}",
            .{@errorName(err)},
        ),
    };
}
