






use prion_ir::{LocalId, Program, Rvalue, Stmt, Value};


fn 序元素型(t: &str) -> Option<String> {
    let (head, inner) = t.split_once('(')?;
    let tail = inner.strip_suffix(')')?;
    match head {
        "list" | "que" | "stack" => Some(tail.trim().to_string()),
        _ => None,
    }
}
use std::collections::{BTreeMap, BTreeSet};

pub fn run(prog: &mut Program) {
    let mut 出: BTreeMap<String, BTreeSet<LocalId>> = BTreeMap::new();
    for f in &prog.functions {
        
        let mut 候选: BTreeSet<LocalId> = BTreeSet::new();
        for (i, l) in f.locals.iter().enumerate() {
            let et = 序元素型(&l.ty.0);
            let Some(et) = et else { continue };
            let 是积 = prog
                .types
                .iter()
                .any(|t| t.name == et && t.variants.is_empty());
            if 是积 {
                候选.insert(i);
            }
        }
        if 候选.is_empty() {
            continue;
        }
        
        let mut 元源: BTreeMap<LocalId, LocalId> = BTreeMap::new();
        for b in &f.blocks {
            for st in &b.stmts {
                if let Stmt::Assign { lhs, rhs } = st {
                    if let Rvalue::ListGet { list: Value::Local(l), .. } = rhs {
                        if 候选.contains(l) {
                            元源.insert(lhs.base, *l);
                        }
                    }
                }
            }
        }
        
        let mut 犯: BTreeSet<LocalId> = BTreeSet::new();
        let mut 摘值 = |v: &Value, 元源: &BTreeMap<LocalId, LocalId>, 犯: &mut BTreeSet<LocalId>| {
            if let Value::Local(l) = v {
                if 元源.contains_key(l) {
                    犯.insert(*l);
                }
            }
        };
        for b in &f.blocks {
            for st in &b.stmts {
                match st {
                    Stmt::Assign { lhs, rhs } => {
                        
                        match rhs {
                            Rvalue::BinOp { lhs: a, rhs: c, .. } => {
                                摘值(a, &元源, &mut 犯);
                                摘值(c, &元源, &mut 犯);
                            }
                            Rvalue::Call { args, .. } | Rvalue::CallIndirect { args, .. } => {
                                for a in args {
                                    摘值(a, &元源, &mut 犯);
                                }
                            }
                            Rvalue::Use(p) => {
                                
                                if 元源.contains_key(&p.base) && lhs.base != p.base {
                                    犯.insert(p.base);
                                }
                            }
                            Rvalue::ListLit { elems } => {
                                for e in elems {
                                    摘值(e, &元源, &mut 犯);
                                }
                            }
                            _ => {}
                        }
                    }
                    Stmt::Call { name, args } => {
                        
                        for a in args {
                            摘值(a, &元源, &mut 犯);
                        }
                        let _ = name;
                    }
                    Stmt::ListSet { value, .. } => {
                        摘值(value, &元源, &mut 犯);
                    }
                    _ => {}
                }
                
                match &b.exit {
                    prion_ir::Exit::Return(Some(v)) => 摘值(v, &元源, &mut 犯),
                    prion_ir::Exit::Branch { targets, .. } => {
                        for s in targets {
                            for a in &s.args {
                                摘值(a, &元源, &mut 犯);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        let 好: BTreeSet<LocalId> = 候选.difference(&犯).copied().collect();
        if !好.is_empty() {
            出.insert(f.name.clone(), 好);
        }
    }
    prog.平铺 = 出;
}
