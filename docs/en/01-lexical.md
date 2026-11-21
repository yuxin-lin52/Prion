# 01 Lexical structure

## Overview

Prion source is UTF-8 without BOM with LF line endings. A `.an` file is one complete program or one module. This chapter defines the lexical units: keywords, symbols, comments, and literals.

## A minimal program

```prion
#: main b
    print("hello, prion")
#: main d
```

Build and run it:

```
prionc run hello
```

Output:

```
hello, prion
```

All code resides in blocks. `#: main b` opens the block `main` (`b` = born); `#: main d` closes it (`d` = death). Indentation is decorative and is not read by the compiler. Blocks are defined in [02](02-basics.md).

## Keywords

The following words are reserved and cannot be used as names:

```
add call take fn val type hide give outs
if else for while match case
unsafe pub sle ABI asm fix
spawn share send
int float bool str
int_8 int_16 int_32 int_64 uint_8 uint_16 uint_32 uint_64
list map que stack
and or not null re fa
```

A name may be written with or without quotes. Quoting is required only when the name is a reserved word or contains characters that cannot appear in an identifier: a keyword can be used as a name only when quoted, as in `add val "if" int = 1`. The loop words `in` and `max` are contextual: they carry meaning only within `for` and `while` syntax and are not reserved.

## Symbols and markers

Ordinary symbols:

```
= : ; ~ , . ( ) [ ] { } + - * / % < > ? & | ^ ! == != <= >= << >>
```

Structural markers:

- `#:` opens or closes a block.
- `##:` reaches into a module.
- `###:` reaches into a package.

## Comments

Comments are whole-line `//` only; there are no block comments and no trailing comments on a code line. The remainder of a `//` line is ignored by the compiler.

```prion
#: main b
    // this line is a comment
    print("done")
#: main d
```

A `//` appended to a statement is parsed as code and is rejected.

## Literals

| Kind | Form |
|---|---|
| Integer | decimal; `0x` hex, `0b` binary, `0o` octal; `_` separates digits |
| Float | decimal point or exponent: `1.5`, `1e6`, `1.5e-3` |
| String | `"…"` with the four escapes `\n \t \\ \"`; verbatim `@"…"` |
| Boolean | `re` (true), `fa` (false) |
| Null | `null` |

Any escape other than `\n \t \\ \"` is a compile error. A verbatim string treats backslashes literally and uses `""` for a quote. Strings cannot span lines.

## File contract

The compiler rejects a byte-order mark and CRLF line endings with an explicit diagnostic. Files must be saved as UTF-8 without BOM and LF.
