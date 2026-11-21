# 03 Statements

## Overview

Control-flow bodies open with `:` and close with a lone `;` on its own line. Indentation has no effect on meaning. This chapter defines conditionals, loops, `match`, and the assignment forms.

## Body termination

`if`, `else`, `for`, `while` and each `match` arm are terminated by a lone `;`. The `;` is the structural boundary, independent of indentation; a missing `;` is a parse error.

## Conditions and loops

- A condition must be boolean; `if 5:` is rejected.
- `for i in a~b:` iterates an inclusive integer range.
- `for x in container:` iterates a list, map or string.
- `while cond:` iterates without a bound; `while cond max N:` caps the iteration count as a stall guard.

Loop variables are ordinary `val` names. Names may be written with or without quotes.

## match

`match` destructures a sum-type value. `case Variant("binding"):` binds a payload; `case _:` is the fallback; a guard `case Variant("v") if cond:` enters the arm only when the condition holds and otherwise tries the next case. The match subject must have a type that is known at compile time, and coverage is checked for exhaustiveness.

## Assignment forms

```anle
val "x" = 5              // re-bind
val "x" = val "x" (+1)   // delta: read the current value, apply the expression, store
val "list"[1] = 7        // index write
val "point".x = 3        // field write
```

The delta form uses the current value as the left operand and requires the name to already have a value.

## Reclamation

Memory is released when the owning block ends. There is no statement that releases memory early; see [08](08-memory.md).

## Example

```anle
#: main b
    add val "sum" int = 0
    for "i" in 1~3:
        val "sum" = val "sum" (+call val "i")
    ;
    print("for range", call val "sum")

    add val "names" list(str) = ["a", "b"]
    for "n" in call val "names":
        print("each", call val "n")
    ;

    add val "count" int = 0
    while call val "count" < 3 max 10:
        val "count" = val "count" (+1)
    ;
    print("while", call val "count")

    add val "v" int = 5
    if call val "v" > 3:
        print("big")
    ;
    else:
        print("small")
    ;

    add type "res" = "ok"(int) | "err"(str)
    add val "r" "res" = "ok"(10)
    match call val "r":
        case "ok"("x") if call val "x" > 0:
            print("ok", call val "x")
        ;
        case _:
            print("other")
        ;
    ;
#: main d
```

Output:

```
for range 6
each a
each b
while 3
big
ok 10
```
