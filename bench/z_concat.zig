const std = @import("std");

pub fn main(init: std.process.Init.Minimal) !void {
    var it = try init.args.iterateAllocator(std.heap.page_allocator);
    defer it.deinit();
    _ = it.skip();
    const n: usize = if (it.next()) |s| try std.fmt.parseInt(usize, s, 10) else 200000;
    const a = std.heap.smp_allocator;
    var k: i64 = 0;
    var i: usize = 0;
    while (i < n) : (i += 1) {
        const s = try std.mem.concat(a, u8, &[_][]const u8{ "ab", "cd", "ef" });
        k += @intCast(s.len);
        a.free(s);
    }
    std.debug.print("{}\n", .{k});
}
