use prion_ir::{BinOp, Block, Exit, Program, Rvalue, Stmt, Value};
use std::collections::BTreeSet;

fn 容器预留名(t: &str) -> Option<&'static str> {
    if t.starts_with("list(") || t.starts_with("que(") || t.starts_with("stack(") {
        Some("#预留列表")
    } else if t.starts_with("map(") {
        Some("#预留映射")
    } else {
        None
    }
}

fn 后继(b: &Block) -> Vec<usize> {
    match &b.exit {
        Exit::Branch { targets, .. } => targets.iter().map(|s| s.target).collect(),
        _ => Vec::new(),
    }
}

fn 可达(起: usize, 块数: usize, 后继表: &[Vec<usize>]) -> BTreeSet<usize> {
    let mut 见 = BTreeSet::new();
    let mut 栈 = vec![起];
    while let Some(x) = 栈.pop() {
        if x >= 块数 || !见.insert(x) {
            continue;
        }
        for y in &后继表[x] {
            栈.push(*y);
        }
    }
    见
}

fn 循环内赋值(环: &BTreeSet<usize>, 块: &[Block]) -> BTreeSet<usize> {
    let mut 赋值 = BTreeSet::new();
    for i in 环 {
        for st in &块[*i].stmts {
            if let Stmt::Assign { lhs, .. } | Stmt::AssignRelease { lhs, .. } = st {
                if lhs.proj.is_empty() {
                    赋值.insert(lhs.base);
                }
            }
        }
    }
    赋值
}

fn 变的是(v: &Value, 环赋值: &BTreeSet<usize>) -> bool {
    matches!(v, Value::Local(x) if 环赋值.contains(x))
}

fn 循环上界(头: &Block, 环赋值: &BTreeSet<usize>) -> Option<Value> {
    let Exit::Branch { cond, .. } = &头.exit else { return None };
    let Value::Local(c) = cond else { return None };
    for st in &头.stmts {
        let Stmt::Assign { lhs, rhs } = st else { continue };
        if lhs.base != *c || !lhs.proj.is_empty() {
            continue;
        }
        let Rvalue::BinOp { op: BinOp::Lt | BinOp::Le, lhs: a, rhs: b } = rhs else { continue };
        let (a变, b变) = (变的是(a, 环赋值), 变的是(b, 环赋值));
        if a变 == b变 {
            continue;
        }
        let 界 = if b变 { a } else { b };
        match 界 {
            Value::Local(_) => return Some(界.clone()),
            Value::Int(k) if *k > 0 => return Some(界.clone()),
            _ => {}
        }
    }
    None
}

pub fn run(prog: &mut Program) {
    for f in prog.functions.iter_mut() {
        let 块数 = f.blocks.len();
        let mut 前驱: Vec<Vec<usize>> = vec![Vec::new(); 块数];
        let mut 后继表: Vec<Vec<usize>> = Vec::with_capacity(块数);
        for (i, b) in f.blocks.iter().enumerate() {
            let ss = 后继(b);
            for t in &ss {
                if *t < 块数 {
                    前驱[*t].push(i);
                }
            }
            后继表.push(ss);
        }
        let mut 计划: Vec<(usize, usize, Value)> = Vec::new();
        for (h, _) in f.blocks.iter().enumerate() {
            let 从头发可达 = 可达(h, 块数, &后继表);
            let 回边前驱: Vec<usize> = 前驱[h]
                .iter()
                .copied()
                .filter(|p| *p != h && 从头发可达.contains(p))
                .collect();
            if 回边前驱.is_empty() {
                continue;
            }
            let mut 环: BTreeSet<usize> = BTreeSet::new();
            let mut 栈: Vec<usize> = 回边前驱.clone();
            while let Some(x) = 栈.pop() {
                if x == h || !环.insert(x) {
                    continue;
                }
                for p in &前驱[x] {
                    栈.push(*p);
                }
            }
            环.insert(h);
            let 有早退 = 环.iter().any(|i| {
                matches!(f.blocks[*i].exit, Exit::Return(_))
            });
            if 有早退 {
                continue;
            }
            let 环赋值 = 循环内赋值(&环, &f.blocks);
            let Some(界) = 循环上界(&f.blocks[h], &环赋值) else { continue };
            let mut 目标: BTreeSet<usize> = BTreeSet::new();
            for i in &环 {
                for st in &f.blocks[*i].stmts {
                    let Stmt::Call { name, args } = st else { continue };
                    if name != "push" && name != "put" {
                        continue;
                    }
                    let Some(Value::Local(l)) = args.first() else { continue };
                    if 环赋值.contains(l) {
                        continue;
                    }
                    let Some(t) = f.locals.get(*l).map(|x| x.ty.0.clone()) else { continue };
                    if 容器预留名(&t).is_none() {
                        continue;
                    }
                    目标.insert(*l);
                }
            }
            if 目标.is_empty() {
                continue;
            }
            let 前置: Vec<usize> = 前驱[h].iter().copied().filter(|p| !环.contains(p)).collect();
            if 前置.len() != 1 {
                continue;
            }
            for l in 目标 {
                计划.push((前置[0], l, 界.clone()));
            }
        }
        for (pre, l, 界) in 计划 {
            let t = f.locals[l].ty.0.clone();
            let Some(名) = 容器预留名(&t) else { continue };
            let 定义在前 = f.blocks.iter().enumerate().any(|(bi, b)| {
                bi <= pre
                    && b.stmts.iter().any(|s| {
                        matches!(s, Stmt::Assign { lhs, .. } if lhs.base == l && lhs.proj.is_empty())
                    })
            });
            if !定义在前 {
                continue;
            }
            f.blocks[pre].stmts.push(Stmt::Call {
                name: 名.to_string(),
                args: vec![Value::Local(l), 界],
            });
        }
    }
}
