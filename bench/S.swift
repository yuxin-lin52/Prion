func num(_ n: Int) -> Int {
    var s = 0
    var i = 1
    while i <= n {
        s = (s * 31 + i) % 1_000_000_007
        i += 1
    }
    return s
}

func fib(_ n: Int) -> Int {
    return n < 2 ? n : fib(n - 1) + fib(n - 2)
}

func listB(_ n: Int) -> Int {
    var v = [Int]()
    v.reserveCapacity(n)
    for i in 0..<n { v.append(i) }
    var s = 0
    for x in v { s += x }
    return s
}

func indexB(_ n: Int) -> Int {
    var v = [Int]()
    v.reserveCapacity(n)
    for i in 0..<n { v.append(i) }
    var s = 0
    for _ in 0..<3000 {
        for x in v { s = (s * 31 + x) % 1_000_000_007 }
    }
    return s
}

func mapB(_ n: Int) -> Int {
    var m = [Int: Int](minimumCapacity: n)
    for i in 0..<n { m[i] = i * 2 }
    var s = 0
    for i in 0..<n { s += m[i]! }
    return s
}

func concatB(_ n: Int) -> Int {
    let parts = ["ab", "cd", "ef"]
    var k = 0
    for _ in 0..<n {
        var s = ""
        s.reserveCapacity(6)
        for p in parts { s += p }
        k += s.utf8.count
    }
    return k
}

let args = CommandLine.arguments
let mode = args.count > 1 ? args[1] : "num"
let n = args.count > 2 ? (Int(args[2]) ?? 0) : 0
var r = 0
if mode == "none" {
    r = 0
} else {
    switch mode {
    case "num": r = num(n)
    case "fib": r = fib(n)
    case "list": r = listB(n)
    case "index": r = indexB(n)
    case "map": r = mapB(n)
    case "concat": r = concatB(n)
    default: r = 0
    }
}
print(r)
