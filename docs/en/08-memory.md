# 08 Memory model

Prion reclaims memory deterministically: values live exactly as long as the block that owns them, and there is no tracing garbage collector. This chapter explains the ownership rules and the explicit tools you have.

## Block-scoped lifetime

Every value created in a block is owned by that block. When the block finishes, its values are released — in a defined order, at a defined point. Because blocks are flat and a block is both the unit of execution and the unit of memory, the lifetime of a value is always visible in the source.

A block that nothing references is a chain root; roots run top-to-bottom in source order. A referenced block runs where it is called, so its memory is live while the caller uses it.

## Copying with `take`

Reading a value with `call val` references the freshest value. When you want an independent copy — for example to hold data past the point where the source might change — use `take`, a deep copy:

```anle
add val "a" list(int) = [1, 2, 3]
add val "b" list(int) = take val "a"   // b is an independent copy
```

`take` copies `val`; it does not apply to `fn` (a function value holds no data).

## Early reclamation with `kill`

Normally you let blocks release their memory at block end. Inside an `unsafe` block you can reclaim another block's live regions early:

```anle
kill #:block
#: block d kill
```

The compiler tracks every live handle at the kill point. If any reference to the target is still live, it is a **compile error** listing each violation — so `kill` can never silently free memory that is still in use.

## unsafe blocks

Some capabilities are only available in a block marked `unsafe`:

```anle
#: raw b unsafe
    ...
#: raw d
```

Inside such a block you may use raw address read/write (`read_8`/`write_8` …), C interop (`str_to_c`/`str_from_c`), `asm`, and `kill`. In an ordinary block these words are **blocked by the mode**, which is a different error from "undefined name".

## Cross-block access

Values you need from another block are read explicitly:

```anle
call #:tools val "constant"
```

See [12](12-modules.md) for modules and [03](03-statements.md) for the `kill` statement.
