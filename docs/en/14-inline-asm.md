# 14 Inline assembly

## Overview

Inline assembly allows machine instructions to be embedded inside an `unsafe` block. It is one of the three unsafe-only capabilities, alongside raw address access and C interoperability.

## Form

```prion
asm("template" : output : input : clobber)
```

- **template** — the instruction string, ASCII only.
- **output** — binds a register to a Prion name: `reg -> name` (write-only) or `reg <-> name` (read-write).
- **input** — binds a register to an expression: `reg <- expr`.
- **clobber** — register names, or `memory` / `cc`.

The four segments are separated by `:`. Register names map to Prion registers, e.g. `al/ax/eax/rax -> a` and `r8..r15 -> r8..r15`.

## Example

```prion
#: raw b unsafe
    add val "reg" uint_8 = 0
    asm("movb $9, %[reg]" : al -> reg : : memory)
    print("asm out", call val "reg")
#: raw d
```

Output:

```
asm out 9
```

## Rules

- `asm` may appear only inside a block declared `#: name b unsafe`.
- `asm` is an action statement: it has no output value and cannot be assigned like an expression. Outputs are bound to existing vals.
- Every name in an output binding must already be defined in the same block with `add val`.
- The isolation region (inside the quotes) must be ASCII; a non-ASCII character is an error.

## Rationale

Assembly bypasses all of Prion's type and memory rules. Restricting it to `unsafe` blocks means that reading an ordinary block tells the reader that no assembly runs inside it.
