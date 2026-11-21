# 14 Inline assembly

Inline assembly lets you drop down to machine instructions inside an `unsafe` block. It is one of the four unsafe-only capabilities (along with raw address access, C interop and `kill`).

## The form

```anle
asm("template" : output : input : clobber)
```

- The **template** is the instruction string (ASCII only).
- The **output** segment binds a register to a Prion name: `reg -> name` (write-only) or `reg <-> name` (read-write).
- The **input** segment binds a register to an expression: `reg <- expr`.
- The **clobber** segment lists registers or `memory` / `cc` that the assembly clobbers.

All four segments are separated by `:`. A register name is mapped to a Prion register, e.g. `al/ax/eax/rax -> a`, `r8..r15 -> r8..r15`.

## Try it

```anle
#: raw b unsafe
    add val "reg" uint_8 = 0
    asm("movb $9, %[reg]" : al -> reg : : memory)
    print("asm out", call val "reg")
#: raw d
```

Expected output:

```
asm out 9
```

## Rules

- `asm` may only appear inside a block declared `#: name b unsafe`.
- `asm` is an **action statement** — it has no output value and cannot be assigned like an expression. Bind outputs to existing vals instead.
- Every name in an output binding must already be defined in the same block with `add val`.
- The isolation region (inside the quotes) must be ASCII; any non-ASCII character is an error.

## Why it is fenced off

Assembly bypasses all of Prion's type and memory rules. Restricting it to `unsafe` blocks means that reading a normal block tells you exactly whether dangerous code can run inside it.
