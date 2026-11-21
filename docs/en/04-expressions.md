# 04 Expressions

## Overview

Prion uses ordinary infix expressions with explicit verbs for reads (`call val`) and calls (`call fn`). This chapter defines the operator set, literal forms already introduced in [01](01-lexical.md), the expression forms for indexing, slicing and field access, and the `if` expression.

## Operators

| Category | Operators |
|---|---|
| Arithmetic | `+ - * / %` |
| Comparison | `< > <= >= == !=` |
| Logical (words) | `and or not` |
| Bitwise | `& \| ^ ! << >>` |
| Range (in `for`) | `~` |
| Error propagation | `?` (see [09](09-errors.md)) |

A unary minus must be followed by a numeric literal. Chained comparison such as `1 < x < 5` is not supported; write `1 < x and x < 5`. The ternary operator `?:` is not part of the language; use an `if` expression.

## Index, slice and field

These forms are part of a `call val` expression:

```prion
call val "list"[1]     // one element (indices start at 1)
call val "list"[1~2]   // inclusive slice
call val "point".x     // a field
```

## if expression

An `if` used in value position produces a value; both branches must have the same type:

```prion
add fn "max" (p int, q int) int = if p > q { p } else { q }
```

## Example

```prion
#: main b
    print("arith", 1 + 2 * 3, 7 % 2)
    print("compare", 5 > 3, 2 == 2)
    add val "b" bool = re
    print("logic", call val "b" and not fa, call val "b" or fa)
    print("bits", 6 & 3, 6 | 1, 1 << 4)
    print("radix", 0xff, 0b1010, 0o17)
    print("escape", "a\nb", "quote \"x\"")
    print("raw", @"a\b")
    add fn "larger" (p int, q int) int = if p > q { p } else { q }
    print("if expression", call fn "larger"(3, 2))
#: main d
```

Output:

```
arith 7 1
compare re re
logic re re
bits 2 7 16
radix 255 10 15
escape a
b quote "x"
raw a\b
if expression 3
```
