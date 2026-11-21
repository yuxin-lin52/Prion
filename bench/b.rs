use std::collections::HashMap;

fn fib(n: i64) -> i64 {
    if n < 2 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

const MOD: i64 = 1_000_000_007;

fn main() {
    let mut it = std::env::args().skip(1);
    let w = it.next().unwrap_or_else(|| "num".to_string());
    let arg: i64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let r: i64 = match w.as_str() {
        "num" => {
            let n = if arg != 0 { arg } else { 20_000_000 };
            let mut s = 0i64;
            for i in 1..=n {
                s = (s * 31 + i) % MOD;
            }
            s
        }
        "fib" => fib(if arg != 0 { arg } else { 35 }),
        "list" => {
            let n = if arg != 0 { arg } else { 2_000_000 } as usize;
            let mut v: Vec<i64> = Vec::with_capacity(n);
            v.extend(0..n as i64);
            v.iter().sum()
        }
        "index" => {
            let n = if arg != 0 { arg } else { 10_000 } as usize;
            let v: Vec<i64> = (0..n as i64).collect();
            let mut s = 0i64;
            for _ in 0..3000 {
                for x in &v {
                    s = (s * 31 + x) % MOD;
                }
            }
            s
        }
        "map" => {
            let n = if arg != 0 { arg } else { 200_000 } as usize;
            let mut m: HashMap<i64, i64> = HashMap::with_capacity(n);
            for i in 0..n as i64 {
                m.insert(i, i * 2);
            }
            (0..n as i64).map(|i| m[&i]).sum()
        }
        "concat" => {
            let n = if arg != 0 { arg } else { 200_000 };
            let mut k = 0;
            for _ in 0..n {
                let mut s = String::with_capacity(6);
                s.push_str("ab");
                s.push_str("cd");
                s.push_str("ef");
                std::hint::black_box(s.as_bytes());
                k += s.len() as i64;
            }
            k
        }
        _ => 0,
    };
    println!("{}", r);
}
