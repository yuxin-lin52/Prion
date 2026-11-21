# 01 Lexical structure

In this chapter you write and run your first Prion program, then learn the small set of lexical rules that the rest of the language is built on.

## Your first program

Create `hello.an`:

```anle
#: main b
    print("hello, prion")
#: main d
```

Build and run it:

```
prionc run hello
```

Expected output:

```
hello, prion
```

Everything you write lives inside a **block**. `#: main b` opens the block named `main` (the `b` means *born*) and `#: main d` closes it (`d` means *death*). Indentation here is only for humans — the compiler does not read it. Blocks are described in [02](02-basics.md).

## Keywords

These words are reserved:

```
add call take fn val type hide give outs
if else for while match case
unsafe pub sle ABI asm fix
trait impl spawn share send
int float bool str
int_8 int_16 int_32 int_64 uint_8 uint_16 uint_32 uint_64
list map que stack
and or not null re fa
```

A keyword cannot be used as a name, except inside quotes: `add val "if" int = 1` is allowed because the name is quoted. The loop words `in` and `max` are contextual: they carry meaning only inside `for` and `while` syntax.

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

Only whole-line `//` comments exist — no `/* */` and no comments trailing code on the same line. The rest of a `//` line is ignored:

```anle
#: main b
    // this line is a comment
    print("done") // this is NOT a comment - it is parsed as code and rejected
#: main d
```

Put the comment on its own line instead:

```anle
#: main b
    // print the word done
    print("done")
#: main d
```

## Literals

- **Integers** are decimal by default; `0x` hex, `0b` binary and `0o` octal are also accepted, and `_` separates digits: `0b1111_0000` is 240.
- **Floats** use a decimal point or scientific notation: `1.5`, `1e6`, `1.5e-3`.
- **Strings** use `"…"` with the four escapes `\n \t \\ \"`. Any other escape is a compile error. A verbatim string is written `@"…"`, where backslashes are literal and `""` escapes a quote. Strings cannot span lines.
- **Booleans** are `re` (true) and `fa` (false).
- **Null** is `null`.

## Files

A source file must be **UTF-8 without BOM** with **LF** line endings. One `.an` file is one complete program or one module. The compiler rejects a BOM or CRLF with a clear error.
