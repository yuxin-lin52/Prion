# 04 Expressions

Status: implemented.

## Operators

| Family | Operators | Notes |
|---|---|---|
| Arithmetic | `+ - * / %` | `int` and `float` do not mix without an explicit cast; safe-level overflow is an error |
| Comparison | `== != < > <= >=` | value comparison; `==` descends into strings, lists, maps, and struct fields |
| Logic | `and` `or` `not` | keywords; `and`/`or` short-circuit |
| Bitwise | `& \| ^ << >> !` | integers only; `!` is bitwise NOT (`not` is logical NOT); `>>` is arithmetic shift |
| Conditional | `if cond { a } else { b }` | expression form; `else` is required and both branches must share a type; `?` is reserved for error propagation (see [09](09-errors.md)) |

Precedence: comparison `< | < ^ < & < shift < add < multiply`.

## Calls and reads

- `call val "name"` reads a value; `call fn "f"(x)` calls a function.
- Field access: `position.x`; chains auto-dereference handles with per-level null checks (`head.next.value`).
- Indexing: `list[1]`, `map["key"]` — 1-based, matching the inclusive range operator.
- Slices: `list[1~3]`, `s[2~4]` — 1-based, both ends inclusive, reusing the range operator.

## Conversions

Type names as functions: `int("5")`, `str(number)`, `float(text)`, `uint8(300)`. There are no implicit conversions — `int` and `float` do not mix without one, and where a number came from stays written down.

## Copying

- `take val "name"` copies a value in the same block; `take #:block val "name"` and `take ##:module #:block val "name"` copy across blocks and modules.
- The copy is a snapshot: lists, maps, and strings come back as independent copies; scalars copy by value.
- The syntax mirrors `call`; the only difference is the verb — take a copy instead of reading a reference slot. `(+0)` no longer means copy.

## Why

- `if` expressions: branches are values; the recursive exit lives in the expression itself, and a required `else` rules out missing branches.
- No chained comparisons: parsing complexity outweighs the use case; write `and`.
- Explicit conversions: implicit promotion turns a type mismatch into a runtime surprise.
- `!` for bitwise NOT and `not` for logical NOT: `~` is already the range operator, `!` was free, and both are familiar.
- One `take` for copying: local and cross-block use the same verb, so readers no longer guess whether `(+0)` adds or copies.
