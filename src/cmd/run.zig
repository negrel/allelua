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

    // Default to stdin.
    var lua_file = stdio.in;
    var lua_fpath: [:0]const u8 = "[stdin]";

    // Open file.
    var f: std.fs.File = undefined;
    var r: std.fs.File.Reader = undefined;
    var buffer: [4096]u8 = undefined;
    if (args.len > 0) {
        lua_fpath = args[0];
        f = std.fs.cwd().openFile(lua_fpath, .{}) catch |err| {
            try utils.fatalError(
                stdio.err,
                "failed to open file '{s}': {s}",
                .{ lua_fpath, @errorName(err) },
            );
        };
        r = f.reader(buffer[0..]);
        lua_file = &r.interface;
    }
    defer if (lua_file != stdio.in) f.close();

    var lua_args: []const []const u8 = &.{};

    if (args.len > 1) {
        if (utils.strEql(args[1], "--")) {
            lua_args = args[2..];
        } else {
            try utils.cliError(
                stdio.err,
                "unexpected argument after FILE argument: '{s}'",
                .{args[1]},
            );
        }
    }

    // Setup runtime.
    const rt = try Allelua.init(.{
        .allocator = &allocator,
    });
    defer rt.deinit();

    // Execute Lua file.
    rt.doFile(lua_file, lua_fpath, lua_args) catch |err| switch (err) {
        error.InvalidSyntax => {
            rt.L.dumpStack();
            try utils.fatalError(stdio.err, "invalid Lua code", .{});
        },
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
