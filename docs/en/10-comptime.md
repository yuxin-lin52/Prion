# 10 Compile-time evaluation

## Overview

A `fix` is a constant whose value is computed at compile time. It is used for values that do not change and that should be folded into the binary.

## Declaration and reading

```anle
add fix "pi" float = 3.14159
add fix "buf" int = 4096
add fix "sum" int = 2 * 3 + 4 * 5
```

A constant is read with its own verb, `call fix`:

```anle
print(call fix "pi" * 2.0)
```

Using `call val` on a constant is an error: `val` and `fix` are separate families and are not mixed.

## Constant expressions

The right-hand side must be evaluable at compile time: literals, operators, and calls to `fn`s whose arguments are all constants.

```anle
add fn "double" (n int) int = n * 2
add fix "twice_six" int = call fn "double"(6)
```

## Example

```anle
#: main b
    add fix "pi" float = 3.14159
    add fix "buf" int = 4096
    add fn "double" (n int) int = n * 2
    add fix "twice_six" int = call fn "double"(6)
    print(call fix "pi" * 2.0)
    print(call fix "buf" + 1)
    print(call fix "twice_six")
#: main d
```

Output:

```
6.28318
4097
12
```

## Quotas

Compile-time evaluation is bounded so that a runaway constant expression fails the build instead of hanging it:

- `--const-depth N` — maximum nesting depth (default 256).
- `--const-steps N` — maximum evaluation steps (default 1280000).

`0` means no limit. Exceeding a quota is reported as a quota error, not a syntax error.
