

use prion_ir::{is_container_ty, Exit, Function, Local, LocalId, ObjId, ObjEnt, Place, Proj, Program, Rvalue, Stmt, StorageClass, Ty, Value};
use std::collections::{BTreeMap, BTreeSet};

fn 是堆型(t: &str) -> bool {
    let n = t.trim_end_matches('*');
    t == "str"
        || t.ends_with('*')
        || is_container_ty(n)
        || matches!(n, "list" | "map" | "que" | "stack")
}

fn 语句读(s: &Stmt, out: &mut std::collections::BTreeSet<LocalId>) {
    use std::collections::BTreeSet;
    fn 值用(v: &Value, out: &mut BTreeSet<LocalId>) {
        if let Value::Local(id) = v {
            out.insert(*id);
        }
    }
    fn 右用(r: &Rvalue, out: &mut BTreeSet<LocalId>) {
        match r {
            Rvalue::Use(p) | Rvalue::Ref(p) => {
                out.insert(p.base);
            }
            Rvalue::Const(v) => 值用(v, out),
            Rvalue::BinOp { lhs, rhs, .. } => {
                值用(lhs, out);
                值用(rhs, out);
            }
            Rvalue::Call { args, .. } => args.iter().for_each(|a| 值用(a, out)),
            Rvalue::CallIndirect { callee, args } => {
                值用(callee, out);
                args.iter().for_each(|a| 值用(a, out));
            }
            Rvalue::ListLit { elems } => elems.iter().for_each(|e| 值用(e, out)),
            Rvalue::ListGet { list, index } => {
                值用(list, out);
                值用(index, out);
            }
            Rvalue::MapLit { pairs } => pairs.iter().for_each(|(_, v)| 值用(v, out)),
            Rvalue::MapGet { map, key } => {
                值用(map, out);
                值用(key, out);
            }
            Rvalue::CtxArray { args } => args.iter().for_each(|a| 值用(a, out)),
            Rvalue::CtxGet { ctx, .. } => {
                out.insert(*ctx);
            }
            Rvalue::VariantLit { args, .. } => args.iter().for_each(|a| 值用(a, out)),
            Rvalue::VariantTag(v) => 值用(v, out),
            Rvalue::VariantGet { value, .. } => 值用(value, out),
            Rvalue::FnRef { .. } => {}
        }
    }
    match s {
        Stmt::Assign { lhs, rhs } | Stmt::AssignRelease { lhs, rhs } => {
            右用(rhs, out);
            if !lhs.proj.is_empty() {
                out.insert(lhs.base);
            }
        }
        Stmt::Call { args, .. } => args.iter().for_each(|a| 值用(a, out)),
        Stmt::ListSet { list, index, value } => {
            值用(list, out);
            值用(index, out);
            值用(value, out);
        }
        Stmt::MapSet { map, key, value } => {
            值用(map, out);
            值用(key, out);
            值用(value, out);
        }
        Stmt::Assert { cond, .. } => 值用(cond, out),
        Stmt::Asm(f) => f.ins.iter().for_each(|(_, v)| 值用(v, out)),
        Stmt::AccessBegin(_) | Stmt::AccessEnd(_)
        | Stmt::Alloc { .. } | Stmt::Bind { .. }
        | Stmt::Drop { .. } | Stmt::DropVar { .. } => {}
    }
}



fn 值新鲜(
    prog: &Program,
    定义: &std::collections::BTreeMap<usize, std::collections::BTreeMap<LocalId, Vec<Rvalue>>>,
    新鲜fn: &std::collections::BTreeSet<usize>,
    fi: usize,
    r: &Rvalue,
    visiting: &mut std::collections::BTreeSet<LocalId>,
    f: &Function,
) -> bool {
    match r {
        Rvalue::Call { name, .. } => prog
            .functions
            .iter()
            .position(|g| &g.name == name)
            .map(|j| 新鲜fn.contains(&j))
            .unwrap_or(false),
        Rvalue::Const(Value::Local(x)) => 局部新鲜(prog, 定义, 新鲜fn, fi, *x, visiting, f),
        Rvalue::Const(_) => true,
        _ => false,
    }
}

fn 局部新鲜(
    prog: &Program,
    定义: &std::collections::BTreeMap<usize, std::collections::BTreeMap<LocalId, Vec<Rvalue>>>,
    新鲜fn: &std::collections::BTreeSet<usize>,
    fi: usize,
    l: LocalId,
    visiting: &mut std::collections::BTreeSet<LocalId>,
    f: &Function,
) -> bool {
    if !visiting.insert(l) {
        return false;
    }
    let ty = f.locals.get(l).map(|x| x.ty.0.clone()).unwrap_or_default();
    let base = ty.trim_end_matches('*').to_string();
    let mut r = false;
    if let Some(defs) = 定义.get(&fi).and_then(|m| m.get(&l)) {
        if !defs.is_empty() {
            r = defs.iter().all(|d| 值新鲜(prog, 定义, 新鲜fn, fi, d, visiting, f));
        }
    }
    if !r {
        if let Some(st) = prog.types.iter().find(|t| t.name == base) {
            let mut 见 = std::collections::BTreeSet::new();
            let mut 好 = true;
            for b in &f.blocks {
                for s in &b.stmts {
                    if let Stmt::Assign { lhs, rhs } = s {
                        if lhs.base == l && lhs.proj.len() == 1 {
                            if let Proj::Field(fn_) = &lhs.proj[0] {
                                if !值新鲜(prog, 定义, 新鲜fn, fi, rhs, visiting, f) {
                                    好 = false;
                                }
                                见.insert(fn_.clone());
                            }
                        }
                    }
                }
            }
            if 好 && 见.len() == st.fields.len() && !st.fields.is_empty() {
                r = true;
            }
        }
    }
    visiting.remove(&l);
    r
}

