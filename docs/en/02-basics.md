# 02 Basic concepts

This chapter teaches the four ideas you need before writing any real Prion code: blocks, create-once names, full explicitness, and the verb slots. By the end you will have run a small program that uses each of the scalar types.

## Blocks

All code lives in blocks. A block opens with `#: name b` and closes with `#: name d`. Blocks are **flat** — they never nest.

At run time, blocks chain: every block that nothing references is a **chain root**, and roots run top-to-bottom in source order. A referenced block runs where it is called. Two blocks that only call each other with no root are rejected at compile time (they could never run).

## Create once, then re-bind

Names are created with `add`, exactly once:

```anle
add val "x" int = 1          // create a value
add fn "f" (n int) int = n   // create a function
add type "point" = (x int, y int)  // create a type
```

Re-binding never uses `add` — it is a plain assignment:

```anle
val "x" = 2
val "x" = val "x" (+1)
```

Assigning to a name that was never created is a compile error. There is no "first assignment creates the variable" rule: a typo becomes a loud error instead of a new invisible variable.

## Full explicitness

Reads spell out the verb: `call val "x"`. Calls spell out the verb: `call fn "f"(1)`. A bare identifier such as `x` is rejected — a function and a value may share a name, so a bare identifier cannot say which one is meant.

## The verb slots

- `call` — reference the freshest value.
- `take` — deep-copy the value as it is right now.
- `kill` — explicit early reclamation (inside `unsafe` blocks; see [08](08-memory.md)).

## Try it

```anle
#: main b
    add val "i" int = 42
    add val "r" float = 1.5
    add val "b" bool = re
    add val "s" str = "abc"
    add val "u8" uint_8 = 200
    add val "nil" str = null
    print("int", call val "i")
    print("float", call val "r")
    print("bool", call val "b")
    print("str", call val "s")
    print("uint8", call val "u8")
    if call val "nil" == null:
        print("null check", "ok")
    ;
#: main d
```

Expected output:

```
int 42
float 1.5
bool re
str abc
uint8 200
null check ok
```

## Why these rules

- **Flat blocks**: a block is the unit of execution and the unit of memory; nesting would blur both boundaries.
- **`add` once**: repeatedly re-`add`ing turns "change the value" into "recreate it", and first-assignment-creates hides typos as new variables.
- **Full explicitness**: a few extra characters per read buy you the guarantee that no read depends on context.
