# 02 Basic concepts

## Overview

Four rules govern every Prion program: code lives in flat blocks; names are created once with `add`; reads and calls state their kind explicitly; and values are released when their owning block ends. This chapter defines these rules and demonstrates the scalar types.

## Blocks

A block opens with `#: name b` and closes with `#: name d`. Blocks are flat and never nest.

At run time blocks form chains. A block that no other block references is a chain root; roots execute top to bottom in source order. A referenced block executes where it is called. Two blocks that only reference each other, with no root, are rejected at compile time because they can never execute.

## Creation and re-binding

A name is created with `add`, exactly once:

```prion
add val "x" int = 1
add fn "f" (n int) int = n
add type "point" = (x int, y int)
```

Re-binding does not use `add`:

```prion
val "x" = 2
val "x" = val "x" (+1)
```

Assigning to a name that was never created is a compile error. There is no rule that a first assignment creates a variable, so a misspelled name is reported rather than silently becoming a new variable.

## Explicit reads and calls

A read is written `call val "x"`; a call is written `call fn "f"(1)`. A bare identifier is rejected, because a function and a value may share a name and a bare identifier cannot say which is intended.

Two verbs form the read surface:

- `call` references the freshest value.
- `take` deep-copies the value as it is at that moment.

Memory is released when the owning block ends; there is no statement that releases memory early.

## Scalar types

```prion
int  float  bool  str
int_8 int_16 int_32 int_64  uint_8 uint_16 uint_32 uint_64
```

## Example

```prion
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

Output:

```
int 42
float 1.5
bool re
str abc
uint8 200
null check ok
```

## Rules

- A block is both the unit of execution and the unit of memory; nesting is therefore disallowed.
- `add` appears once per name; repeated creation is a compile error.
- Reads and calls state their verb; there are no bare identifiers.
- Values are released at block end; the lifetime of a value never outlives its block.
