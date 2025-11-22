const std = @import("std");
const utils = @import("utils.zig");

/// The help command.
pub fn help(
    allocator: std.mem.Allocator,
    stdio: utils.StdIo,
    args: []const [:0]const u8,
) !void {
    _ = allocator;

    const cmd = args[0];
    if (utils.strEql(cmd, "help")) {
        try stdio.out.print("bro seriously ?\n", .{});
        try stdio.out.flush();
        std.process.exit(0);
    }

    try utils.cliError(stdio.err, "unknown command '{s}'", .{cmd});
}