fn 计算归属字段(
    prog: &Program,
) -> (
    std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    std::collections::BTreeSet<String>,
) {
    use std::collections::{BTreeMap, BTreeSet};
    let mut 定义: BTreeMap<usize, BTreeMap<LocalId, Vec<Rvalue>>> = BTreeMap::new();
    for (fi, f) in prog.functions.iter().enumerate() {
        let mut m: BTreeMap<LocalId, Vec<Rvalue>> = BTreeMap::new();
        for b in &f.blocks {
            for st in &b.stmts {
                if let Some((p, r)) = st.赋值体() {
                    if p.proj.is_empty() {
                        m.entry(p.base).or_default().push(r.clone());
                    }
                }
            }
        }
        定义.insert(fi, m);
    }
    let mut 新鲜fn: BTreeSet<usize> = (0..prog.functions.len()).collect();
    loop {
        let mut 删: Vec<usize> = Vec::new();
        for (fi, f) in prog.functions.iter().enumerate() {
            if !新鲜fn.contains(&fi) {
                continue;
            }
            let Some(v) = f.blocks.iter().find_map(|b| match &b.exit {
                Exit::Return(Some(Value::Local(l))) => Some(*l),
                _ => None,
            }) else {
                删.push(fi);
                continue;
            };
            if !局部新鲜(prog, &定义, &新鲜fn, fi, v, &mut BTreeSet::new(), f) {
                删.push(fi);
            }
        }
        if 删.is_empty() {
            break;
        }
        for i in 删 {
            新鲜fn.remove(&i);
        }
    }
    let mut 统计: BTreeMap<(String, String), bool> = BTreeMap::new();
    for (fi, f) in prog.functions.iter().enumerate() {
        let mut 读: BTreeSet<LocalId> = BTreeSet::new();
        let mut 写数: BTreeMap<LocalId, usize> = BTreeMap::new();
        for b in &f.blocks {
            for st in &b.stmts {
                let mut s = BTreeSet::new();
                语句读(st, &mut s);
                读.extend(s);
                if let Some((p, _)) = st.赋值体() {
                    if p.proj.is_empty() {
                        *写数.entry(p.base).or_insert(0) += 1;
                    }
                }
            }
        }
        for b in &f.blocks {
            for st in &b.stmts {
                let Stmt::Assign { lhs, rhs } = st else { continue };
                if lhs.proj.len() != 1 {
                    continue;
                }
                let Proj::Field(fname) = &lhs.proj[0] else { continue };
                let Some(base) = f.locals.get(lhs.base) else { continue };
                let tname = base.ty.0.trim_end_matches('*').to_string();
                let Some(st_t) = prog.types.iter().find(|t| t.name == tname) else { continue };
                let Some((_, ft)) = st_t.fields.iter().find(|(n, _)| n == fname) else { continue };
                if !ft.0.ends_with('*') {
                    continue;
                }
                let 子型 = ft.0.trim_end_matches('*');
                if !prog.types.iter().any(|t| t.name == 子型) {
                    continue;
                }
                let 合格 = match rhs {
                    Rvalue::Const(Value::Local(c)) => {
                        写数.get(c).copied().unwrap_or(0) == 1
                            && 读.contains(c)
                            && 局部新鲜(prog, &定义, &新鲜fn, fi, *c, &mut BTreeSet::new(), f)
                    }
                    Rvalue::Const(Value::Null) => true,
                    _ => false,
                };
                let e = 统计.entry((tname, fname.clone())).or_insert(true);
                *e = *e && 合格;
            }
        }
    }
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for ((t, fld), ok) in 统计 {
        if ok {
            out.entry(t).or_default().insert(fld);
        }
    }
    let 新鲜名: BTreeSet<String> = 新鲜fn
        .iter()
        .filter_map(|i| prog.functions.get(*i).map(|f| f.name.clone()))
        .collect();
    (out, 新鲜名)
}

