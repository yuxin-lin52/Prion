




use crate::{字段型, Ctx};
use prion_parser::expr::{BinOp as EBinOp, Expr, ExprKind, Target, UnOp, Verb};


thread_local! {
    static TYPECK: std::cell::RefCell<std::collections::BTreeMap<u32, String>> =
        std::cell::RefCell::new(std::collections::BTreeMap::new());
}


pub fn 记(id: u32, ty: &str) {
    TYPECK.with(|m| m.borrow_mut().insert(id, ty.to_string()));
}


pub fn typeck清空() {
    TYPECK.with(|m| m.borrow_mut().clear());
}


pub fn typeck快照() -> std::collections::BTreeMap<u32, String> {
    TYPECK.with(|m| m.borrow().clone())
}


pub fn 推断(e: &Expr, cx: &Ctx, block: &str) -> Option<String> {
    let r = 推断节点(e, cx, block);
    if let Some(t) = &r {
        记(e.id(), t);
    }
    r
}


fn 推断节点(e: &Expr, cx: &Ctx, block: &str) -> Option<String> {
    match &e.kind {
        ExprKind::Int(_) => Some("int".into()),
        ExprKind::Float(_) => Some("float".into()),
        ExprKind::Str(_) => Some("str".into()),
        ExprKind::Bool(_) => Some("bool".into()),
        ExprKind::Null => None,

        ExprKind::Name { target, name } => match target {
            Target::Val => 查值(cx, block, name),
            
            Target::Fn => 查函数(cx, block, name).map(|(r, ps)| format!("fn({}) {}", ps.join(", "), r)),
            _ => None,
        },

        ExprKind::Call { verb, target, block: b, name, args, .. } => {
            推断调用(*verb, *target, b.as_deref(), name, args, cx, block)
        }

        ExprKind::Index { base, .. } => 推断(base, cx, block).and_then(|t| 元素型(&t)),
        ExprKind::Slice { base, .. } => 推断(base, cx, block),
        ExprKind::Field { base, name } => {
            推断(base, cx, block).and_then(|t| 字段型(&cx.types, &t, name))
        }
        ExprKind::Deref(inner) => 推断(inner, cx, block).and_then(|t| Some(t.trim_end_matches('*').to_string())),

        ExprKind::Unary { op, operand } => match op {
            UnOp::Not => Some("bool".into()),
            _ => 推断(operand, cx, block),
        },
        ExprKind::Binary { op, lhs, rhs } => match op {
            EBinOp::Eq | EBinOp::Ne | EBinOp::Lt | EBinOp::Le | EBinOp::Gt | EBinOp::Ge
            | EBinOp::And | EBinOp::Or => Some("bool".into()),
            _ => 推断(lhs, cx, block).or_else(|| 推断(rhs, cx, block)),
        },
        ExprKind::Ternary { then, otherwise, .. } => {
            推断(then, cx, block).or_else(|| 推断(otherwise, cx, block))
        }

        ExprKind::ListLit(xs) => {
            
            let 首 = xs.first()?;
            let t = 推断(首, cx, block)?;
            Some(format!("list({})", t))
        }
        ExprKind::MapLit(ps) => {
            
            let (k, v) = ps.first()?;
            let kt = 推断(k, cx, block)?;
            let vt = 推断(v, cx, block)?;
            Some(format!("map({}, {})", kt, vt))
        }
    }
}


pub fn 推断带期望(e: &Expr, cx: &Ctx, block: &str, 期望: Option<&str>) -> Option<String> {
    match &e.kind {
        
        ExprKind::ListLit(xs) if xs.is_empty() => 期望.map(|s| s.to_string()),
        ExprKind::MapLit(ps) if ps.is_empty() => 期望.map(|s| s.to_string()),
        _ => 推断(e, cx, block),
    }
}


fn 查值(cx: &Ctx, block: &str, name: &str) -> Option<String> {
    let key = cx.键(block, name)?;
    cx.local_idx.get(&key).map(|&i| cx.locals[i].ty.clone())
}


fn 查函数(cx: &Ctx, block: &str, name: &str) -> Option<(String, Vec<String>)> {
    let key = cx.键(block, name)?;
    cx.fns.get(&key).map(|s| (s.ret.clone(), s.params.clone()))
}


