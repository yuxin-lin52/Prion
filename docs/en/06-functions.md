# 06 Functions

Functions are created once with `add fn`, called with `call fn "name"(…)`, and can be used as first-class values.

## Creating and calling

```anle
add fn "double" (x int) int = x * 2
add fn "add" (a int, b int) int = a + b
```

The body after `=` is a single expression. Call it with `call fn "double"(21)`.

## Declare, then supply the body

Recursive and mutually recursive functions use a declaration (`add fn` with no `=`) plus a body form (`fn` without `add`):

```anle
add fn "try_div" (n int, i int) bool
add fn "is_prime" (n int) bool = if n < 2 { fa } else { call fn "try_div"(n, 2) }
fn "try_div" (n int, i int) bool = if i * i > n { re } else { if n % i == 0 { fa } else { call fn "try_div"(n, i + 1) } }
```

## Statement-body functions with `give`

When a function needs intermediate steps, open a body with `:` and return with `give`:

```anle
add fn "total"(n int) int:
    add val "s" int = 0
    add val "i" int = 1
    while call val "i" <= call val "n":
        val "s" = call val "s" + call val "i"
        val "i" = call val "i" + 1
    ;
    give call val "s"
;
```

## First-class functions

An `fn` value is written `fn "name"`; its type is `fn(args) ret`. Call through a value with `call val "f"(…)`:

```anle
add fn "inc" (x int) int = x + 1
add fn "twice" (f fn(int) int, n int) int = call val "f"(n)
add val "op" fn(int) int = fn "inc"
print(call fn "twice"(call val "op", 41))
```

## Try it

```anle
#: main b
    add fn "try_div" (n int, i int) bool
    add fn "is_prime" (n int) bool = if n < 2 { fa } else { call fn "try_div"(n, 2) }
    fn "try_div" (n int, i int) bool = if i * i > n { re } else { if n % i == 0 { fa } else { call fn "try_div"(n, i + 1) } }
    print("prime", call fn "is_prime"(7), call fn "is_prime"(8))

    add fn "total"(n int) int:
        add val "s" int = 0
        add val "i" int = 1
        while call val "i" <= call val "n":
            val "s" = call val "s" + call val "i"
            val "i" = call val "i" + 1
        ;
        give call val "s"
    ;
    print("total", call fn "total"(4))

    add fn "inc" (x int) int = x + 1
    add fn "twice" (f fn(int) int, n int) int = call val "f"(n)
    add val "op" fn(int) int = fn "inc"
    print("hfn", call fn "twice"(call val "op", 41))
#: main d
```

Expected output:

```
prime re fa
total 10
hfn 42
```

## Rules to remember

- Functions are **define-before-use**: a call must come after its `add fn`. A bare declaration counts as defined; the body may come later.
- The number and types of arguments must match the declaration exactly.
- Parameter and local types are mandatory at function boundaries.
