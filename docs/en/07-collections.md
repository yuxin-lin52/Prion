# 07 Collections

## Overview

Prion provides four builtin containers: `list`, `map`, `que` (queue) and `stack`. Each is created and manipulated through builtin functions.

## Lists

```anle
add val "l" list(int) = [3, 1, 2]
push(call val "l", 4)
sort(call val "l")
print(call val "l")        // [1, 2, 3, 4]
print(len(call val "l"))   // length
print(call val "l"[1])     // element (indices start at 1)
print(call val "l"[1~2])   // inclusive slice
```

Other list operations: `insert(list, i, v)`, `remove(list, i)`, `has`, `index`, `empty`, `slice`, `concat` and `sort`.

## Maps

```anle
add val "m" map(str, int) = {}
put(call val "m", "a", 1)
put(call val "m", "b", 2)
val "m"["b"] = 5           // index write
print(call val "m"["a"])   // read
print(keys(call val "m"), vals(call val "m"))
```

Keys may be `str` or `int`. Use `has` before reading a key that may be absent.

## Queue and stack

```anle
add val "q" que(int) = que([1, 2])
add val "first" int = pop(call val "q")   // FIFO

add val "s" stack(int) = stack()
push(call val "s", 1)
push(call val "s", 2)
add val "top" int = pop(call val "s")     // LIFO
```

Queues and stacks are push/pop only; slicing and sorting apply to lists.

## Example

```anle
#: main b
    add val "l" list(int) = [3, 1, 2]
    push(call val "l", 4)
    insert(call val "l", 1, 9)
    remove(call val "l", 2)
    sort(call val "l")
    print("list", call val "l", "len", len(call val "l"), "at", call val "l"[1], "slice", call val "l"[1~2])
    print("has 3", has(call val "l", 3), "empty", empty(call val "l"), "index 3", index(call val "l", 3))
    val "l"[1] = 7
    print("after write", call val "l"[1])

    add val "m" map(str, int) = {}
    put(call val "m", "a", 1)
    put(call val "m", "b", 2)
    val "m"["b"] = 5
    print("map a", call val "m"["a"], "b", call val "m"["b"], "keys", keys(call val "m"), "vals", vals(call val "m"))

    add val "q" que(int) = que([1, 2])
    add val "qf" int = pop(call val "q")
    print("queue", call val "qf", "left", len(call val "q"))

    add val "s" stack(int) = stack()
    push(call val "s", 1)
    push(call val "s", 2)
    add val "st" int = pop(call val "s")
    print("stack", call val "st", "left", len(call val "s"))

    print("bytes", len("hi"), "chars", char_len("hi"))
#: main d
```

Output:

```
list [1, 2, 4, 9] len 4 at 1 slice [1, 2]
has 3 fa empty fa index 3 0
after write 7
map a 1 b 5 keys ["a", "b"] vals [1, 5]
queue 1 left 1
stack 2 left 1
bytes 2 chars 2
```

## Nested containers

A container cannot directly hold another container. To group collections, use a `map(str, list(int))` or a struct field.