fn 推断调用(
    verb: Verb,
    target: Target,
    blk: Option<&str>,
    name: &str,
    args: &[Expr],
    cx: &Ctx,
    block: &str,
) -> Option<String> {
    let 用块 = blk.unwrap_or(block);
    match (verb, target) {
        
        (Verb::Call, Target::Val) if args.is_empty() => 查值(cx, 用块, name),
        
        (Verb::Call, Target::Val) => 查值(cx, 用块, name).and_then(|t| 函数返回(&t)),
        
        (Verb::Call, Target::Fn) => 查函数(cx, 用块, name).map(|(r, _)| r),
        
        (Verb::Delta, Target::Val) => 查值(cx, 用块, name),
        
        (_, Target::Type) => {
            if cx.types.contains_key(name) {
                return Some(format!("{}*", name));
            }
            
            if let Some(t) = cx.变体所属.get(name) {
                return Some(t.clone());
            }
            if name == "que" || name == "stack" {
                
                let t = args.first().and_then(|a| 推断(a, cx, block)).unwrap_or_else(|| "int".into());
                let t = 元素型(&t).unwrap_or(t);
                Some(format!("{}({})", name, t))
            } else if name == "list" {
                let t = args.first().and_then(|a| 推断(a, cx, block)).unwrap_or_else(|| "int".into());
                Some(format!("list({})", t))
            } else if name == "map" {
                Some("map(str, int)".into())
            } else {
                None
            }
        }
        
        (_, Target::Builtin) => 内置返回(name, args, cx, block),
        _ => None,
    }
}


fn 函数返回(t: &str) -> Option<String> {
    let inner = t.strip_prefix("fn(")?;
    let mut 深 = 0i32;
    for (i, c) in inner.char_indices() {
        match c {
            '(' => 深 += 1,
            ')' => {
                深 -= 1;
                if 深 < 0 {
                    return Some(inner[i + 1..].trim().to_string());
                }
            }
            _ => {}
        }
    }
    None
}


fn 元素型(t: &str) -> Option<String> {
    let inner = t.strip_prefix("list(").or_else(|| t.strip_prefix("que(")).or_else(|| t.strip_prefix("stack("))?;
    let inner = inner.strip_suffix(')')?;
    Some(inner.trim().to_string())
}


pub fn 对比(ast: &prion_parser::Program, cx: &Ctx) {
    
    if std::env::var("PRION_INFER_DIFF").is_err() {
        return;
    }
    for b in &ast.blocks {
        走(&b.stmts, cx, &b.name);
    }
}


fn 走(stmts: &[prion_parser::Stmt], cx: &Ctx, block: &str) {
    use prion_parser::Stmt as S;
    for s in stmts {
        match s {
            S::Line { line, exprs, kind, .. } => {
                let Some(e) = exprs.first() else { continue };
                let 新 = 推断(e, cx, block);
                
                let t: &[prion_lexer::Token] = kind.tokens();
                let mut 深 = 0i32;
                let mut eq = None;
                for (k, tk) in t.iter().enumerate() {
                    match tk {
                        prion_lexer::Token::Sym("(") | prion_lexer::Token::Sym("[")
                        | prion_lexer::Token::Sym("{") => 深 += 1,
                        prion_lexer::Token::Sym(")") | prion_lexer::Token::Sym("]")
                        | prion_lexer::Token::Sym("}") => 深 -= 1,
                        prion_lexer::Token::Sym("=") if 深 == 0 => {
                            eq = Some(k);
                            break;
                        }
                        _ => {}
                    }
                }
                let 起 = eq.map(|x| x + 1).unwrap_or(0);
                if 起 >= t.len() {
                    continue;
                }
                
                if 新.is_none() {
                    let 旧 = crate::infer(&t[起..], &cx.locals, &cx.local_idx, &cx.names, block, &cx.types);
                    eprintln!("[推不出] 第{}行 旧={:?} AST={}", line, 旧, e);
                }
            }
            S::Struct { body, .. } => 走(body, cx, block),
            
            S::Fn { head, body, .. } => {
                let n = head
                    .iter()
                    .skip(1)
                    .find_map(|t| match t {
                        prion_lexer::Token::Str(n) | prion_lexer::Token::Ident(n) => Some(n.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let inner = if n.is_empty() { block.to_string() } else { format!("{}#{}", block, n) };
                走(body, cx, &inner);
            }
        }
    }
}


fn 内置返回(name: &str, args: &[Expr], cx: &Ctx, block: &str) -> Option<String> {
    match name {
        "len" | "len_utf8" | "index" | "char_code" | "int" | "time_now" => Some("int".into()),
        "has" | "empty" | "contains" => Some("bool".into()),
        "float" | "floor" | "ceil" | "round" => Some("float".into()),
        "str" | "str_f" | "concat" | "sub_str" | "nth_char" | "char" | "read_file" | "read_line"
        | "str_from_c" => Some("str".into()),
        "sort" | "push" | "put" | "insert" | "clear" | "sleep" | "exit" => None,
        "pop" => args.first().and_then(|a| 推断(a, cx, block)).and_then(|t| 元素型(&t)),
        "keys" => Some("list(str)".into()),
        "vals" => args.first().and_then(|a| 推断(a, cx, block)).and_then(|t| {
            let inner = t.strip_prefix("map(")?.strip_suffix(')')?;
            let mut 深 = 0i32;
            for (i, c) in inner.char_indices() {
                match c {
                    '(' => 深 += 1,
                    ')' => 深 -= 1,
                    ',' if 深 == 0 => return Some(format!("list({})", inner[i + 1..].trim())),
                    _ => {}
                }
            }
            None
        }),
        _ => None,
    }
}
