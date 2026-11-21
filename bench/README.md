# Prion micro-benchmarks / 微基准测试

Six small programs implemented in Prion, C++, Rust, Zig, Go, Java, C# and Swift.
Each language prints one result; the runner checks that all results match
before reporting times. See `docs/en/16-performance.md` (or
`docs/zh/16-性能.md`) for the measured report.

> These are micro-benchmarks. They measure a few small programs on one machine
> and do not represent production performance.
> 这些是微基准测试，只衡量少数小程序在单台机器上的表现，不代表实际生产体验。

## Benchmarks / 基准项

| Name | Workload |
|---|---|
| `num` | 20,000,000 rounds of `s = (s * 31 + i) % 1000000007` |
| `fib` | Naive recursive `fib(35)` |
| `list` | Build a 2,000,000-element array, then sum it |
| `index` | 3,000 passes over a 10,000-element array with the same recurrence |
| `map` | Insert 200,000 integer keys, then read them back |
| `concat` | 2,000,000 times: build a fresh 6-byte string from three segments |

`num` and `index` use a modulo recurrence so that a compiler cannot fold the
sum into a closed form or hoist the inner loop.

## Requirements / 运行要求

- `prionc` with `PRION_CC` pointing at `clang`, or `clang` on `PATH`
- Any subset of: `g++`, `rustc`, `zig`, `go`, `javac`/`java`, `dotnet`, `swiftc`
- Python 3.8+

Languages whose toolchain is missing are skipped. Paths can be overridden with
the environment variables `PRIONC`, `CXX`, `RUSTC`, `ZIG`, `GO`, `JAVAC`,
`JAVA`, `DOTNET`, `SWIFTC`.

## Run / 运行

```text
python bench/run.py
```

The runner builds every program, removes the build artifact first so compile
time is measured from scratch, then runs each benchmark `REP` times (default
10), reporting the median. Results are written to `bench/out/results.json`.

## Method / 方法

- Wall time of the whole process, median of `REP` runs; minimum and maximum are
  kept in the JSON.
- JVM/.NET/Swift/Go runs subtract a `none` baseline measured with the same
  process startup.
- Peak memory is read from a Windows job object (`PeakProcessMemoryUsed`); on
  Linux it is sampled from `/proc/<pid>/status` (`VmHWM`).
- The official report used: C++ `-O3`, Rust `-C opt-level=3`, Zig
  `ReleaseFast`, Prion `--mode release` (opt 3), Swift `-O`, C# `Release`,
  Java and Go defaults.
