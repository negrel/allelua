const std = @import("std");
const allelua = @import("allelua");
const zluajit = @import("allelua").zluajit;
const Allelua = allelua.Allelua;

pub fn main() !void {
    var alloc = std.heap.c_allocator;

    var al = try Allelua.init(.{ .allocator = &alloc });
    defer al.deinit();

    const args = try std.process.argsAlloc(alloc);
    defer std.process.argsFree(alloc, args);

    const fname: ?[]const u8 = if (args.len >= 2) args[1] else null;
    al.doFile(fname orelse null) catch |err| handleLuaError(al, err);
}

fn handleLuaError(
    al: *Allelua,
    err: anyerror,
) void {
    switch (err) {
        error.Runtime, error.Handler, error.InvalidSyntax, error.OpenRead => {
            std.debug.print("{s} error\n", .{@errorName(err)});
            if (al.L.toString(-1)) |errmsg| {
                std.debug.print("{s}\n", .{errmsg});
            } else {
                al.L.dumpStack();
            }
            std.process.exit(1);
        },
        error.OutOfMemory => @panic("out of memory"),
        else => {
            std.debug.print("{s} error\n", .{@errorName(err)});
            std.process.exit(1);
        },
    }
}
