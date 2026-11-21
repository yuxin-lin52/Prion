# 11 Concurrency

## Overview

Concurrency is expressed with task tokens. `spawn` starts a task body and yields a `token(T)`; `join` waits for the task and returns its value. A token must be joined before its block ends; nothing is detached implicitly.

## Spawn and join

```anle
add val "t" token(int) = spawn:
    give 42
;
add val "r" int = join(call val "t")
```

`spawn:` opens a task body and `give` supplies the result. A token that would escape its block is a compile error.

A task that needs no result ends with a lone `join` and is self-terminating:

```anle
add val "t" token(void) = spawn:
    add val "x" int = 1 + 1
    join
;
```

A self-terminating task may not capture outer values, because the parent block may exit first.

## Sharing and moving

By default a task may not touch outer values, because the compiler cannot decide whether that is thread-safe. This is opted into with a marker:

- `add share val "n" int = 21` — read-only sharing across tasks.
- `add val(send) "x" int = 7` — move ownership into one task.

`share` and `(send)` are mutually exclusive. A `share` value cannot be written. A `(send)` value becomes invalid in the original block once moved, and a value can be moved only once.

## Example

```anle
#: main b
    add val "t" token(int) = spawn:
        give 42
    ;
    add val "r" int = join(call val "t")
    print("result", call val "r")

    add share val "base" int = 21
    add val "u" token(int) = spawn:
        give call val "base" * 2
    ;
    print("shared", join(call val "u"))
#: main d
```

Output:

```
result 42
shared 42
```

## Rules

- Every `spawn` produces a token; every token is joined, or its task body self-terminates with `join`.
- A container of tokens is joined as a whole within the block.
- A failure propagates through `join`.
- A value moves at most once; for read-only sharing use `add share val`.
