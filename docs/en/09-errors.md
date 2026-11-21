# 09 Error handling

Prion has no exceptions. An operation that can fail returns a **sum type**, and the caller destructures it with `match`. This chapter shows the pattern.

## Model errors as a sum type

```anle
add type "res" = "ok"(int) | "err"(str)
```

A function returns `"ok"(value)` on success and `"err"(message)` on failure:

```anle
add fn "safe_div" (a int, b int) "res" = if b == 0 { "err"("divide by zero") } else { "ok"(a / b) }
```

## Propagate with the expression form of match

Inside a function body, `match` is also an expression. That lets you transform a result while passing the error through:

```anle
add fn "double_div" (a int, b int) "res" = match call fn "safe_div"(a, b) { case "ok"("v"): "ok"(call val "v" * 2), case "err"("why"): "err"(call val "why") }
```

## Consume with the statement form of match

At the call site, match the result and act:

```anle
match call fn "double_div"(10, 2):
    case "ok"("v"):
        print("ok", call val "v")
    ;
    case "err"("why"):
        print("err", call val "why")
    ;
;
```

## Try it

```anle
#: main b
    add type "res" = "ok"(int) | "err"(str)
    add fn "safe_div" (a int, b int) "res" = if b == 0 { "err"("divide by zero") } else { "ok"(a / b) }
    add fn "double_div" (a int, b int) "res" = match call fn "safe_div"(a, b) { case "ok"("v"): "ok"(call val "v" * 2), case "err"("why"): "err"(call val "why") }

    match call fn "double_div"(10, 2):
        case "ok"("v"):
            print("success", call val "v")
        ;
        case "err"("why"):
            print("failure", call val "why")
        ;
    ;
    match call fn "double_div"(10, 0):
        case "ok"("v"):
            print("success", call val "v")
        ;
        case "err"("why"):
            print("failure", call val "why")
        ;
    ;
#: main d
```

Expected output:

```
success 10
failure divide by zero
```

## Why this design

- Failure is part of the type, so the compiler can check that you handled it. `match` exhaustiveness means a new error variant surfaces every place that must be updated.
- There is no hidden control flow: no exception unwinding, no stack of in-flight cleanups. The caller sees the error in the return type and decides what to do.
- The standard library ships `result(T, E)` and `option(T)` types built this way; see the appendix for the builtins you will use most.
