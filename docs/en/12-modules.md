# 12 Modules and packages

## Overview

A single `.an` file is a complete program. A larger program is organized as a project directory of modules. Access across blocks and modules is always explicit.

## One file, one program

A file containing one or more blocks is a program. Every block that no other block references is a chain root and runs in source order.

## Cross-block access within a file

Blocks reach each other through three channels: passing arguments, return values, and taking a value with `call #:block`. To run another block, write `call #:block`.

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

Output:

```
cross fn 42
cross val 9
```

## Project directories

A multi-file project is a directory:

```
myproject/
    main.an        // entry point
    lib.prion      // optional metadata (e.g. name = ...)
    util.an        // additional module
```

`main.an` is always the entry point. When the current directory is the project, `prionc build` needs no name.

## Cross-module access

A module is referenced by its dotted path:

```anle
call ##:std.strings #:strings fn "split"(call val "text", " ")
```

`##:` names a module (its file path with `.` separators); `#:` names a block inside that module. The standard library lives under `std.`; set `PRION_STD` or place it at `<SDK>/lib/prion`.

## Visibility

- A block is callable across modules only when it is declared `pub`.
- `hide` removes an item from a `pub` block's public surface:

```anle
#: api b pub
    add fn "public_fn" (x int) int = x
    add hide fn "helper"
#: api d
```

`hide` applies to `fn`, `type` and `fix`. A `val` is already part of the public surface and cannot be hidden on its own. Field-level `hide` restricts cross-module access to individual fields.
