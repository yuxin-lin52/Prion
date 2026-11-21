# 16 · Performance

> **Disclaimer.** These are micro-benchmarks: six small programs on one
> machine. They do not represent production performance. The language is under
> active development and may still contain unknown bugs — please report any you
> find.

Everything here is reproducible: the benchmark sources and the runner are in
[`bench/`](../bench/README.md).

## Environment

| Item | Value |
|---|---|
| Machine | AMD Ryzen 9 8945HX with Radeon Graphics |
| OS | Windows |
| Prion | 0.3.0, release mode (`--opt 3`) |
| C++ | GCC 16.2.0, `-O3` |
| Rust | 1.98.1, `-C opt-level=3` |
| Zig | 0.17.0, `ReleaseFast` |
| Go | 1.27.2, default flags |
| Java | OpenJDK 17.0.19, default flags (JIT time included) |
| C# | .NET 10.0.401, `Release` |
| Swift | 6.4.2, `-O` |

## Method

- Wall time of the whole process, **median of 10 runs**; minimum and maximum
  are in the JSON output.
- Startup baselines for JVM, .NET, Go and Swift are measured with an empty
  `none` run and subtracted.
- Every language must print the same result for a benchmark to count.
- `num` and `index` use `s = (s * 31 + x) % 1000000007` so that the loop cannot
  be folded into a closed form or hoisted.
- `concat` builds a fresh 6-byte string from three segments on every iteration.

## Runtime (ms, median)

| Benchmark | C++ | Rust | Zig | Swift | Java | C# | Go | Prion |
|---|---|---|---|---|---|---|---|---|
| num | 63.4 | 63.2 | 74.5 | 54.7 | 54.5 | 54.8 | 58.3 | 68.4 |
| fib | 20.0 | 25.4 | 25.5 | 32.9 | 16.4 | 39.9 | 31.4 | 25.1 |
| list | 14.4 | 14.1 | 14.1 | 5.5 | 6.6 | 3.6 | 5.4 | 14.2 |
| index | 87.9 | 90.3 | 107.0 | 86.5 | 81.3 | 81.5 | 80.7 | 94.7 |
| map | 20.1 | 14.0 | 14.5 | 11.2 | 27.9 | 9.3 | 11.0 | 14.4 |
| concat | 14.2 | 53.0 | 14.4 | 38.2 | 38.4 | 23.3 | 36.6 | 63.6 |

## Peak memory (MB)

| Benchmark | C++ | Rust | Zig | Swift | Java | C# | Go | Prion |
|---|---|---|---|---|---|---|---|---|
| num | 0.8 | 0.8 | 0.6 | 2.1 | 363.1 | 6.5 | 12.3 | 0.7 |
| fib | 0.8 | 0.8 | 0.6 | 2.1 | 363.1 | 6.6 | 12.3 | 0.7 |
| list | 16.1 | 16.1 | 23.6 | 17.3 | 363.1 | 22.0 | 29.9 | 15.9 |
| index | 0.8 | 0.9 | 0.7 | 2.1 | 363.2 | 7.0 | 12.4 | 0.8 |
| map | 10.6 | 5.0 | 4.9 | 10.1 | 364.9 | 19.2 | 19.0 | 7.7 |
| concat | 0.8 | 0.8 | 0.7 | 2.1 | 364.1 | 23.0 | 19.7 | 0.7 |

## Compile time and artifact size

| Language | Compile time | Artifact |
|---|---|---|
| C++ | 0.51 s | 500 KB |
| Rust | 0.39 s | 140 KB |
| Zig | 34.02 s | 5134 KB |
| Swift | 0.75 s | 30 KB |
| Java | 0.33 s | 2 KB |
| C# | 0.80 s | 6 KB |
| Go | 0.30 s | 1624 KB |
| Prion | 2.03 s | 1200 KB |

Compile time is one full build with a warm toolchain cache after deleting the
artifact. C++, Rust, Java, C#, Go and Swift compile a single source that
contains all six modes, while Prion and Zig build six programs — the Prion and
Zig figures are totals over six builds.

## Reading the numbers

- **Arrays and hash maps**: Prion is on par with C++/Rust/Zig for array
  traversal, and with Rust/Zig for hash maps.
- **Scalar work and recursion**: mid-field — about 8% behind C++/Rust on
  `num`, about 25% behind C++ and Java on `fib`.
- **String construction**: the weakest area. Prion allocates once per
  `concat`, the same model as Rust (53.0 ms); C++ and Zig avoid the allocation
  per call through small-string optimization and a faster allocator. Prion has
  no small-string optimization and no pooled allocator by design.
- **Memory**: peak usage stays in the same range as the native languages and
  far below the JVM.
