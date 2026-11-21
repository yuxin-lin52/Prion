const std = @import("std");

fn fib(n: i64) i64 {
    return if (n < 2) n else fib(n - 1) + fib(n - 2);
}

pub fn main(init: std.process.Init.Minimal) !void {
    var it = try init.args.iterateAllocator(std.heap.page_allocator);
    defer it.deinit();
    _ = it.skip();
    const n: i64 = if (it.next()) |s| try std.fmt.parseInt(i64, s, 10) else 35;
    std.debug.print("{}\n", .{fib(n)});
}
