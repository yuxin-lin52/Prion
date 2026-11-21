const std = @import("std");

pub fn main(init: std.process.Init.Minimal) !void {
    var it = try init.args.iterateAllocator(std.heap.page_allocator);
    defer it.deinit();
    _ = it.skip();
    const n: i64 = if (it.next()) |s| try std.fmt.parseInt(i64, s, 10) else 20000000;
    var sum: i64 = 0;
    var i: i64 = 1;
    while (i <= n) : (i += 1) sum = @mod(sum * 31 + i, 1000000007);
    std.debug.print("{}\n", .{sum});
}
