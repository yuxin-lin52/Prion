# 10 Compile-time evaluation

A `fix` is a constant whose value is computed at compile time. Use it for values that never change and that you want folded into the binary.

## Declaring a constant

```anle
add fix "pi" float = 3.14159
add fix "buf" int = 4096
add fix "sum" int = 2 * 3 + 4 * 5
```

The right-hand side must be evaluable at compile time: literals, operators, and calls to `fn`s whose arguments are all constants.

## Reading a constant

A constant is read with its own verb, `call fix`:

```anle
print(call fix "pi" * 2.0)
```

Using `call val "pi"` on a constant is an error — `val` and `fix` are two families and are not mixed.

## Try it

```anle
#: main b
    add fix "pi" float = 3.14159
    add fix "buf" int = 4096
    print(call fix "pi" * 2.0)
    print(call fix "buf" + 1)
#: main d
```

Expected output:

```
6.28318
4097
```

## Folding through functions

Constants may call functions whose arguments are all constant:

```anle
add fn "double" (n int) int = n * 2
add fix "twice_six" int = call fn "double"(6)
```

## Limits

Compile-time evaluation is bounded so that a runaway const expression fails the build instead of hanging it. Two knobs control the bound:

- `--const-depth N` — maximum nesting depth (default 256).
- `--const-steps N` — maximum evaluation steps (default 1280000).

`0` means no limit. Exceeding a limit is reported as a quota error, not a syntax error.
