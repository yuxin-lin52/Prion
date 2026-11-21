# 06 Functions

## Overview

A function is created with `add fn`, called with `call fn "name"(…)`, and may be used as a first-class value. Function boundaries require explicit types.

## Expression bodies

```prion
add fn "double" (x int) int = x * 2
add fn "add" (a int, b int) int = a + b
```

The body after `=` is a single expression. Recursive and mutually recursive functions use a declaration (an `add fn` with no `=`) followed by a body form (`fn` without `add`):

```prion
add fn "try_div" (n int, i int) bool
add fn "is_prime" (n int) bool = if n < 2 { fa } else { call fn "try_div"(n, 2) }
fn "try_div" (n int, i int) bool = if i * i > n { re } else { if n % i == 0 { fa } else { call fn "try_div"(n, i + 1) } }
```

## Statement bodies with give

A body that needs intermediate steps opens with `:` and returns with `give`:

```prion
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

An intermediate `give` returns immediately. Every `give`, intermediate or final, must match the declared return type, and a statement body must end with one.

## First-class functions

A function value is written `fn "name"`; its type is `fn(args) ret`. It is called through a value with `call val "f"(…)`:

```prion
add fn "inc" (x int) int = x + 1
add fn "twice" (f fn(int) int, n int) int = call val "f"(n)
add val "op" fn(int) int = fn "inc"
print(call fn "twice"(call val "op", 41))
```

## Example

```prion
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

Output:

```
prime re fa
total 10
hfn 42
```

## Rules

- Functions are defined before use: a call must come after its `add fn`. A declaration counts as defined; the body may follow later.
- Argument count and argument types must match the declaration.
- Parameter and return types are mandatory at function boundaries.
