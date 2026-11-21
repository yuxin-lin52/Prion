# 13 Portability and targets

## Overview

Prion compiles ahead of time through LLVM to native code. The produced executable has no virtual machine and no runtime dependency.

## Build modes

```
prionc build [name] --mode debug|release --opt 0-3
```

- `debug` — full guards (bounds and null checks); default `--opt 0`.
- `release` — optimized with all guards stripped; default `--opt 3`. Out-of-bounds and null accesses become undefined behavior, so programs should be tested in `debug` first.

The mode selects the output directory: `build/<mode>/<name>.exe`.

## Commands

| Command | Effect |
|---|---|
| `prionc build [name]` | compile and link |
| `prionc run [name]` | build, then run; `--` passes arguments to the program |
| `prionc check [name]` | run the front end only, no files |
| `prionc fmt [path] [--check]` | add decorative indentation |
| `prionc clean [name]` | remove the `build/` directory |
| `prionc -V` / `prionc -h` | version / usage |

## Targets

The LLVM backend targets `x86` and `aarch64`. Code generation uses LLVM 22 and links with `clang`.

## Environment

- `PRION_CC` — the `clang` used for linking, and the fallback toolchain.
- `PRION_STD` — the standard library root, when not found by the SDK layout.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 101 | a Prion error (diagnostic printed) |
| 1 | usage error |

`run` forwards the program's own exit code (0–255).

## Output layout

```
build/debug/hello.exe      // executable
build/debug/hello.ll       // LLVM IR
build/debug/hello.o        // object file
```

## Toolchain resolution

The compiler does not silently substitute an incompatible toolchain. If `PRION_CC` points at a compiler that cannot link, the failure is reported; the toolchain is expected to be corrected rather than worked around.
