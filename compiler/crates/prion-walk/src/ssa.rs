







use prion_ir::{BlockId, Exit, Function, LocalId, Place, Rvalue, Stmt, Value};
use std::collections::BTreeSet;


fn 后继(f: &Function, b: usize) -> Vec<BlockId> {
    match &f.blocks[b].exit {
        Exit::Branch { targets, .. } => targets.iter().map(|s| s.target).collect(),
        _ => Vec::new(),
    }
}


pub fn 前驱表(f: &Function) -> Vec<Vec<BlockId>> {
    let mut ps: Vec<Vec<BlockId>> = vec![Vec::new(); f.blocks.len()];
    for i in 0..f.blocks.len() {
        for t in 后继(f, i) {
            if (t as usize) < ps.len() {
                ps[t as usize].push(i as BlockId);
            }
        }
    }
    ps
}


fn 链深(idom: &[Option<BlockId>], mut b: BlockId) -> usize {
    let mut d = 0usize;
    let mut 圈 = 0;
    while let Some(p) = idom[b as usize] {
        d += 1;
        b = p;
        圈 += 1;
        if 圈 > 100000 {
            break;
        }
    }
    d
}


fn 求交(idom: &[Option<BlockId>], mut a: BlockId, mut b: BlockId) -> BlockId {
    let mut 圈 = 0;
    while a != b {
        let da = 链深(idom, a);
        let db = 链深(idom, b);
        let step = if da > db { a } else { b };
        let Some(p) = idom[step as usize] else {
            return step;
        };
        if da > db {
            a = p;
        } else {
            b = p;
        }
        圈 += 1;
        if 圈 > 100000 {
            break;
        }
    }
    a
}


pub fn 支配树(f: &Function) -> Vec<Option<BlockId>> {
    let n = f.blocks.len();
    let mut idom: Vec<Option<BlockId>> = vec![None; n];
    if n == 0 {
        return idom;
    }
    let ps = 前驱表(f);
    let mut 变 = true;
    let mut 轮 = 0;
    while 变 && 轮 < 500 {
        变 = false;
        轮 += 1;
        for b in 1..n {
            let mut 新: Option<BlockId> = None;
            for &p in &ps[b] {
                
                if idom[p as usize].is_none() && p != 0 {
                    continue;
                }
                新 = match 新 {
                    None => Some(p),
                    Some(x) => Some(求交(&idom, x, p)),
                };
            }
            if 新 != idom[b] {
                idom[b] = 新;
                变 = true;
            }
        }
    }
    idom
}


pub fn 支配(idom: &[Option<BlockId>], a: BlockId, b: BlockId) -> bool {
    if a == b {
        return true;
    }
    let mut cur = b;
    let mut 圈 = 0;
    while let Some(p) = idom[cur as usize] {
        if p == a {
            return true;
        }
        cur = p;
        圈 += 1;
        if 圈 > 100000 {
            break;
        }
    }
    false
}


fn 是标量(t: &str) -> bool {
    matches!(t, "int" | "float" | "bool") || t.ends_with('*')
}


fn 带投影(p: &Place) -> bool {
    !p.proj.is_empty()
}


fn 读值(v: &Value, out: &mut Vec<(LocalId, usize, usize)>) {
    if let Value::Local(id) = v {
        out.push((*id, 0, 0));
    }
}


fn 读右值(r: &Rvalue, out: &mut Vec<(LocalId, usize, usize)>) {
    match r {
        Rvalue::Use(p) => {
            if 带投影(p) {
                
                out.push((p.base, 1, 0));
            } else {
                out.push((p.base, 0, 0));
            }
        }
        Rvalue::Ref(p) => {
            
            out.push((p.base, 1, 0));
        }
        Rvalue::Const(v) => 读值(v, out),
        Rvalue::BinOp { lhs, rhs, .. } => {
            读值(lhs, out);
            读值(rhs, out);
        }
        Rvalue::Call { args, .. } | Rvalue::CallIndirect { args, .. } => {
            for a in args {
                读值(a, out);
            }
        }
        Rvalue::ListLit { elems } => {
            for e in elems {
                读值(e, out);
            }
        }
        Rvalue::ListGet { list, index } => {
            读值(list, out);
            读值(index, out);
        }
        Rvalue::MapLit { pairs } => {
            for (_, v) in pairs {
                读值(v, out);
            }
        }
        Rvalue::MapGet { map, key } => {
            读值(map, out);
            读值(key, out);
        }
        Rvalue::VariantLit { args, .. } => {
            for a in args {
                读值(a, out);
            }
        }
        Rvalue::VariantTag(v) => 读值(v, out),
        Rvalue::VariantGet { value, .. } => 读值(value, out),
        Rvalue::FnRef { .. } => {}
    }
}


