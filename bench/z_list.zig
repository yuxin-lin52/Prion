const std = @import("std");

pub fn main(init: std.process.Init.Minimal) !void {
    var it = try init.args.iterateAllocator(std.heap.page_allocator);
    defer it.deinit();
    _ = it.skip();
    const n: usize = if (it.next()) |s| try std.fmt.parseInt(usize, s, 10) else 2000000;
    const a = std.heap.smp_allocator;
    var v: std.ArrayList(i64) = .empty;
    defer v.deinit(a);
    try v.ensureTotalCapacity(a, n);
    var i: i64 = 0;
    while (i < n) : (i += 1) v.appendAssumeCapacity(i);
    var sum: i64 = 0;
    for (v.items) |x| sum += x;
    std.debug.print("{}\n", .{sum});
}
