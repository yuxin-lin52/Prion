const std = @import("std");

pub fn main(init: std.process.Init.Minimal) !void {
    var it = try init.args.iterateAllocator(std.heap.page_allocator);
    defer it.deinit();
    _ = it.skip();
    const n: usize = if (it.next()) |s| try std.fmt.parseInt(usize, s, 10) else 200000;
    const a = std.heap.smp_allocator;
    var m: std.AutoHashMapUnmanaged(i64, i64) = .empty;
    defer m.deinit(a);
    try m.ensureTotalCapacity(a, @intCast(n));
    var i: i64 = 0;
    while (i < n) : (i += 1) try m.put(a, i, i * 2);
    var sum: i64 = 0;
    i = 0;
    while (i < n) : (i += 1) sum += m.get(i).?;
    std.debug.print("{}\n", .{sum});
}