fn 读语句(s: &Stmt, out: &mut Vec<(LocalId, usize, usize)>) {
    match s {
        Stmt::Assign { lhs, rhs } => {
            读右值(rhs, out);
            if 带投影(lhs) {
                out.push((lhs.base, 1, 0));
            }
        }
        Stmt::Call { args, .. } => {
            for a in args {
                读值(a, out);
            }
        }
        Stmt::ListSet { list, index, value } => {
            读值(list, out);
            读值(index, out);
            读值(value, out);
        }
        Stmt::MapSet { map, key, value } => {
            读值(map, out);
            读值(key, out);
            读值(value, out);
        }
        Stmt::Assert { cond, .. } => 读值(cond, out),
        Stmt::AccessBegin(p) | Stmt::AccessEnd(p) => {
            if 带投影(p) {
                out.push((p.base, 1, 0));
            }
        }
        Stmt::Alloc { .. } | Stmt::Bind { .. } | Stmt::Drop { .. } | Stmt::DropVar { .. } | Stmt::Kill { .. } => {}
        Stmt::Asm(form) => {
            
            for (_, v, _) in &form.outs {
                out.push((*v, 1, 0));
            }
            for (_, v) in &form.ins {
                读值(v, out);
            }
        }
    }
}


pub fn ssa值局部(f: &Function) -> BTreeSet<LocalId> {
    let n = f.locals.len();
    
    let mut 定义数 = vec![0usize; n];
    let mut 定义处: Vec<Option<(usize, usize)>> = vec![None; n];
    
    let mut 使用: Vec<Vec<(usize, usize, u8)>> = vec![Vec::new(); n];
    
    let mut 取址: BTreeSet<LocalId> = BTreeSet::new();
    for (bi, b) in f.blocks.iter().enumerate() {
        for (si, s) in b.stmts.iter().enumerate() {
            let mut 读 = Vec::new();
            读语句(s, &mut 读);
            for (id, 标, _) in 读 {
                if id < n {
                    使用[id].push((bi, si, 标 as u8));
                }
            }
            if let Stmt::Assign { lhs, rhs } = s {
                if !带投影(lhs) && lhs.base < n {
                    定义数[lhs.base] += 1;
                    定义处[lhs.base] = Some((bi, si));
                }
                if let Rvalue::Ref(p) = rhs {
                    取址.insert(p.base);
                }
            }
        }
        
        let mut 读 = Vec::new();
        match &b.exit {
            Exit::Return(Some(v)) => 读值(v, &mut 读),
            Exit::Branch { cond, targets } => {
                读值(cond, &mut 读);
                
                for s in targets {
                    for a in &s.args {
                        读值(a, &mut 读);
                    }
                }
            }
            _ => {}
        }
        for (id, _, _) in 读 {
            if id < n {
                使用[id].push((bi, b.stmts.len(), 0));
            }
        }
    }
    let idom = 支配树(f);
    
    let mut 块参: BTreeSet<LocalId> = BTreeSet::new();
    for b in &f.blocks {
        for (pid, _) in &b.params {
            块参.insert(*pid);
        }
    }
    let mut 出 = BTreeSet::new();
    for i in 0..n {
        let l = &f.locals[i];
        if 块参.contains(&(i as LocalId)) {
            continue;
        }
        if 定义数[i] != 1 {
            continue;
        }
        if !是标量(&l.ty.0) {
            continue;
        }
        if l.is_param {
            continue;
        }
        if 取址.contains(&i) {
            continue;
        }
        if l.name.as_deref().map(|x| x.contains('#')).unwrap_or(false) {
            continue;
        }
        let Some((db, ds)) = 定义处[i] else {
            continue;
        };
        
        let mut 好 = true;
        for &(ub, us, 标) in &使用[i] {
            if 标 == 1 {
                
                好 = false;
                break;
            }
            if ub == db {
                if us <= ds {
                    好 = false;
                    break;
                }
            } else if !支配(&idom, db as BlockId, ub as BlockId) {
                好 = false;
                break;
            }
        }
        if !好 {
            continue;
        }
        出.insert(i);
    }
    出
}
