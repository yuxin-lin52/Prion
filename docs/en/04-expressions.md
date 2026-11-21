# 04 Expressions

Prion expressions are ordinary infix expressions, but two things are always explicit: reads of a value use `call val "name"`, and calls use `call fn "name"(…)`. This chapter runs through the operators and literals.

## Try it

```anle
#: main b
    print("arith", 1 + 2 * 3, 7 % 2)
    print("compare", 5 > 3, 2 == 2)
    add val "b" bool = re
    print("logic", call val "b" and not fa, call val "b" or fa)
    print("bits", 6 & 3, 6 | 1, 1 << 4)
    print("radix", 0xff, 0b1010, 0o17)
    print("escape", "a\nb", "quote \"x\"")
    print("raw", @"a\b")
#: main d
```

Expected output:

```
arith 7 1
compare re re
logic re re
bits 2 7 16
radix 255 10 15
escape a
b quote "x"
raw a\b
```

## Operators

- Arithmetic: `+ - * / %`
- Comparison: `< > <= >= == !=`
- Logic: `and or not` (words, not `&& || !`)
- Bitwise: `& | ^ ! << >>`
- Range (in `for`): `~`

A unary minus must be followed by a numeric literal, e.g. `-1` or `-2.5`.

## Literals

- Integers: decimal, `0x` hex, `0b` binary, `0o` octal; `_` separates digits.
- Floats: `1.5`, `1e6`, `1.5e-3`.
- Strings: `"…"` with `\n \t \\ \"`; verbatim `@"…"`.
- Booleans: `re` / `fa`. Null: `null`.

## Indexing, slicing and fields

These appear as part of a `call val` expression:

```anle
call val "list"[1]     // one element
call val "list"[1~2]   // an inclusive slice
call val "point".x     // a field
```

A comparison chain such as `1 < x < 5` is **not** supported; write `1 < x and x < 5`.
