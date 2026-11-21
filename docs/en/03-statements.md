# 03 Statements

In this chapter you write loops, conditionals, `match`, assignments and reclamation. Every construct is closed with a lone `;` on its own line; indentation is cosmetic.

## Closing bodies with `;`

`if`, `else`, `for`, `while` and `match` arms all end with a `;` on its own line. The `;` is the structural close — it never depends on indentation, so a missing one is a loud parse error rather than a silent misread.

## Conditions and loops

- A condition must be a boolean. `if 5:` is rejected.
- `for i in 1~5:` walks the inclusive integer range; `for x in collection:` iterates lists, maps and strings.
- `while cond:` loops freely; `while cond max 100:` caps iterations as a stall guard (infinite loops are legal, the cap just stops a hang).

## Try it: control flow

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
#: main d
```

Expected output:

```
for range 6
each a
each b
while 3
big
```

## match

A `match` destructures a sum-type value. `case Variant("binding"):` binds a payload, `case _:` is the fallback, and guards use `if`:

```anle
#: main b
    add type "res" = "ok"(int) | "err"(str)
    add val "r" "res" = "ok"(10)
    match call val "r":
        case "ok"("v"):
            print("ok", call val "v")
        ;
        case "err"("why"):
            print("err", call val "why")
        ;
    ;
#: main d
```

Expected output:

```
ok 10
```

The match subject must be an expression whose type is known at compile time. Exhaustiveness is checked: adding a variant to a type exposes every `match` that no longer covers it.

## Re-binding, delta, index and field writes

```anle
val "x" = 5            // re-bind
val "x" = val "x" (+1) // delta: read current value, add 1, store
val "list"[1] = 7      // index write
```

The delta form `(+1)` takes the current value as the left operand. A write position (`list[1] = …`, `head.next = …`) is written explicitly, e.g. `val "list"[1] = 7`.

## kill

Inside an `unsafe` block, `kill #:block` reclaims another block's live regions early, and `#: block d kill` cleans the block on exit. The compiler tracks every live handle at the kill point; a live reference there is a compile error. See [08](08-memory.md).
