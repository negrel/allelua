const std = @import("std");
const allelua = @import("allelua");
const zluajit = @import("allelua").zluajit;
const Allelua = allelua.Allelua;

pub fn main() !void {
    var alloc = std.heap.c_allocator;

    var al = try Allelua.init(.{ .allocator = &alloc });
    defer al.deinit();

    // al.L.pushAnyType(struct {
    //     fn sleep(L: zluajit.State) !c_int {
    //         const secs = L.checkNumber(1);
    //         _ = secs;
    //
    //         // Async yield.
    //         L.getGlobal("coroutine");
    //         L.getField(-1, "_nursery");
    //
    //         // Create reference to nursery.
    //         L.pushValue(-1);
    //         const nursery = try L.ref(zluajit.Registry);
    //
    //         // Spawn thread.
    //         const thread_config = std.Thread.SpawnConfig{};
    //         _ = try std.Thread.spawn(thread_config, struct {
    //             fn thread(lua: zluajit.State, n: c_int) void {
    //                 std.time.sleep(1 * std.time.ns_per_s);
    //
    //                 // Retrieve nursery.
    //                 lua.pushAnyType(n);
    //                 lua.getTable(zluajit.Registry);
    //
    //                 // Remove it from registry.
    //                 lua.unref(zluajit.Registry, n);
    //
    //                 // Retrieve _wake and call it.
    //                 lua.getField(-1, "_wake");
    //                 lua.pushValue(-2);
    //                 _ = lua.pushState();
    //                 lua.call(2, 0);
    //             }
    //         }.thread, .{ L, nursery });
    //
    //         return L.yield(1);
    //     }
    // }.sleep);
    al.L.setGlobal("sleep");

    const args = try std.process.argsAlloc(alloc);
    defer std.process.argsFree(alloc, args);

    const fname: ?[:0]const u8 = if (args.len >= 2) args[1] else null;
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
