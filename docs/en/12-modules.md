# 12 Modules and packages

A single `.an` file is a complete program. Larger programs are split into a directory of modules.

## One file, one program

If the file has one or more blocks, it is a program. Every block that nothing references is a chain root and runs in source order.

## Cross-block access (same file)

Blocks reach each other through three explicit channels: passing arguments, return values, and taking a value with `call #:block`. To run another block, write `call #:block`.

```anle
#: tools b
    add fn "double" (x int) int = x * 2
    add val "constant" int = 9
#: tools d

#: main b
    print("cross fn", call #:tools fn "double"(21))
    call #:tools
    print("cross val", call #:tools val "constant")
#: main d
```

Expected output:

```
cross fn 42
cross val 9
```

## A directory project

For more than one file, use a project directory:

```
myproject/
    main.an        // entry point
    lib.prion      // optional metadata (e.g. name = ...)
    util.an        // additional module
```

`main.an` is always the entry. Build it with `prionc build` (no name needed if the current directory is the project).

## Cross-module access

A module is referenced by its dotted path:

```anle
call ##:std.strings #:strings fn "split"(call val "text", " ")
```

- `##:` names a module (its file path with `.` separators).
- `#:` names a block inside that module.
- The standard library lives under `std.`; set `PRION_STD` or place it at `<SDK>/lib/prion`.

## Visibility

- A block is only callable across modules if it is declared `pub`.
- `hide` removes an item from a `pub` block's public surface so it stays internal:

```anle
#: api b pub
    add fn "public_fn" (x int) int = x
    add hide fn "helper"
#: api d
```

`hide` applies to `fn`, `type` and `fix`. A `val` is already part of the public surface and cannot be hidden on its own.
