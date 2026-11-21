# 15 Appendix

Reference tables for keywords, builtins, operators and error codes.

## Keywords

```
add call fn val type fix give take outs
if else for while in max match case
unsafe pub sle ABI hide kill asm
and or not null re fa
int float bool str
int_8 int_16 int_32 int_64
uint_8 uint_16 uint_32 uint_64
list map que stack
```

## Builtin functions

```
print push len char_len nth_char empty has index pop insert remove
clear sort slice concat code char put map que stack keys vals
args read_file save_file exit join run
sle_ms rnd now sqrt print_raw
read_8 read_16 read_32 read_64
write_8 write_16 write_32 write_64
str_to_c str_from_c asm
```

The `read_*` / `write_*` / `str_to_c` / `str_from_c` / `asm` builtins are unsafe-only.

## Operator families

| Kind | Operators |
|---|---|
| Arithmetic | `+ - * / %` |
| Comparison | `< > <= >= == !=` |
| Logic (words) | `and or not` |
| Bitwise | `& \| ^ ! << >>` |
| Range | `~` |

`/` and `%` by zero, and integer overflow, are reported at run time in `debug` mode.

## Error code ranges

| Range | Meaning |
|---|---|
| `E0xx` | lexical (bad character, bad escape, CRLF/BOM, number format) |
| `E1xx` / `E2xx` | syntax and semantic checks (names, types, blocks, control flow) |
| `E4xx` | code generation / linking |
| `E5xx` | command-line usage |
| `E6xx` | environment (files, directories, toolchain) |
| `R0x` | run-time errors (bounds, overflow, division by zero, I/O) |

A diagnostic has the shape:

```
[E201] error at file.an line 2, column 5, in <context>: <problem>; source: <line>
Expected: ...
Fix: ...
Details: ...
```

## Command summary

```
prionc build [name] [--mode debug|release] [--opt 0-3]
prionc run   [name] [--mode ...] [--opt ...] [-- args...]
prionc check [name]
prionc fmt   [path] [--check]
prionc clean [name]
prionc -V | prionc -h
```

Exit codes: `0` success, `101` Prion error, `1` usage error.