pub fn 对象挂名(prog: &mut Program, 圈尾回收开: bool) {
    if std::env::var("PRION_FLAT_DEBUG").is_ok() {
        eprintln!("平铺集 {:?}", prog.平铺);
    }
    use std::collections::{BTreeMap, BTreeSet};
    let (归属字段, 新鲜名) = 计算归属字段(prog);
    prog.owned_fields = 归属字段;
    let 归属表 = prog.owned_fields.clone();
    prog.objects.clear();
    
    let 观感 = prion_ir::形参观感(prog);
    
    let 真函数: BTreeSet<String> = prog
        .functions
        .iter()
        .map(|f| f.name.clone())
        .chain(prog.externs.iter().map(|e| e.name.clone()))
        .collect();
    let mut 表: BTreeMap<String, (BTreeMap<ObjId, LocalId>, BTreeMap<(usize, usize), Vec<(ObjId, LocalId)>>)> = BTreeMap::new();
    
    let mut 槽登记: BTreeMap<String, Ty> = BTreeMap::new();
    for f in prog.functions.iter() {
        for l in f.locals.iter() {
            if let Some(n) = &l.name {
                if n.contains('#') {
                    槽登记.entry(n.clone()).or_insert_with(|| l.ty.clone());
                }
            }
        }
    }
    for f in &mut prog.functions {
        let mut 绑: BTreeMap<LocalId, ObjId> = BTreeMap::new();
        let mut 待放: BTreeSet<ObjId> = BTreeSet::new();
        槽旧值释放(f);
        
        let mut 死: BTreeSet<LocalId> = BTreeSet::new();
        for b in &f.blocks {
            for st in &b.stmts {
                if let Stmt::Assign { lhs, rhs } = st {
                    if !lhs.proj.is_empty() {
                        continue;
                    }
                    if matches!(rhs, Rvalue::Const(Value::Int(0))) {
                        死.insert(lhs.base);
                        continue;
                    }
                    let 是容器 = f
                        .locals
                        .get(lhs.base)
                        .map(|l| is_container_ty(&l.ty.0))
                        .unwrap_or(false);
                    if !是容器 {
                        continue;
                    }
                    let 源 = match rhs {
                        Rvalue::Const(Value::Local(u)) => Some(*u),
                        Rvalue::Use(Place { base: u, .. }) => Some(*u),
                        Rvalue::Call { name, args }
                            if (name == "que" || name == "stack") && args.len() == 1 =>
                        {
                            match args[0] {
                                Value::Local(u) => Some(u),
                                _ => None,
                            }
                        }
                        _ => None,
                    };
                    if let Some(u) = 源 {
                        if f.locals.get(u).map(|l| l.name.is_none()).unwrap_or(false) {
                            死.insert(u);
                        }
                    }
                }
            }
        }
        
        loop {
            let mut 变 = false;
            for b in &f.blocks {
                for st in &b.stmts {
                    if let Stmt::Assign { lhs, rhs } = st {
                        if !lhs.proj.is_empty() || !死.contains(&lhs.base) {
                            continue;
                        }
                        let 源 = match rhs {
                            Rvalue::Const(Value::Local(u)) => Some(*u),
                            Rvalue::Use(Place { base: u, proj }) if proj.is_empty() => Some(*u),
                            _ => None,
                        };
                        if let Some(u) = 源 {
                            if f.locals.get(u).map(|l| l.name.is_none()).unwrap_or(false)
                                && 死.insert(u)
                            {
                                变 = true;
                            }
                        }
                    }
                }
            }
            if !变 {
                break;
            }
        }
        
        let 基id = f.locals.len();
        let mut 新locals: Vec<Local> = Vec::new();
        
        let mut 循环内: BTreeSet<usize> = BTreeSet::new();
        for b in &f.blocks {
            if let Exit::Branch { targets, .. } = &b.exit {
                for s in targets {
                    if s.target <= b.id {
                        for k in s.target..=b.id {
                            循环内.insert(k);
                        }
                    }
                }
            }
        }
        
        let mut 借obj: BTreeSet<ObjId> = BTreeSet::new();
        for b in &mut f.blocks {
            let mut 出: Vec<Stmt> = Vec::with_capacity(b.stmts.len() + 8);
            for st in std::mem::take(&mut b.stmts) {
                

                if let Stmt::Assign { lhs, rhs } = st.clone() {
                    let mut 语句 = st.clone();
                    if std::env::var("PRION_OWN_DEBUG").is_ok() {
                        eprintln!(
                            "assign L{} proj{} 块{} 堆={}",
                            lhs.base,
                            lhs.proj.len(),
                            b.id,
                            f.locals.get(lhs.base).map(|l| 是堆型(&l.ty.0)).unwrap_or(false)
                        );
                    }
                    if lhs.proj.is_empty() {
                        let 堆 = f.locals.get(lhs.base).map(|l| l.ty.clone()).filter(|t| 是堆型(&t.0));
                        if let Some(ty) = 堆 {
                            
                            let 置零 = matches!(rhs, Rvalue::Const(Value::Int(0)));
                            let mut 后放: Option<ObjId> = None;
                            if !置零 {
                                if let Some(&o_old) = 绑.get(&lhs.base) {
                                    let 活别名 = 绑
                                        .iter()
                                        .any(|(lid, o)| *o == o_old && *lid != lhs.base && !死.contains(lid));
                                    
                                    
                                    if 循环内.contains(&b.id) {
                                        
                                        let mut 读 = BTreeSet::new();
                                        语句读(&st, &mut 读);
                                        let 读自 = 读.contains(&lhs.base);
                                        if !活别名 && 待放.contains(&o_old) {
                                            待放.remove(&o_old);
                                            
                                            if ty.0 == "str" {
                                                if 读自 {
                                                    语句 = Stmt::AssignRelease {
                                                        lhs: lhs.clone(),
                                                        rhs: rhs.clone(),
                                                    };
                                                } else {
                                                    出.push(Stmt::DropVar { lid: lhs.base });
                                                }
                                            }
                                        }
                                    } else if !活别名 {
                                        
                                        let 旧id = 基id + 新locals.len();
                                        let 旧ty = f
                                            .locals
                                            .get(lhs.base)
                                            .map(|l| l.ty.clone())
                                            .unwrap_or_else(|| ty.clone());
                                        新locals.push(Local {
                                            name: None,
                                            ty: 旧ty,
                                            is_param: false,
                                            bare_ok: false,
                                            line: 0,
                                            col: 0,
                                            loop_local: false,
                                            fn_owned: false,
                                            obj: None,
                                        });
                                        绑.insert(旧id, o_old);
                                        出.push(Stmt::Assign {
                                            lhs: Place::local(旧id),
                                            rhs: Rvalue::Use(Place::local(lhs.base)),
                                        });
                                        出.push(Stmt::Bind { var: 旧id, obj: o_old });
                                        待放.remove(&o_old);
                                        后放 = Some(o_old);
                                    }
                                }
                            }
                            
                            if matches!(
                                rhs,
                                Rvalue::ListGet { .. }
                                    | Rvalue::MapGet { .. }
                                    | Rvalue::VariantGet { .. }
                                    | Rvalue::Ref(_)
                            ) {
                                
                                绑.remove(&lhs.base);
                                出.push(语句.clone());
                                if let Some(o) = 后放 {
                                    出.push(Stmt::Drop { obj: o });
                                }
                                continue;
                            }
                            if 置零 {
                                
                                绑.remove(&lhs.base);
                                出.push(语句.clone());
                                continue;
                            }
                            let 别名源: Option<LocalId> = match rhs {
                                Rvalue::Const(Value::Local(s)) => Some(s),
                                
                                Rvalue::Use(Place { base: s, .. }) => Some(s),
                                
                                Rvalue::Call { ref name, ref args }
                                    if (name == "que" || name == "stack") && args.len() == 1 =>
                                {
                                    match args[0] {
                                        Value::Local(s) => Some(s),
                                        _ => None,
                                    }
                                }
                                _ => None,
                            };
                            let 别名源 = if ty.0 == "str" { None } else { 别名源 };
                            match 别名源.and_then(|s| 绑.get(&s).copied()) {
                                Some(o) => {
                                    
                                    绑.insert(lhs.base, o);
                                    出.push(语句.clone());
                                    出.push(Stmt::Bind { var: lhs.base, obj: o });
                                    if let Some(o) = 后放 {
                                        出.push(Stmt::Drop { obj: o });
                                    }
                                    continue;
                                }
                                None => {
                                    
                                    if 别名源.is_some() {
                                        
                                        let 源值 = f.locals.get(别名源.unwrap());
                                        let 聚合构造 = ty.0.ends_with('*')
                                            && 源值.map(|s| !是堆型(&s.ty.0)).unwrap_or(false)
                                            && f.locals.get(lhs.base).map(|l| l.fn_owned).unwrap_or(false);
                                        if !聚合构造 {
                                            
                                            绑.remove(&lhs.base);
                                            出.push(语句.clone());
                                            if let Some(o) = 后放 {
                                                出.push(Stmt::Drop { obj: o });
                                            }
                                            continue;
                                        }
                                    }
                                    
                                    let obj = prog.objects.len() as ObjId;
                                    prog.objects.push(ObjEnt { class: StorageClass::Tmp, ty: ty.clone() });
                                    绑.insert(lhs.base, obj);
                                    if matches!(rhs, Rvalue::CtxGet { .. }) {
                                        借obj.insert(obj);
                                    }
                                    
                                    let 新构造调用 = matches!(rhs, Rvalue::Call { ref name, .. } if 新鲜名.contains(name));
                                    if !matches!(rhs, Rvalue::Call { .. }) || ty.0 == "str" || 新构造调用 {
                                        待放.insert(obj);
                                    }
                                    出.push(语句.clone());
                                    出.push(Stmt::Alloc { obj, class: StorageClass::Tmp, ty });
                                    出.push(Stmt::Bind { var: lhs.base, obj });
                                    if let Some(o) = 后放 {
                                        出.push(Stmt::Drop { obj: o });
                                    }
                                    continue;
                                }
                            }
                        }
                    }
                }
                出.push(st);
            }
            b.stmts = 出;
        }
        
        for b in f.blocks.iter() {
            for st in b.stmts.iter() {
                let Stmt::Assign { lhs, rhs } = st else { continue };
                if lhs.proj.len() != 1 {
                    continue;
                }
                let Proj::Field(fname) = &lhs.proj[0] else { continue };
                let 父名 = f
                    .locals
                    .get(lhs.base)
                    .map(|l| l.ty.0.trim_end_matches('*').to_string())
                    .unwrap_or_default();
                if !归属表.get(&父名).map(|s| s.contains(fname)).unwrap_or(false) {
                    continue;
                }
                let 子 = match rhs {
                    Rvalue::Const(Value::Local(c)) => Some(*c),
                    Rvalue::Use(p) if p.proj.is_empty() => Some(p.base),
                    _ => None,
                };
                if let Some(c) = 子 {
                    if let Some(&o) = 绑.get(&c) {
                        待放.remove(&o);
                    }
                }
            }
        }
        
        let mut 逃: BTreeSet<ObjId> = BTreeSet::new();
        let mut 返: BTreeSet<ObjId> = BTreeSet::new();
        let mut 捕获: BTreeSet<ObjId> = BTreeSet::new();
        逃.extend(借obj);
        
        let mut 借: BTreeMap<LocalId, LocalId> = BTreeMap::new();
        for b in f.blocks.iter() {
            for st in b.stmts.iter() {
                if let Some((lhs, rhs)) = st.赋值体() {
                    if !lhs.proj.is_empty() {
                        continue;
                    }
                    match rhs {
                        Rvalue::ListGet { list: Value::Local(容器), .. } => {
                            借.insert(lhs.base, *容器);
                        }
                        Rvalue::Use(Place { base: 源, .. }) | Rvalue::Const(Value::Local(源)) => {
                            if let Some(容器) = 借.get(源).copied() {
                                借.insert(lhs.base, 容器);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut 复印点: Vec<(usize, usize, ObjId, LocalId, ObjId)> = Vec::new();
        
        enum 源类 {
            借,
            移,
            构,
            链(usize),
        }
        let mut 定义: BTreeMap<LocalId, 源类> = BTreeMap::new();
        for b in f.blocks.iter() {
            for st in b.stmts.iter() {
                if let Some((lhs, rhs)) = st.赋值体() {
                    if !lhs.proj.is_empty() || 定义.contains_key(&lhs.base) {
                        continue;
                    }
                    let k = match rhs {
                        Rvalue::ListGet { .. } => 源类::借,
                        Rvalue::Call { name, .. }
                            if name != "粒子构造占位" && !真函数.contains(name) =>
                        {
                            
                            源类::构
                        }
                        Rvalue::Call { .. } => 源类::移,
                        Rvalue::Use(p) if p.proj.is_empty() => 源类::链(p.base),
                        _ => 源类::构,
                    };
                    定义.insert(lhs.base, k);
                }
            }
        }
        
        let mut 值推: BTreeMap<ObjId, (bool, bool)> = BTreeMap::new();
        for (bi, b) in f.blocks.iter().enumerate() {
            for (si, st) in b.stmts.iter().enumerate() {
                if let Stmt::Call { name, args } = st {
                    if name == "push" {
                        if let (Some(Value::Local(容器lid)), Some(Value::Local(元lid))) =
                            (args.get(0), args.get(1))
                        {
                            if let Some(&容器obj) = 绑.get(容器lid) {
                                
                                let mut 源 = *元lid;
                                let mut 尾 = None;
                                for _ in 0..64 {
                                    if let Some(k) = 定义.get(&源) {
                                        match k {
                                            源类::借 => {
                                                尾 = Some("借");
                                                break;
                                            }
                                            源类::移 => {
                                                尾 = Some("移");
                                                break;
                                            }
                                            源类::链(b) => {
                                                源 = *b;
                                            }
                                            源类::构 => {
                                                尾 = Some("构");
                                                break;
                                            }
                                        }
                                    } else {
                                        尾 = if f
                                            .locals
                                            .get(源)
                                            .map(|l| l.fn_owned)
                                            .unwrap_or(false)
                                        {
                                            Some("构")
                                        } else {
                                            Some("借")
                                        };
                                        break;
                                    }
                                }
                                let e = 值推.entry(容器obj).or_insert((false, false));
                                match 尾 {
                                    Some("构") => e.0 = true,
                                    _ => e.1 = true,
                                }
                            }
                        }
                        if let Some(Value::Local(lid)) = args.get(1) {
                            if f.locals.get(*lid).map(|l| l.fn_owned).unwrap_or(false) {
                                if let Some(&o) = 绑.get(lid) {
                                    
                                    let ety = f.locals.get(*lid).map(|l| l.ty.clone());
                                    if let Some(ety) = ety {
                                        let 副 = prog.objects.len() as ObjId;
                                        prog.objects.push(ObjEnt { class: StorageClass::Tmp, ty: ety });
                                        if let Some(Value::Local(容器lid)) = args.get(0) {
                                            if let Some(&容器obj) = 绑.get(容器lid) {
                                                prog.owned_elem.entry(容器obj).or_default().push(副);
                                            }
                                        }
                                        
                                        let 活别名 = 绑
                                            .iter()
                                            .any(|(l2, oo)| *oo == o && *l2 != *lid && !死.contains(l2));
                                        if !活别名 {
                                            
                                            if !循环内.contains(&bi) {
                                                复印点.push((bi, si, o, *lid, 副));
                                            }
                                            待放.remove(&o);
                                        }
                                        
                                        绑.insert(*lid, 副);
                                    }
                                }
                            } else if let Some(&o) = 绑.get(lid) {
                                
                                if !借.contains_key(lid) {
                                    if let Some(Value::Local(容器lid)) = args.get(0) {
                                        if let Some(&容器obj) = 绑.get(容器lid) {
                                            prog.owned_elem.entry(容器obj).or_default().push(o);
                                            待放.remove(&o);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                let mut 读: BTreeSet<LocalId> = BTreeSet::new();
                语句读(st, &mut 读);
                if let Stmt::Assign { rhs: Rvalue::CtxArray { args }, .. } = st {
                    for a in args {
                        if let Value::Local(lid) = a {
                            if f.locals.get(*lid).map(|l| 是堆型(&l.ty.0)).unwrap_or(false) {
                                if let Some(&o) = 绑.get(lid) {
                                    捕获.insert(o);
                                    逃.insert(o);
                                }
                            }
                        }
                    }
                }
                if let Stmt::Assign { rhs: Rvalue::Call { name, args }, .. } | Stmt::Call { name, args } = st {
                    if 真函数.contains(name) {
                        for (k, a) in args.iter().enumerate() {
                            
                            if 观感.get(&(name.clone(), k)).copied() == Some(false) {
                                continue;
                            }
                            if let Value::Local(lid) = a {
                                if f.locals.get(*lid).map(|l| is_container_ty(&l.ty.0)).unwrap_or(false) {
                                    if let Some(&o) = 绑.get(lid) {
                                        if 读.contains(lid) {
                                            逃.insert(o);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Exit::Return(Some(Value::Local(lid))) = &b.exit {
                
                if let Some(&容器) = 借.get(lid) {
                    let 是把手 = f
                        .locals
                        .get(*lid)
                        .map(|l| l.ty.0.ends_with('*'))
                        .unwrap_or(false);
                    if 是把手 {
                        if let Some(&容器o) = 绑.get(&容器) {
                            逃.insert(容器o);
                        }
                    }
                }
                if let Some(&o) = 绑.get(lid) {
                    返.insert(o);
                }
            }
        }
        
        复印点.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let mut 复印件: BTreeSet<ObjId> = BTreeSet::new();
        for (bi, si, o, lid, 副) in 复印点 {
            复印件.insert(o);
            let 旧id = 基id + 新locals.len();
            let 旧ty = f
                .locals
                .get(lid)
                .map(|l| l.ty.clone())
                .unwrap_or_else(|| Ty("int".into()));
            新locals.push(Local {
                name: None,
                ty: 旧ty,
                is_param: false,
                bare_ok: false,
                line: 0,
                col: 0,
                loop_local: false,
                fn_owned: false,
                obj: None,
            });
            绑.insert(旧id, o);
            if let Some(b) = f.blocks.get_mut(bi) {
                let at = si.min(b.stmts.len());
                b.stmts.insert(
                    at,
                    Stmt::Assign { lhs: Place::local(旧id), rhs: Rvalue::Use(Place::local(lid)) },
                );
                b.stmts.insert(at + 1, Stmt::Bind { var: 旧id, obj: o });
                let at2 = (at + 3).min(b.stmts.len());
                b.stmts.insert(at2, Stmt::Bind { var: lid, obj: 副 });
                b.stmts.insert(at2 + 1, Stmt::Drop { obj: o });
            }
        }
        
        f.locals.append(&mut 新locals);
        
        for (obj, (值, 把手)) in &值推 {
            if *值 && !*把手 {
                prog.值入列.insert(*obj);
            }
        }
        
        for (lid, obj) in &绑 {
            if let Some(l) = f.locals.get_mut(*lid) {
                l.obj = Some(*obj);
            }
        }
        
        
        let mut 对象位: BTreeMap<ObjId, (usize, usize)> = BTreeMap::new();
        let mut 局部末读: BTreeMap<LocalId, (usize, usize)> = BTreeMap::new();
        for (bi, b) in f.blocks.iter().enumerate() {
            for (si, st) in b.stmts.iter().enumerate() {
                let mut 用: BTreeSet<LocalId> = BTreeSet::new();
                语句读(st, &mut 用);
                for lid in 用 {
                    局部末读.insert(lid, (bi, si));
                    if let Some(&o) = 绑.get(&lid) {
                        对象位.insert(o, (bi, si));
                    }
                }
            }
        }
        
        {
            let mut 延: Vec<(ObjId, LocalId)> = Vec::new();
            for b in f.blocks.iter() {
                for st in b.stmts.iter() {
                    if let Some((lhs, rhs)) = st.赋值体() {
                        
                        let 源 = match rhs {
                            Rvalue::ListGet { list: Value::Local(源), .. } => Some(*源),
                            Rvalue::Ref(p) => Some(p.base),
                            _ => None,
                        };
                        if let Some(源) = 源 {
                            if let Some(&容器o) = 绑.get(&源) {
                                延.push((容器o, lhs.base));
                            }
                        }
                    }
                }
            }
            for (容器o, lid) in 延 {
                if let Some(&pos) = 局部末读.get(&lid) {
                    let e = 对象位.entry(容器o).or_insert(pos);
                    if pos > *e {
                        *e = pos;
                    }
                }
            }
        }
        
        let mut 活: BTreeMap<ObjId, Vec<LocalId>> = BTreeMap::new();
        for (lid, o) in &绑 {
            if !死.contains(lid) {
                活.entry(*o).or_default().push(*lid);
            }
        }
        for (o, lids) in 活.iter_mut() {
            if !对象位.contains_key(o) {
                
                lids.retain(|lid| f.locals.get(*lid).map(|l| l.name.is_some()).unwrap_or(false));
            }
        }
        let mut 就地: BTreeMap<(usize, usize), Vec<(ObjId, LocalId)>> = BTreeMap::new();
        let mut drop_map: BTreeMap<ObjId, LocalId> = BTreeMap::new();
        let 拥有: BTreeSet<ObjId> = prog.owned_elem.values().flatten().copied().collect();
        let mut 见过: BTreeSet<ObjId> = BTreeSet::new();
        for (lid, obj) in &绑 {
            if !见过.insert(*obj) {
                continue;
            }
            if 逃.contains(obj) || 返.contains(obj) || 拥有.contains(obj) {
                continue;
            }
            let Some(l) = f.locals.get(*lid) else { continue };
            
            let 可放 = is_container_ty(&l.ty.0)
                || l.ty.0 == "str"
                || l.ty
                    .0
                    .strip_suffix('*')
                    .map(|b| prog.types.iter().any(|t| t.name == b))
                    .unwrap_or(false);
            if l.is_param || !可放 {
                continue;
            }
            if !待放.contains(obj) {
                continue;
            }
            
            let 绑定 = 活.get(obj).map(|v| v.as_slice()).unwrap_or(&[]);
            if 绑定.len() != 1 {
                
                if !绑定.is_empty() && 待放.contains(obj) && !复印件.contains(obj) {
                    drop_map.entry(*obj).or_insert(绑定[0]);
                }
                continue;
            }
            let 活lid = 绑定[0];
            if !f.blocks.iter().any(|b| b.id > 0) {
                
                if f.blocks.len() == 1 {
                    if let Some(&pos) = 对象位.get(obj) {
                        就地.entry(pos).or_default().push((*obj, 活lid));
                        continue;
                    }
                }
            }
            
            drop_map.entry(*obj).or_insert(活lid);
        }
        
        if 圈尾回收开 {
            let (入, 出) = 活跃表(f);
            let mut 定义块: BTreeMap<LocalId, BTreeSet<usize>> = BTreeMap::new();
            for (bi, b) in f.blocks.iter().enumerate() {
                for (pid, _) in &b.params {
                    定义块.entry(*pid).or_default().insert(bi);
                }
                for st in &b.stmts {
                    if let Some((p, _)) = st.赋值体() {
                        if p.proj.is_empty() {
                            定义块.entry(p.base).or_default().insert(bi);
                        }
                    }
                }
            }
            let 循环们 = 循环表(f);
            for (头, 体, 锁) in 循环们 {
                if 体.iter().any(|&bi| matches!(f.blocks[bi].exit, Exit::Return(_))) {
                    continue;
                }
                let mut 删: Vec<ObjId> = Vec::new();
                for (obj, _) in drop_map.iter() {
                    if 逃.contains(obj) || 返.contains(obj) || 捕获.contains(obj) || 复印件.contains(obj) {
                        continue;
                    }
                    if 拥有.contains(obj) {
                        continue;
                    }
                    let mut 好 = true;
                    let mut rep: Option<LocalId> = None;
                    for (l2, o2) in 绑.iter() {
                        if o2 != obj || 死.contains(l2) {
                            continue;
                        }
                        if rep.is_none() {
                            rep = Some(*l2);
                        }
                        let Some(l) = f.locals.get(*l2) else { 好 = false; break };
                        if l.is_param || l.name.as_deref().map(|n| n.contains('#')).unwrap_or(false) {
                            好 = false;
                            break;
                        }
                        match 定义块.get(l2) {
                            Some(s) if s.iter().all(|d| 体.contains(d)) => {}
                            _ => {
                                好 = false;
                                break;
                            }
                        }
                        if 入[头].contains(l2) {
                            好 = false;
                            break;
                        }
                        let 出界 = 体.iter().any(|&bi| {
                            let 有外 = match &f.blocks[bi].exit {
                                Exit::Branch { targets, .. } => {
                                    targets.iter().any(|t| !体.contains(&(t.target as usize)))
                                }
                                Exit::Return(_) | Exit::Unreachable => true,
                            };
                            有外 && 出[bi].contains(l2)
                        });
                        if 出界 {
                            好 = false;
                            break;
                        }
                    }
                    let Some(rep) = rep else { continue };
                    if !好 {
                        continue;
                    }
                    for &u in &锁 {
                        f.blocks[u].stmts.push(Stmt::DropVar { lid: rep });
                    }
                    删.push(*obj);
                }
                for o in 删 {
                    drop_map.remove(&o);
                }
            }
        }
        

        
        for b in f.blocks.iter_mut() {
            if matches!(b.exit, Exit::Return(_)) {
                for obj in &捕获 {
                    b.stmts.push(Stmt::Drop { obj: *obj });
                }
                for (_obj, lid) in drop_map.iter() {
                    b.stmts.push(Stmt::DropVar { lid: *lid });
                }
            }
        }
        
        
        let mut 插入点: Vec<((usize, usize), Vec<(ObjId, LocalId)>)> = 就地.clone().into_iter().collect();
        插入点.sort_by(|a, b| b.0.cmp(&a.0));
        for ((bi, si), 项) in 插入点 {
            if let Some(b) = f.blocks.get_mut(bi) {
                let mut at = (si + 1).min(b.stmts.len());
                for (_o, lid) in 项 {
                    b.stmts.insert(at, Stmt::DropVar { lid });
                    at += 1;
                }
            }
        }
        
        块单元释放(f);
    }
}

fn 是堆来源串深(r: &Rvalue, 定义: &BTreeMap<LocalId, Rvalue>, 深: usize) -> bool {
    if 深 > 8 {
        return false;
    }
    match r {
        Rvalue::BinOp { .. } => true,
        Rvalue::Call { name, .. } => prion_ir::返回自有串(name),
        Rvalue::Use(p) if p.proj.is_empty() => 定义
            .get(&p.base)
            .map(|d| 是堆来源串深(d, 定义, 深 + 1))
            .unwrap_or(false),
        Rvalue::Const(Value::Local(u)) => 定义
            .get(u)
            .map(|d| 是堆来源串深(d, 定义, 深 + 1))
            .unwrap_or(false),
        _ => false,
    }
}

fn 槽旧值释放(f: &mut Function) {
    let mut 槽参: BTreeSet<LocalId> = BTreeSet::new();
    for (i, l) in f.locals.iter().enumerate() {
        if l.is_param && l.ty.0 == "str*" {
            槽参.insert(i);
        }
    }
    if 槽参.is_empty() {
        return;
    }
    let mut 定义: BTreeMap<LocalId, Rvalue> = BTreeMap::new();
    for b in f.blocks.iter() {
        for st in b.stmts.iter() {
            if let Stmt::Assign { lhs, rhs } = st {
                if lhs.proj.is_empty() {
                    定义.insert(lhs.base, rhs.clone());
                }
            }
        }
    }
    for b in f.blocks.iter_mut() {
        for st in b.stmts.iter_mut() {
            if let Stmt::Assign { lhs, rhs } = st {
                if lhs.proj.len() == 1
                    && matches!(lhs.proj[0], Proj::Deref)
                    && 槽参.contains(&lhs.base)
                    && 是堆来源串深(rhs, &定义, 0)
                {
                    *st = Stmt::AssignRelease { lhs: lhs.clone(), rhs: rhs.clone() };
                }
            }
        }
    }
}

fn 块单元释放(f: &mut Function) {
    let mut 组: BTreeMap<String, Vec<LocalId>> = BTreeMap::new();
    for (i, l) in f.locals.iter().enumerate() {
        let Some(n) = &l.name else { continue };
        let Some((块, val)) = n.split_once('#') else { continue };
        let _ = val;
        if is_container_ty(&l.ty.0) || l.ty.0 == "str" {
            组.entry(块.trim_start_matches('|').to_string()).or_default().push(i);
        }
    }
    let mut 投放: Vec<(usize, usize, Vec<LocalId>)> = Vec::new();
    for (块名, lids) in 组 {
        let mut 位: Vec<(usize, usize)> = Vec::new();
        for (bi, b) in f.blocks.iter().enumerate() {
            for (si, st) in b.stmts.iter().enumerate() {
                let mut 读 = BTreeSet::new();
                语句读(st, &mut 读);
                let 关联 = lids.iter().any(|l| 读.contains(l))
                    || matches!(st, Stmt::Call { name, .. } if name == &块名);
                if 关联 {
                    位.push((bi, si));
                }
            }
        }
        let Some(&(bi, _)) = 位.first() else { continue };
        if 位.iter().any(|(b2, _)| *b2 != bi) {
            continue;
        }
        let si = 位.iter().map(|(_, s)| *s).max().unwrap_or(0);
        投放.push((bi, si, lids));
    }
    投放.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    for (bi, si, lids) in 投放 {
        let Some(b) = f.blocks.get_mut(bi) else { continue };
        let mut at = (si + 1).min(b.stmts.len());
        for lid in lids {
            b.stmts.insert(at, Stmt::DropVar { lid });
            at += 1;
        }
    }
}

fn 后继块(f: &Function, b: usize) -> Vec<usize> {
    match &f.blocks[b].exit {
        Exit::Branch { targets, .. } => targets.iter().map(|s| s.target as usize).collect(),
        _ => Vec::new(),
    }
}

fn 前驱块(f: &Function) -> Vec<Vec<usize>> {
    let mut ps = vec![Vec::new(); f.blocks.len()];
    for i in 0..f.blocks.len() {
        for t in 后继块(f, i) {
            if t < ps.len() {
                ps[t].push(i);
            }
        }
    }
    ps
}

fn 链深(idom: &[Option<usize>], mut b: usize) -> usize {
    let mut d = 0;
    while let Some(p) = idom[b] {
        d += 1;
        b = p;
        if d > 100000 {
            break;
        }
    }
    d
}

fn 求交(idom: &[Option<usize>], mut a: usize, mut b: usize) -> usize {
    let mut 圈 = 0;
    while a != b {
        let da = 链深(idom, a);
        let db = 链深(idom, b);
        if da >= db {
            match idom[a] {
                Some(p) => a = p,
                None => return a,
            }
        } else {
            match idom[b] {
                Some(p) => b = p,
                None => return b,
            }
        }
        圈 += 1;
        if 圈 > 100000 {
            break;
        }
    }
    a
}

fn 支配树(f: &Function) -> Vec<Option<usize>> {
    let n = f.blocks.len();
    let mut idom: Vec<Option<usize>> = vec![None; n];
    if n == 0 {
        return idom;
    }
    let ps = 前驱块(f);
    let mut 变 = true;
    let mut 轮 = 0;
    while 变 && 轮 < 500 {
        变 = false;
        轮 += 1;
        for b in 1..n {
            let mut 新: Option<usize> = None;
            for &p in &ps[b] {
                if idom[p].is_none() && p != 0 {
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

fn 支配(idom: &[Option<usize>], a: usize, b: usize) -> bool {
    if a == b {
        return true;
    }
    let mut 走 = b;
    let mut 圈 = 0;
    while let Some(p) = idom[走] {
        if p == a {
            return true;
        }
        走 = p;
        圈 += 1;
        if 圈 > 100000 {
            break;
        }
    }
    false
}

fn 循环表(f: &Function) -> Vec<(usize, BTreeSet<usize>, Vec<usize>)> {
    let idom = 支配树(f);
    let ps = 前驱块(f);
    let mut 表: BTreeMap<usize, (BTreeSet<usize>, Vec<usize>)> = BTreeMap::new();
    for u in 0..f.blocks.len() {
        for v in 后继块(f, u) {
            if !支配(&idom, v, u) {
                continue;
            }
            let 项 = 表.entry(v).or_insert_with(|| {
                let mut s = BTreeSet::new();
                s.insert(v);
                (s, Vec::new())
            });
            项.1.push(u);
            let mut 栈 = vec![u];
            while let Some(x) = 栈.pop() {
                if 项.0.insert(x) {
                    for &p in &ps[x] {
                        栈.push(p);
                    }
                }
            }
        }
    }
    let mut out: Vec<(usize, BTreeSet<usize>, Vec<usize>)> =
        表.into_iter().map(|(h, (b, l))| (h, b, l)).collect();
    out.sort_by_key(|(_, b, _)| b.len());
    out
}

fn 块用定(b: &prion_ir::Block) -> (BTreeSet<LocalId>, BTreeSet<LocalId>) {
    let mut 用: BTreeSet<LocalId> = BTreeSet::new();
    let mut 定: BTreeSet<LocalId> = BTreeSet::new();
    for (pid, _) in &b.params {
        定.insert(*pid);
    }
    for st in &b.stmts {
        if let Stmt::DropVar { lid } = st {
            if !定.contains(lid) {
                用.insert(*lid);
            }
        }
        let mut r: BTreeSet<LocalId> = BTreeSet::new();
        语句读(st, &mut r);
        for x in r {
            if !定.contains(&x) {
                用.insert(x);
            }
        }
        if let Some((p, _)) = st.赋值体() {
            if p.proj.is_empty() {
                定.insert(p.base);
            }
        }
    }
    (用, 定)
}

fn 活跃表(f: &Function) -> (Vec<BTreeSet<LocalId>>, Vec<BTreeSet<LocalId>>) {
    let n = f.blocks.len();
    let mut 入: Vec<BTreeSet<LocalId>> = vec![BTreeSet::new(); n];
    let mut 出: Vec<BTreeSet<LocalId>> = vec![BTreeSet::new(); n];
    let ud: Vec<(BTreeSet<LocalId>, BTreeSet<LocalId>)> =
        f.blocks.iter().map(块用定).collect();
    let mut 变 = true;
    let mut 轮 = 0;
    while 变 && 轮 < 1000 {
        变 = false;
        轮 += 1;
        for b in (0..n).rev() {
            let mut o: BTreeSet<LocalId> = BTreeSet::new();
            for t in 后继块(f, b) {
                if t < n {
                    o.extend(入[t].iter().copied());
                }
            }
            let mut i = ud[b].0.clone();
            for x in o.iter() {
                if !ud[b].1.contains(x) {
                    i.insert(*x);
                }
            }
            if o != 出[b] || i != 入[b] {
                出[b] = o;
                入[b] = i;
                变 = true;
            }
        }
    }
    (入, 出)
}
