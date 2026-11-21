<h1 align="center">Prion</h1>

<p align="center"><a href="https://github.com/yuxin-lin52/Prion/actions/workflows/build.yml"><img src="https://github.com/yuxin-lin52/Prion/actions/workflows/build.yml/badge.svg" alt="build"></a></p>

<p align="center"><a href="README.zh-CN.md">中文</a> | English</p>

Prion is an ahead-of-time compiled language. Memory is managed by the compiler: reclamation points are computed from the program's block structure, and the runtime has no garbage collector, no pauses, and no per-object counting.

## Design motivation

Prion is built on a single division of labor: **simple to write, complex to implement.**

Every language must answer one question: who is responsible for memory? Manual management hands correctness to the programmer. Runtime management removes that work, but adds an extra burden and runtime pauses. Compiler-checked ownership guarantees correctness, at the price of a set of rules the programmer must learn and satisfy.

Prion takes none of these trade-offs. The program is structured as blocks; values live in the block that declares them; all data crossing a block boundary goes through an explicit channel. Given that structure, the compiler computes the lifetime and reclamation point of every value. The programmer writes blocks and states data flow; the compiler does the rest.

The same division runs through the whole language. The syntax is small and regular. Direct memory access, C interoperability and inline assembly exist only inside `unsafe` blocks. There is no manual memory allocation. The compiler computes where memory is released from the block structure: each block runs in its own region, and when the block's chain finishes, its segments are handed back to the platform allocator in one step.

## Language overview

```anle
#: main b
    add val "total" int = 0
    add val "i" int = 1
    while call val "i" <= 100:
        val "total" = call val "total" + call val "i"
        val "i" = call val "i" + 1
    ;
    print(call val "total")
#: main d
```

This prints `5050`. The rules visible in this example:

* All code lives in blocks. A block opens with `#: name b` (born) and closes with `#: name d` (death). Blocks are flat — they do not nest.
* Names are created with `add` and re-bound with `val "name" =` (`add` is never repeated).
* Values are read as `call val "name"`. There are no bare identifiers: every read states its kind, and every cross-block reference states its source.
* No implicit conversions. `int` and `float` do not mix without an explicit cast.
* Conditions must be booleans — `if 5:` is a compile error.
* Indices start at 1, matching the inclusive range operator (`for i in 1\~5` covers 1 through 5).
* Control-flow bodies are terminated by a lone `;` on its own line. Indentation is purely cosmetic and can never change meaning; a mismatched `;` is a loud parse error. The Prion editor extension ships an auto-formatter that keeps indentation consistent.

## Memory model

* Memory is allocated per block instance. When a block finishes, all of its values are released together rather than object by object.
* Reclamation points are fixed at compile time from the block reference graph. At run time there is no GC thread, no stop-the-world pause, and no reference counting.
* Data crossing a block boundary takes one of two explicit forms: a **reference slot** (`call #:block val "name"` — reads see the freshest value, writes go through) or a **deep copy** (`take #:block val "name"` — the value at that moment belongs to the caller).
* Reference cycles do not leak, because reclamation follows block scope and does not count references.

The model gives every programmer the same simple default: put the code in blocks, and memory takes care of itself — a beginner can write an entire program in one block with zero memory burden. The block layout never affects correctness, only memory footprint. For memory- or performance-sensitive projects, advanced developers can tune it: splitting the program into more blocks releases each block's memory earlier and keeps the peak lower.

## Download

A prebuilt Windows x64 compiler is published on the [releases page](https://github.com/yuxin-lin52/Prion/releases). It uses the LLVM backend, so compiling a program additionally needs a linker toolchain: `clang` on `PATH`, or `PRION_CC` pointing at one. LLVM itself is linked into the compiler and does not need to be installed separately.

## Building

The compiler source is in [`compiler/`](compiler) — a Rust workspace. LLVM is the only backend.

```
cd compiler
cargo build --release -p prionc
```

The LLVM backend links LLVM 22.1 statically: set `LLVM_SYS_221_PREFIX` to an LLVM 22.1 installation, and add that installation's `deps` directory to the link search path with `RUSTFLAGS="-L native=<prefix>\deps"` — it holds the zlib, zstd and libxml2 import libraries. LLVM is not redistributed here; get it from the upstream release page: <https://github.com/llvm/llvm-project/releases>.

```
./target/release/prionc build hello
```

## Implementation status

* The compiler is implemented in Rust; the source is included in this repository. Self-hosting is in an early stage: the lexer of the self-hosted front end is complete and passes token-for-token comparison against the host implementation.
* A single backend is implemented: **LLVM**, which compiles through `clang` for linking.
* The syntax is **not frozen**. Breaking changes are expected; anything written against this language will need updating.
* The standard library consists of about 40 builtin functions.

## Diagnostics

Diagnostics carry four parts: what happened, the source line, what the compiler expected, and how to fix it. They are emitted in English. A real example:

```
[E202] error at example.an line 2, column 5, in assignment: "v" is declared as float, the assigned value is int; source: add val "v" float = 0 - 5. Expected: the assigned value must be of type float (the declaration decides). Fix: change it to a float value, or change the declaration to int. Details: Prion does not convert types silently: int does not become str automatically. To change the type, convert explicitly with int(value) / str(value) / float(value) / bool(value); if the declaration is wrong, change the declared type to match the value.
```

## Performance

Micro-benchmarks against C++, Rust, Zig, Go, Java, C# and Swift are in [16 · Performance](docs/en/16-performance.md); the sources and the runner are in [`bench/`](bench/). These are micro-benchmarks — they do not represent production performance.

## Feedback

The repository is public to collect design feedback. Issues are open — design questions, rough edges in the examples, and disagreements with the choices above are all welcome. The [documentation](docs/README.md#english) is a hands-on tutorial for the language.

## License

Apache 2.0. See [LICENSE](LICENSE).

