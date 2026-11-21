# 05 Types

## Overview

Prion distinguishes scalar types, container types, product types, sum types, nullable types, refined types, type aliases and handle types. Types are never converted implicitly.

## Scalars

```prion
int  float  bool  str
int_8 int_16 int_32 int_64  uint_8 uint_16 uint_32 uint_64
```

`re` and `fa` are boolean values; the type is `bool`.

## Containers

```prion
list(T)  map(K, V)  que(T)  stack(T)
```

A `map` key may only be `str` or `int`. A container cannot directly hold another container; group nested collections in a `map(str, list(int))` or in a struct field.

## Product and sum types

```prion
add type "point" = (x int, y int)              // product: named fields
add type "expr" = "num"(int) | "add"(int)      // sum: variants
```

Construct a product with `"point"(3, 4)` and a variant with `"num"(7)`. Read a field with `call val "p".x`; destructure a sum type with `match`.

A product with a single unnamed field is a newtype: `add type "UserId" = (int)`. Construct it with `"UserId"(42)` and read its value with `.1`.

## Aliases and refined types

```prion
add type "Len" = int        // alias: Len and int are the same type
add val "p" int(>0) = 5     // refined: the condition is checked at compile time
```

A value that violates a refinement condition (for example `int(>0) = -5`) is rejected at compile time.

## Nullable types and handles

A nullable type is written `?T`:

```prion
add val "found" ?int = null
```

Append `*` to a user-defined type to make it a **handle** (reference): `point*`. Builtin scalar types cannot take `*` because they are already values. A type whose value field leads back to itself is a cycle and is rejected; make one end a handle to break it.

## Example

```prion
#: main b
    add type "Len" = int
    add val "n" "Len" = 3
    add val "p" int(>0) = 5
    add val "z" ?int = null
    add type "UserId" = (int)
    add val "uid" "UserId" = "UserId"(42)
    print("alias", call val "n", "refined", call val "p", "nullable", call val "z" == null, "newtype", call val "uid".1)

    add type "point" = (x int, y int)
    add val "pt" "point" = "point"(3, 4)
    print("fields", call val "pt".x, call val "pt".y)

    add type "expr" = "num"(int) | "add"(int)
    add val "e" "expr" = "num"(7)
    match call val "e":
        case "num"("v"):
            print("num", call val "v")
        ;
        case "add"("v"):
            print("add", call val "v")
        ;
    ;

    add val "h" point* = point(9, 9)
    print("handle", call val "h".x)
#: main d
```

Output:

```
alias 3 refined 5 nullable re newtype 42
fields 3 4
num 7
handle 9
```

## Notes

- Type names are checked for existence; a misspelled or retired name is an error, not a new type.
- Conversions are explicit: `int(v)`, `str(v)`, `float(v)`, `bool(v)`.
