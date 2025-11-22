const std = @import("std");

const utils = @import("utils.zig");

/// CLI main function.
pub fn main(
    allocator: std.mem.Allocator,
    stdio: utils.StdIo,
    args: []const [:0]const u8,
) !void {
    // No args provided.
    if (args.len == 0) {
        try utils.cliError(
            stdio.err,
            "allelua requires at least one command to execute",
            .{},
        );
    }

    const cmd: []const u8 = args[0];

    if (utils.strMatch(cmd, .{ "-h", "--help" }) >= 0) {
        try usage(stdio.out);
        std.process.exit(0);
    }
    if (utils.strEql(cmd, "help")) {
        if (args.len == 1) {
            try usage(stdio.out);
            std.process.exit(0);
        }

        return @import("help.zig").help(allocator, stdio, args[1..]);
    }
    if (utils.strEql(cmd, "run"))
        return @import("run.zig").run(allocator, stdio, args[1..]);

    try utils.cliError(stdio.err, "unknown command '{s}'", .{cmd});
}

fn usage(w: *std.Io.Writer) !void {
    try w.print("allelua - a Lua runtime blessed by programming gods.\n", .{});
    try w.print("Alexandre Negrel <alexandre@negrel.dev>\n", .{});
    try w.print("\n", .{});
    try w.print("USAGE:\n", .{});
    try w.print("   allelua COMMAND [ARGS...]\n", .{});
    try w.print("\n", .{});
    try w.print("COMMANDS:\n", .{});
    try w.print("   help                         Print this menu.\n", .{});
    try w.print("   help COMMAND                 Print command's help menu.\n", .{});
    try w.print("   run  FILE                    Run a Lua file.\n", .{});
    try w.print("\n", .{});
    try w.print("Source code: https://github.com/negrel/allelua\n", .{});
    try w.flush();
}

fn cmdError(
    w: *std.Io.Writer,
    comptime fmt: []const u8,
    args: anytype,
) !noreturn {
    try w.print("Error: " ++ fmt ++ "\n", args);
    try w.flush();
    std.process.exit(1);
}
