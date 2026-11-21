# 08 Memory model

## Overview

Prion reclaims memory deterministically. Values are owned by the block that creates them and are released when that block finishes; there is no tracing garbage collector and no reference counting.

## Lifetime and release

A block is both the unit of execution and the unit of memory. When a block finishes, its values are released in a defined order at a defined point. Because blocks are flat, the lifetime of every value is visible in the source.

Blocks form chains at run time. A block that no other block references is a chain root; roots run top to bottom in source order. A referenced block runs where it is called, so its memory stays live while the caller uses it.

There is no statement that releases memory early. To shorten a value's lifetime, move its work into a smaller block that finishes sooner.

## Copying with take

`call val` references the freshest value. To obtain an independent copy, use `take`, which deep-copies the value as it is at that moment:

```anle
add val "a" list(int) = [1, 2, 3]
add val "b" list(int) = take val "a"
```

`take` applies to `val`; a function value holds no data and is not copied.

## unsafe blocks

Some capabilities are available only inside a block marked `unsafe`:

```anle
#: raw b unsafe
    ...
#: raw d
```

Inside such a block the raw address operations (`read_8` / `write_8` …), C interoperability (`str_to_c` / `str_from_c`) and `asm` are permitted. In an ordinary block these words are **blocked by the mode**; this is distinct from an undefined name.

## Cross-block access

A value in another block is read explicitly:

```anle
call #:tools val "constant"
```

Modules are described in [12](12-modules.md); statements in [03](03-statements.md).
