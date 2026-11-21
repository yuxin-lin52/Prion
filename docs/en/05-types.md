# 05 Types

Prion has scalar types, container types, product (struct) types, sum (variant) types, and handle types written with `*`. Types are never converted silently.

## Scalars

```anle
int  float  bool  str
int_8 int_16 int_32 int_64
uint_8 uint_16 uint_32 uint_64
```

`re` and `fa` are boolean *values*, not types — the type is `bool`.

## Containers

```anle
list(T)  map(K, V)  que(T)  stack(T)
```

`map` keys may only be `str` or `int`. A container cannot directly hold another container; use a `map(str, list(int))` or a struct field to group.

## Product and sum types

```anle
add type "point" = (x int, y int)              // product: named fields
add type "expr" = "num"(int) | "add"(int)      // sum: variants
```

Construct a product with `"point"(3, 4)` and a variant with `"num"(7)`. Read a field with `call val "p".x`; destructure a sum with `match`.

## Handle types

Append `*` to a user-defined type to make it a **handle** (reference): `point*`. Builtin scalar types cannot take `*` — they are already values. A handle can be `null`; check with `== null` before reading a field.

A type whose value field leads back to itself is a cycle and is rejected. Break the cycle by making one end a handle: `node*` instead of `node`.

## Try it

```anle
#: main b
    add type "point" = (x int, y int)
    add val "p" "point" = "point"(3, 4)
    print("fields", call val "p".x, call val "p".y)

    add type "expr" = "num"(int) | "add"(int)
    add val "t" "expr" = "num"(7)
    match call val "t":
        case "num"("v"):
            print("num", call val "v")
        ;
        case "add"("v"):
            print("add", call val "v")
        ;
    ;

    add val "q" point* = point(9, 9)
    print("handle", call val "q".x)
#: main d
```

Expected output:

```
fields 3 4
num 7
handle 9
```

## Why no implicit conversion

`int` never becomes `str` on its own. When you need a conversion, write it: `int(v)`, `str(v)`, `float(v)`, `bool(v)`. Implicit conversion is where type bugs hide, so Prion makes it visible.

Type names are checked for existence: a retired or misspelled type name is an error, not a silent new type.
