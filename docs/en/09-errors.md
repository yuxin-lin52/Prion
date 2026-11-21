# 09 Error handling

## Overview

Prion has no exceptions. An operation that can fail returns a **sum type**, and the caller destructures it with `match` or propagates it with `?`.

## Modelling errors

```anle
add type "Result" = Err(str) | Ok(int)
```

The error variant is declared first. `?` treats the first variant as the error and the second as the success.

A function returns `Ok(value)` on success and `Err(message)` on failure:

```anle
add fn "parse" (s str) "Result" = if s == "bad" { Err("oops") } else { Ok(42) }
```

## Propagation with `?`

Inside a function that returns the same sum type, `?` unwraps a success value and returns the error immediately if the value is a failure:

```anle
add fn "doit" (s str) "Result":
    add val "n" int = call fn "parse"(call val s)?
    give Ok(call val "n")
;
```

## Dispatch with match

Expression form transforms a result while passing the error through:

```anle
add fn "double" (n int) "Result" = match call fn "parse"("good") { case Ok("v"): Ok(call val "v" * 2), case Err("e"): Err(call val "e") }
```

Statement form consumes a result at the call site:

```anle
match call fn "doit"("good"):
    case Ok("v"):
        print(call val "v")
    ;
    case Err("e"):
        print(call val "e")
    ;
;
```

## Example

```anle
#: main b
    add type "Result" = Err(str) | Ok(int)
    add fn "parse" (s str) "Result" = if s == "bad" { Err("oops") } else { Ok(42) }
    add fn "doit" (s str) "Result":
        add val "n" int = call fn "parse"(call val s)?
        give Ok(call val "n")
    ;

    match call fn "doit"("good"):
        case Ok("v"):
            print("success", call val "v")
        ;
        case Err("e"):
            print("failure", call val "e")
        ;
    ;
    match call fn "doit"("bad"):
        case Ok("v"):
            print("success", call val "v")
        ;
        case Err("e"):
            print("failure", call val "e")
        ;
    ;
#: main d
```

Output:

```
success 42
failure oops
```

## Notes

- Failure is part of the type, so the compiler checks that it is handled; `match` coverage is checked for exhaustiveness.
- There is no hidden control flow: no exception unwinding and no cleanup stack. The caller sees the error in the return type.
- The standard library ships `result(T, E)` and `option(T)` built on the same pattern.
