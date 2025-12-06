const std = @import("std");

/// Standard I/O streams.
pub const StdIo = struct {
    in: *std.Io.Reader,
    out: *std.Io.Writer,
    err: *std.Io.Writer,
};

/// Returns whether string a and b are equal.
pub fn strEql(a: []const u8, b: []const u8) bool {
    return std.mem.eql(u8, a, b);
}

/// Returns whether string a has `prefix` prefix.
pub fn hasPrefix(a: []const u8, prefix: []const u8) bool {
    return std.mem.startsWith(u8, a, prefix);
}

/// Returns index of string in bs equal to a. If there is no match, -1 is
/// returned.
pub fn strMatch(a: []const u8, comptime bs: anytype) isize {
    inline for (bs, 0..bs.len) |b, i| {
        if (strEql(a, b)) return i;
    }
    return -1;
}

/// Prints fatal error before exiting.
pub fn fatalError(
    w: *std.Io.Writer,
    comptime fmt: []const u8,
    args: anytype,
) !noreturn {
    try w.print("Error: " ++ fmt ++ "\n", args);
    try w.flush();
    std.process.exit(1);
}

/// Prints fatal CLI error before exiting.
pub fn cliError(
    w: *std.Io.Writer,
    comptime fmt: []const u8,
    args: anytype,
) !noreturn {
    try w.print("Error: " ++ fmt ++ "\n", args);
    try w.print("\n", .{});
    try w.print("USAGE: allelua COMMAND [ARGS...]\n", .{});
    try w.print("\n", .{});
    try w.print("Run 'allelua -h' for more informations\n", .{});
    try w.flush();
    std.process.exit(1);
}
