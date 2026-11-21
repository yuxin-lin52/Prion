# 11 Concurrency

Prion expresses concurrency with a *task token*. You `spawn` a task body, which yields a `token(T)`, and you must `join` it before the block ends. Nothing is implicitly detached.

## Spawning and joining

```anle
add val "t" token(int) = spawn:
    give 42
;
add val "r" int = join(call val "t")
```

`spawn:` opens a task body and `give` supplies the result. `join(call val "t")` waits for the task and returns its value. Every spawned token must be joined — a token that escapes its block is a compile error.

## Sharing versus sending

By default a task may not touch outer values, because the compiler cannot know whether that is thread-safe. You opt in with a marker:

- `add share val "n" int = 21` — read-only sharing across tasks.
- `add val(send) "x" int = 7` — move ownership into a single task.

`share` and `(send)` are mutually exclusive, and a `share` value cannot be written. A `(send)` value becomes invalid in the original block once moved.

## Try it

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

Expected output:

```
result 42
shared 42
```

## Self-terminating tasks

When a task needs no result, end its body with a lone `join` to mark it self-terminating:

```anle
add val "t" token(void) = spawn:
    add val "x" int = 1 + 1
    join
;
```

A self-terminating task may not capture outer values, because the parent block might exit first.

## Rules in one place

- Every spawn produces a token; every token is joined, or its body self-terminates with `join`.
- A token container must be joined as a whole within the block.
- A value moves at most once; to share for read only, use `add share val`.
