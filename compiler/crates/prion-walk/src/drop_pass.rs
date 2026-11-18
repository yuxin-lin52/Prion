

use prion_ir::{is_container_ty, Exit, Function, Local, LocalId, ObjId, ObjEnt, Place, Program, Rvalue, Stmt, StorageClass, Ty, Value};
use std::collections::{BTreeMap, BTreeSet};

fn 是堆型(t: &str) -> bool {
    t == "str" || t.ends_with('*') || is_container_ty(t)
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
            Rvalue::VariantLit { args, .. } => args.iter().for_each(|a| 值用(a, out)),
            Rvalue::VariantTag(v) => 值用(v, out),
            Rvalue::VariantGet { value, .. } => 值用(value, out),
            Rvalue::FnRef { .. } => {}
        }
    }
    match s {
        Stmt::Assign { lhs, rhs } => {
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
        | Stmt::Drop { .. } | Stmt::DropVar { .. } | Stmt::Kill { .. } => {}
    }
}



pub fn 对象挂名(prog: &mut Program) {
    if std::env::var("PRION_FLAT_DEBUG").is_ok() {
        eprintln!("平铺集 {:?}", prog.平铺);
    }
    use std::collections::{BTreeMap, BTreeSet};
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
        
        let mut 杀obj: BTreeMap<LocalId, ObjId> = BTreeMap::new();
        for b in &mut f.blocks {
            let mut 出: Vec<Stmt> = Vec::with_capacity(b.stmts.len() + 8);
            for st in std::mem::take(&mut b.stmts) {
                
                if let Stmt::Kill { block } = &st {
                    for (lid, l) in f.locals.iter().enumerate() {
                        let 属目标 = l
                            .name
                            .as_deref()
                            .and_then(|n| n.rsplit_once('#').map(|x| x.0))
                            .and_then(|bf| bf.rsplit('|').next())
                            == Some(block.as_str());
                        if !属目标 {
                            continue;
                        }
                        
                        let 可放 = is_container_ty(&l.ty.0)
                            || l.ty.0 == "str"
                            || l.ty
                                .0
                                .strip_suffix('*')
                                .map(|b2| prog.types.iter().any(|t| t.name == b2))
                                .unwrap_or(false);
                        if !可放 {
                            continue;
                        }
                        let o = *杀obj.entry(lid).or_insert_with(|| {
                            let o = prog.objects.len() as ObjId;
                            prog.objects.push(ObjEnt { class: StorageClass::Tmp, ty: l.ty.clone() });
                            o
                        });
                        绑.insert(lid, o);
                        出.push(Stmt::Drop { obj: o });
                    }
                    continue;
                }
                if let Stmt::Assign { lhs, rhs } = &st {
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
                                        if !活别名 {
                                            待放.remove(&o_old);
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
                                出.push(st.clone());
                                if let Some(o) = 后放 {
                                    出.push(Stmt::Drop { obj: o });
                                }
                                continue;
                            }
                            if 置零 {
                                
                                绑.remove(&lhs.base);
                                出.push(st.clone());
                                continue;
                            }
                            let 别名源: Option<LocalId> = match rhs {
                                Rvalue::Const(Value::Local(s)) => Some(*s),
                                
                                Rvalue::Use(Place { base: s, .. }) => Some(*s),
                                _ => None,
                            };
                            
                            let 别名源 = if ty.0 == "str" { None } else { 别名源 };
                            match 别名源.and_then(|s| 绑.get(&s).copied()) {
                                Some(o) => {
                                    
                                    绑.insert(lhs.base, o);
                                    出.push(st.clone());
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
                                            出.push(st.clone());
                                            if let Some(o) = 后放 {
                                                出.push(Stmt::Drop { obj: o });
                                            }
                                            continue;
                                        }
                                    }
                                    
                                    let obj = prog.objects.len() as ObjId;
                                    prog.objects.push(ObjEnt { class: StorageClass::Tmp, ty: ty.clone() });
                                    绑.insert(lhs.base, obj);
                                    
                                    if !matches!(rhs, Rvalue::Call { .. }) || ty.0 == "str" {
                                        待放.insert(obj);
                                    }
                                    出.push(st.clone());
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
        
        let mut 逃: BTreeSet<ObjId> = BTreeSet::new();
        let mut 返: BTreeSet<ObjId> = BTreeSet::new();
        
        let mut 借: BTreeMap<LocalId, LocalId> = BTreeMap::new();
        for b in f.blocks.iter() {
            for st in b.stmts.iter() {
                if let Stmt::Assign { lhs, rhs } = st {
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
                if let Stmt::Assign { lhs, rhs } = st {
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
                                
                                if !待放.contains(&o) && !借.contains_key(lid) {
                                    if let Some(Value::Local(容器lid)) = args.get(0) {
                                        if let Some(&容器obj) = 绑.get(容器lid) {
                                            prog.owned_elem.entry(容器obj).or_default().push(o);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                let mut 读: BTreeSet<LocalId> = BTreeSet::new();
                语句读(st, &mut 读);
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
        for (bi, si, o, lid, 副) in 复印点 {
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
                    if let Stmt::Assign { lhs, rhs: Rvalue::ListGet { list: Value::Local(源), .. } } = st {
                        if let Some(&容器o) = 绑.get(源) {
                            延.push((容器o, lhs.base));
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
        let mut 见过: BTreeSet<ObjId> = BTreeSet::new();
        for (lid, obj) in &绑 {
            if !见过.insert(*obj) {
                continue;
            }
            if 逃.contains(obj) || 返.contains(obj) {
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
        
        if f.is_kill && f.is_block {
            let 前缀 = format!("{}#", f.name);
            let 己槽: Vec<(String, Ty)> = 槽登记
                .iter()
                .filter(|(k, _)| k.starts_with(&前缀))
                .map(|(k, t)| (k.clone(), t.clone()))
                .collect();
            for (key, ty) in 己槽 {
                
                let 可放 = is_container_ty(&ty.0)
                    || ty.0 == "str"
                    || ty
                        .0
                        .strip_suffix('*')
                        .map(|b2| prog.types.iter().any(|t| t.name == b2))
                        .unwrap_or(false);
                if !可放 {
                    continue;
                }
                let lid = match f.locals.iter().position(|l| l.name.as_deref() == Some(key.as_str())) {
                    Some(i) => i,
                    None => {
                        f.locals.push(Local {
                            name: Some(key),
                            ty: ty.clone(),
                            is_param: false,
                            bare_ok: false,
                            line: 0,
                            col: 0,
                            loop_local: false,
                            fn_owned: false,
                            obj: None,
                        });
                        f.locals.len() - 1
                    }
                };
                let o = prog.objects.len() as ObjId;
                prog.objects.push(ObjEnt { class: StorageClass::Tmp, ty });
                f.locals[lid].obj = Some(o);
                drop_map.insert(o, lid);
            }
        }
        
        for b in f.blocks.iter_mut() {
            if matches!(b.exit, Exit::Return(_)) {
                for obj in drop_map.keys() {
                    b.stmts.push(Stmt::Drop { obj: *obj });
                }
            }
        }
        
        
        let mut 插入点: Vec<((usize, usize), Vec<(ObjId, LocalId)>)> = 就地.clone().into_iter().collect();
        插入点.sort_by(|a, b| b.0.cmp(&a.0));
        for ((bi, si), 项) in 插入点 {
            if let Some(b) = f.blocks.get_mut(bi) {
                let mut at = (si + 1).min(b.stmts.len());
                for (o, _lid) in 项 {
                    b.stmts.insert(at, Stmt::Drop { obj: o });
                    at += 1;
                }
            }
        }
    }
}