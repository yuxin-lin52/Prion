# 13 Portability and targets

Prion compiles ahead of time through LLVM to a native executable. There is no VM and no runtime dependency in the produced binary.

## Build modes

```
prionc build [name] --mode debug|release --opt 0-3
```

- `debug` — full guards (bounds checks, null checks). Default `--opt 0`.
- `release` — optimized, all guards stripped. Default `--opt 3`. Out-of-bounds and null dereferences become undefined behavior, so test under `debug` first.

The mode selects the output directory: `build/<mode>/<name>.exe`.

## Running

```
prionc run hello
```

`run` builds, then executes the result. Arguments after `--` are passed through to the program.

## Check only

```
prionc check hello
```

`check` runs the front end (lex → parse → check) and produces no files. It is the fastest way to verify code.

## Targets

The LLVM backend targets `x86` and `aarch64`. Code generation uses the LLVM 22 toolchain and links with `clang`.

## Environment

- `PRION_CC` — the `clang` used for linking (and as a fallback toolchain).
- `PRION_STD` — the standard library root, if not found by the SDK layout.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 101 | a Prion error (diagnostic printed) |
| 1 | usage error |

`run` forwards the program's own exit code (0–255).

## Output layout

```
build/debug/hello.exe      // the executable
build/debug/hello.ll       // LLVM IR (debug aid)
build/debug/hello.o        // object file
```

`prionc clean` removes the whole `build/` directory.

## No silent conversion of the environment

The compiler does not auto-detect a "compatible" toolchain and silently fall back. If `PRION_CC` points at a compiler that cannot link, you get an explicit error — fix the toolchain, do not work around it.
