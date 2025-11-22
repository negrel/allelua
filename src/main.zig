const std = @import("std");
const Allelua = @import("allelua").Allelua;

pub fn main() !void {
    const utils = @import("cmd/utils.zig");

    const allocator = std.heap.c_allocator;

    // Retrieve args.
    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    // Setup stdio streams.
    var in_buffer: [4096]u8 = undefined;
    var out_buffer: [4096]u8 = undefined;
    var err_buffer: [4096]u8 = undefined;
    var stdin = std.fs.File.stdin().reader(in_buffer[0..]);
    var stdout = std.fs.File.stdout().writer(out_buffer[0..]);
    var stderr = std.fs.File.stdout().writer(err_buffer[0..]);

    try @import("cmd/main.zig").main(
        allocator,
        utils.StdIo{
            .in = &stdin.interface,
            .out = &stdout.interface,
            .err = &stderr.interface,
        },
        args[1..],
    );
}

// fn handleLuaError(
//     L: zluajit.State,
//     err: anyerror,
// ) noreturn {
//     switch (err) {
//         error.Runtime, error.Handler, error.InvalidSyntax, error.OpenRead => {
//             std.debug.print("{s} error\n", .{@errorName(err)});
//             if (L.toString(-1)) |errmsg| {
//                 std.debug.print("{s}\n", .{errmsg});
//             } else {
//                 std.debug.print("{s} error\n", .{@errorName(err)});
//                 L.dumpStack();
//             }
//         },
//         error.OutOfMemory => @panic("out of memory"),
//         else => {
//             std.debug.print("{s} error\n", .{@errorName(err)});
//         },
//     }
//
//     std.process.exit(1);
// }
