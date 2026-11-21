










use prion_error::Span;
use prion_lexer::Token;
use prion_parser::{Block as AstBlock, Expr, Program as AstProgram, Stmt as AstStmt, TokStream};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use prion_ir::{
    AsmForm, BinOp, Block, BlockId, ExternFn, Exit, Function, Local, LocalId, MapKey, Place, Proj,
    Program, Rvalue, Stmt, StructType, Succ, Ty, Value, Variant,
};


#[derive(Debug, Clone, PartialEq)]
pub struct BuildError {
    pub message: String,
    pub code: prion_error::Code,
    
    pub line: usize,
    
    
    pub file: String,
}

thread_local! {
    
    static CUR_LINE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    
    static GENERICS: std::cell::RefCell<BTreeMap<String, Vec<String>>> =
        std::cell::RefCell::new(BTreeMap::new());
    
    static INST_TODO: std::cell::RefCell<Vec<(String, Vec<String>)>> =
        std::cell::RefCell::new(Vec::new());
    
    #[allow(non_upper_case_globals)]
    static 常量fn表: std::cell::RefCell<BTreeMap<String, (Vec<String>, Vec<Token>)>> =
        std::cell::RefCell::new(BTreeMap::new());
    
    #[allow(non_upper_case_globals)]
    static 泛型fn表: std::cell::RefCell<
        BTreeMap<String, (Vec<(String, Vec<String>)>, Vec<(String, Ty)>, Ty, Vec<Token>, String, String)>,
    > =
        std::cell::RefCell::new(BTreeMap::new());
    
    #[allow(non_upper_case_globals)]
    static 泛型语句体fn表: std::cell::RefCell<
        BTreeMap<String, (Vec<(String, Vec<String>)>, Vec<(String, Ty)>, Ty, Vec<AstStmt>, String, String)>,
    > =
        std::cell::RefCell::new(BTreeMap::new());
    
    #[allow(non_upper_case_globals)]
    static fn_used表: std::cell::RefCell<BTreeMap<String, BTreeMap<String, Ty>>> =
        std::cell::RefCell::new(BTreeMap::new());
    
    #[allow(non_upper_case_globals)]
    static 类型代换表: std::cell::RefCell<Option<(Vec<String>, Vec<String>)>> =
        const { std::cell::RefCell::new(None) };
}


fn 常量名显示(段: &[Token]) -> String {
    match 段.get(2) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
        _ => "?".to_string(),
    }
}


fn 类型能力(型: &str, op: &str) -> bool {
    let 算术 = ["+", "-", "*", "/", "%"];
    let 比较 = ["<", "<=", ">", ">="];
    let 相等 = ["==", "!="];
    match 型 {
        "int" | "float" | "int_8" | "int_16" | "int_32" | "int_64" | "uint_8" | "uint_16"
        | "uint_32" | "uint_64" => {
            算术.contains(&op) || 比较.contains(&op) || 相等.contains(&op)
        }
        "str" => 比较.contains(&op) || 相等.contains(&op),
        "bool" => 相等.contains(&op),
        
        _ => 相等.contains(&op),
    }
}


enum 复印 {
    无需,
    拷贝(String, i64),
    无法,
}


fn 区域单(t: &str, 区域: &BTreeSet<String>) -> bool {
    if t.ends_with('*') || t == "str" {
        return true;
    }
    let n = t.split('(').next().unwrap_or(t);
    if matches!(n, "list" | "que" | "stack" | "map") {
        return true;
    }
    区域.contains(t)
}


fn 语句写(s: &Stmt, out: &mut BTreeSet<LocalId>) {
    match s {
        Stmt::Assign { lhs, .. } => {
            out.insert(lhs.base);
        }
        Stmt::Asm(f) => f.outs.iter().for_each(|(_, id, _)| {
            out.insert(*id);
        }),
        _ => {}
    }
}


fn 跨圈(f: &Function, dst: LocalId, h: usize, c: usize) -> bool {
    for b in f.blocks.iter() {
        if b.id >= h && b.id <= c {
            continue;
        }
        let mut 读 = BTreeSet::new();
        let mut 写 = BTreeSet::new();
        let mut 写过 = false;
        for s in b.stmts.iter() {
            读.clear();
            写.clear();
            语句读(s, &mut 读);
            语句写(s, &mut 写);
            if 读.contains(&dst) && !写过 {
                return true;
            }
            if 写.contains(&dst) {
                写过 = true;
            }
        }
        读.clear();
        match &b.exit {
            Exit::Branch { cond, .. } => 值用(cond, &mut 读),
            Exit::Return(Some(v)) => 值用(v, &mut 读),
            _ => {}
        }
        if 读.contains(&dst) && !写过 {
            return true;
        }
    }
    false
}


fn 区型表(types: &[StructType]) -> BTreeSet<String> {
    let mut 集: BTreeSet<String> = BTreeSet::new();
    loop {
        let mut 加 = false;
        for t in types {
            if 集.contains(&t.name) {
                continue;
            }
            let 有 = t.fields.iter().any(|(_, ty)| 区域单(&ty.0, &集))
                || t.variants.iter().any(|v| v.fields.iter().any(|(_, ty)| 区域单(&ty.0, &集)));
            if 有 {
                集.insert(t.name.clone());
                加 = true;
            }
        }
        if !加 {
            return 集;
        }
    }
}


fn 可深拷(型: &str, types: &[StructType]) -> bool {
    let 标 = |t: &str| {
        matches!(
            t,
            "int" | "float" | "bool" | "str" | "int_8" | "int_16" | "int_32" | "int_64"
                | "uint_8" | "uint_16" | "uint_32" | "uint_64"
        )
    };
    if 标(型) {
        return true;
    }
    if 型.ends_with('*') {
        return false; 
    }
    let 内 = |p: &str| 型.strip_prefix(p).and_then(|r| r.strip_suffix(')')).map(|x| x.trim().to_string());
    
    let 叶 = |t: &str| 标(t) || t == "str";
    for 前 in ["list(", "que(", "stack("] {
        if let Some(e) = 内(前) {
            return 叶(&e);
        }
    }
    if let Some(kv) = 内("map(") {
        let v = kv.rsplit(',').next().unwrap_or("").trim();
        return 叶(v);
    }
    let Some(st) = types.iter().find(|t| t.name == 型) else {
        return false;
    };
    st.variants.is_empty() && st.fields.iter().all(|(_, ty)| 可深拷(&ty.0, types))
}


fn 复印方式(型: &str, 区域: &BTreeSet<String>, types: &[StructType]) -> 复印 {
    let 标量 = |t: &str| {
        matches!(
            t,
            "int" | "float" | "bool" | "int_8" | "int_16" | "int_32" | "int_64" | "uint_8"
                | "uint_16" | "uint_32" | "uint_64"
        )
    };
    if 标量(型) || !区域单(型, 区域) {
        return 复印::无需;
    }
    if 型 == "str" {
        return 复印::拷贝("#str_copy".into(), 0);
    }
    let 内 = |p: &str| -> Option<String> {
        型.strip_prefix(p)?.strip_suffix(')').map(|s| s.trim().to_string())
    };
    for 前 in ["list(", "que(", "stack("] {
        if let Some(e) = 内(前) {
            if e == "str" {
                return 复印::拷贝("#dc_list".into(), 1);
            }
            if 标量(&e) {
                return 复印::拷贝("#dc_list".into(), 0);
            }
            return 复印::无法;
        }
    }
    if let Some(kv) = 内("map(") {
        let v = kv.rsplit(',').next().unwrap_or("").trim();
        if v == "str" {
            return 复印::拷贝("#dc_map".into(), 1);
        }
        if 标量(v) {
            return 复印::拷贝("#dc_map".into(), 0);
        }
        return 复印::无法;
    }
    
    if 可深拷(型, types) {
        return 复印::拷贝(format!("#dcdeep_{}", 型), 0);
    }
    复印::无法
}


fn 值用(v: &Value, out: &mut BTreeSet<LocalId>) {
    if let Value::Local(id) = v {
        out.insert(*id);
    }
}


fn 位用(p: &Place, out: &mut BTreeSet<LocalId>) {
    out.insert(p.base);
}


fn 右用(r: &Rvalue, out: &mut BTreeSet<LocalId>) {
    match r {
        Rvalue::Use(p) | Rvalue::Ref(p) => 位用(p, out),
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
        Rvalue::CtxArray { args } => args.iter().for_each(|a| 值用(a, out)),
        Rvalue::CtxGet { ctx, .. } => {
            out.insert(*ctx);
        }
        Rvalue::VariantTag(v) => 值用(v, out),
        Rvalue::VariantGet { value, .. } => 值用(value, out),
        Rvalue::FnRef { .. } => {}
    }
}


fn 语句读(s: &Stmt, out: &mut BTreeSet<LocalId>) {
    match s {
        Stmt::Assign { rhs, .. } | Stmt::AssignRelease { rhs, .. } => 右用(rhs, out),
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
        | Stmt::Alloc { .. } | Stmt::Bind { .. } | Stmt::Drop { .. }
        | Stmt::DropVar { .. }
 => {}
    }
}


fn 临时直通(f: &mut Function) {
    let mut 读次: BTreeMap<LocalId, usize> = BTreeMap::new();
    for b in f.blocks.iter() {
        for s in b.stmts.iter() {
            let mut 用 = BTreeSet::new();
            语句读(s, &mut 用);
            for l in 用 {
                *读次.entry(l).or_insert(0) += 1;
            }
        }
        if let Exit::Return(Some(Value::Local(l))) = &b.exit {
            *读次.entry(*l).or_insert(0) += 1;
        }
    }
    for b in f.blocks.iter_mut() {
        let 句数 = b.stmts.len();
        let mut 并: BTreeMap<usize, Option<Stmt>> = BTreeMap::new();
        let mut i = 0;
        while i + 1 < 句数 {
            if 并.contains_key(&i) {
                i += 1;
                continue;
            }
            let (tmp, 右) = match &b.stmts[i] {
                Stmt::Assign { lhs, rhs }
                    if lhs.proj.is_empty()
                        && matches!(rhs, Rvalue::Call { .. } | Rvalue::BinOp { .. }) =>
                {
                    (lhs.base, rhs.clone())
                }
                _ => {
                    i += 1;
                    continue;
                }
            };
            let 合 = match f.locals.get(tmp) {
                Some(l) => l.name.is_none() && l.ty.0 == "str",
                None => false,
            };
            if !合 || 读次.get(&tmp).copied().unwrap_or(0) != 1 {
                i += 1;
                continue;
            }
            let dst = match &b.stmts[i + 1] {
                Stmt::Assign { lhs, rhs } if lhs.proj.is_empty() => match rhs {
                    Rvalue::Use(p) if p.proj.is_empty() && p.base == tmp => lhs.base,
                    Rvalue::Const(Value::Local(u)) if *u == tmp => lhs.base,
                    _ => {
                        i += 1;
                        continue;
                    }
                },
                _ => {
                    i += 1;
                    continue;
                }
            };
            if dst == tmp {
                i += 1;
                continue;
            }
            并.insert(i, Some(Stmt::Assign { lhs: Place::local(dst), rhs: 右 }));
            并.insert(i + 1, None);
            i += 2;
        }
        if 并.is_empty() {
            continue;
        }
        let mut 留: Vec<Stmt> = Vec::with_capacity(句数);
        let mut 标: Vec<Span> = Vec::with_capacity(句数);
        for (i, s) in b.stmts.iter().enumerate() {
            let sp = b.spans.get(i).copied().unwrap_or_default();
            match 并.get(&i) {
                Some(None) => {}
                Some(Some(x)) => {
                    留.push(x.clone());
                    标.push(sp);
                }
                None => {
                    留.push(s.clone());
                    标.push(sp);
                }
            }
        }
        b.stmts = 留;
        b.spans = 标;
    }
}


fn 核能力(
    名: &str,
    带能力: &[(String, Vec<String>)],
    实参类型: &[String],
) -> Result<(), BuildError> {
    for ((参数, 能力), 实) in 带能力.iter().zip(实参类型.iter()) {
        for op in 能力 {
            if !类型能力(实, op) {
                return Err(BuildError {
                    message: format!(
                        "generic fn \"{}\" has type parameter \"{}\" declaring capability \"{}\", but argument type \"{}\" does not support it",
                        名, 参数, op, 实
                    ),
                    code: prion_error::Code::E201,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                });
            }
        }
    }
    Ok(())
}


fn 编译期执行(
    名: &str,
    实参: &[crate::consts::Const],
    程序常量: &BTreeMap<String, crate::consts::Const>,
    深: usize,
) -> Option<crate::consts::Const> {
    if !crate::quota::可入(深) {
        return None; 
    }
    let (形参, 体) = 常量fn表.with(|t| t.borrow().get(名).cloned())?;
    if 形参.len() != 实参.len() {
        return None;
    }
    let mut 环: BTreeMap<String, crate::consts::Const> = 程序常量.clone();
    for (k, v) in 形参.iter().zip(实参.iter()) {
        环.insert(k.clone(), v.clone());
    }
    crate::consts::折叠带环境(
        &体,
        &mut |n: &str| 环.get(n).cloned(),
        &mut |f, a| 编译期执行(f, a, 程序常量, 深 + 1),
    )
}


fn 读类型参数表(段: &[Token], 开: usize) -> Option<Vec<(String, Vec<String>)>> {
    let 止 = 括号止(段, 开)?;
    let mut 出 = Vec::new();
    for 段 in 段[开 + 1..止].split(|t| *t == Token::Sym(",")) {
        
        match 段 {
            [Token::Ident(n)] => 出.push((n.to_string(), Vec::new())),
            [Token::Ident(n), Token::Sym(op)] => 出.push((n.to_string(), vec![(*op).to_string()])),
            _ => return None,
        }
    }
    if 出.is_empty() {
        None
    } else {
        Some(出)
    }
}


fn 括号止(段: &[Token], 开: usize) -> Option<usize> {
    let mut 深 = 0i32;
    for (k, t) in 段.iter().enumerate().skip(开) {
        match t {
            Token::Sym("(") => 深 += 1,
            Token::Sym(")") => {
                深 -= 1;
                if 深 == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}


fn 泛型判别(名: &str) -> bool {
    GENERICS.with(|g| g.borrow().contains_key(名))
}


fn 取实参段(t: &[Token], 左: usize) -> Vec<Vec<Token>> {
    let mut 深 = 0i32;
    let mut 内 = Vec::new();
    for tk in t.iter().skip(左) {
        match tk {
            Token::Sym("(") => {
                深 += 1;
                if 深 == 1 {
                    continue;
                }
            }
            Token::Sym(")") => {
                深 -= 1;
                if 深 == 0 {
                    break;
                }
            }
            _ => {}
        }
        内.push(tk.clone());
    }
    split_args(&内)
}


fn 类型串转token(s: &str) -> Vec<Token> {
    let 基 = s.trim_end_matches('*');
    let mut 出 = vec![Token::Ident(基.to_string().into())];
    for _ in 0..(s.len() - 基.len()) {
        出.push(Token::Sym("*"));
    }
    出
}


fn 替换类型参数token(段: &[Token], 参数: &[String], 实参: &[String]) -> Vec<Token> {
    段.iter()
        .map(|t| match t {
            Token::Ident(n) => match 参数.iter().position(|p| p == n.as_ref()) {
                Some(k) => Token::Ident(实参[k].clone().into()),
                None => Token::Ident(n.clone()),
            },
            other => {
                
                let _ = other;
                t.clone()
            }
        })
        .collect()
}


fn 建fn实例(
    bx: &mut Builder,
    实例名: &str,
    类型参数: &[String],
    实参类型: &[String],
    形参: &[(String, Ty)],
    返回: &Ty,
    体: &[Token],
) -> Result<(), BuildError> {
    let _ = bx;
    let mut 行: Vec<Token> = vec![
        Token::Kw("add"),
        Token::Kw("fn"),
        Token::Str(实例名.to_string().into()),
        Token::Sym("("),
    ];
    for (k, (名, 型)) in 形参.iter().enumerate() {
        if k > 0 {
            行.push(Token::Sym(","));
        }
        行.push(Token::Str(名.clone().into()));
        let 型串 = 替换参数(&型.0, 类型参数, 实参类型);
        行.extend(类型串转token(&型串));
    }
    行.push(Token::Sym(")"));
    let 返串 = 替换参数(&返回.0, 类型参数, 实参类型);
    行.extend(类型串转token(&返串));
    行.push(Token::Sym("="));
    行.extend(替换类型参数token(体, 类型参数, 实参类型));
    let 行流 = prion_parser::TokStream::无位(行.clone());
    let 行种 = prion_parser::line_kind(&行流);
    bx.line(&行种, &行, &[])
}


fn 拆类型(s: &str) -> (String, Vec<String>) {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("fn(") {
        let mut 深 = 1i32;
        let mut 闭 = None;
        for (k, c) in rest.char_indices() {
            match c {
                '(' => 深 += 1,
                ')' => {
                    深 -= 1;
                    if 深 == 0 {
                        闭 = Some(k);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(闭) = 闭 {
            let mut 实参 = 切类型实参(&rest[..闭]);
            let 返 = rest[闭 + 1..].trim();
            if !返.is_empty() {
                实参.push(返.to_string());
            }
            return ("fn".to_string(), 实参);
        }
    }
    match s.find('(') {
        None => (s.to_string(), Vec::new()),
        Some(k) => {
            let 名 = s[..k].trim().to_string();
            let 内 = s[k + 1..].trim_end_matches(')');
            let 实参 = 切类型实参(内);
            (名, 实参)
        }
    }
}


fn 切类型实参(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut 深 = 0i32;
    for c in s.chars() {
        match c {
            '(' => {
                深 += 1;
                cur.push(c);
            }
            ')' => {
                深 -= 1;
                cur.push(c);
            }
            ',' if 深 == 0 => {
                let t = cur.trim().to_string();
                if !t.is_empty() {
                    out.push(t);
                }
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    let t = cur.trim().to_string();
    if !t.is_empty() {
        out.push(t);
    }
    out
}


fn 统一类型(pat: &str, got: &str, 参数: &[String], 绑定: &mut [Option<String>]) -> bool {
    let (p, g) = (pat.trim(), got.trim());
    if p.ends_with('*') && g.ends_with('*') {
        return 统一类型(&p[..p.len() - 1], &g[..g.len() - 1], 参数, 绑定);
    }
    let (pn, pa) = 拆类型(p);
    if pa.is_empty() {
        if let Some(i) = 参数.iter().position(|x| *x == pn) {
            return match &绑定[i] {
                Some(x) => x == g,
                None => {
                    绑定[i] = Some(g.to_string());
                    true
                }
            };
        }
    }
    let (gn, ga) = 拆类型(g);
    if pn != gn || pa.len() != ga.len() {
        return false;
    }
    pa.iter().zip(ga.iter()).all(|(a, b)| 统一类型(a, b, 参数, 绑定))
}


fn 替换参数(s: &str, 参数: &[String], 实参: &[String]) -> String {
    let mut 出 = String::new();
    let 字 = |c: char| c.is_alphanumeric() || c == '_';
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < cs.len() {
        if 字(cs[i]) {
            let 起 = i;
            while i < cs.len() && 字(cs[i]) {
                i += 1;
            }
            let 词: String = cs[起..i].iter().collect();
            match 参数.iter().position(|p| *p == 词) {
                Some(k) => 出.push_str(&实参[k]),
                None => 出.push_str(&词),
            }
            continue;
        }
        出.push(cs[i]);
        i += 1;
    }
    出
}




fn collect_ast_lines<'a>(stmts: &'a [AstStmt], out: &mut Vec<&'a TokStream>) {
    for s in stmts {
        match s {
            AstStmt::Line { kind, .. } => out.push(kind.tokens()),
            AstStmt::Struct { head, body, .. } | AstStmt::Fn { head, body, .. } => {
                out.push(head);
                collect_ast_lines(body, out);
            }
        }
    }
}


fn collect_ast_lines_ln<'a>(stmts: &'a [AstStmt], out: &mut Vec<(usize, &'a TokStream)>) {
    for s in stmts {
        match s {
            AstStmt::Line { line, kind, .. } => out.push((*line, kind.tokens())),
            AstStmt::Struct { line, head, body, .. } | AstStmt::Fn { line, head, body, .. } => {
                out.push((*line, head));
                collect_ast_lines_ln(body, out);
            }
        }
    }
}

fn err<T>(m: impl Into<String>) -> Result<T, BuildError> {
    Err(BuildError {
        message: m.into(),
        code: prion_error::Code::E309,
        line: CUR_LINE.with(|c| c.get()),
        file: String::new(),
    })
}


#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    If,
    Else,
    While,
    Match,
    For,
}




fn builtin_ret_ty(name: &str) -> Ty {
    match name {
        "concat" | "sub_str" | "nth_char" | "char" | "str" | "read_file" => Ty("str".into()),
        "float" => Ty("float".into()),
        
        "bool" => Ty("bool".into()),

        "run" => Ty("int".into()),
        "sqrt" => Ty("float".into()),
        "has" | "empty" => Ty("bool".into()),
        
        "que" => Ty("que".into()),
        "stack" => Ty("stack".into()),
        
        "args" | "keys" => Ty("list(str)".into()),
        "vals" => Ty("list(int)".into()),
        
        "int_8" | "int_16" | "int_32" | "int_64" | "uint_8" | "uint_16" | "uint_32" | "uint_64" => {
            Ty(name.to_string())
        }
        "str_from_c" => Ty("str".into()),
        _ => Ty("int".into()),
    }
}


fn hh_is_fn_form(t: &[Token], start: usize) -> bool {
    let mut j = start;
    loop {
        match t.get(j) {
            Some(Token::Ident(_)) | Some(Token::Str(_)) => {
                j += 1;
                if matches!(t.get(j), Some(Token::Sym("."))) {
                    j += 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    if !matches!(t.get(j), Some(Token::HashColon)) {
        return false;
    }
    matches!(t.get(j + 1), Some(Token::Ident(_)) | Some(Token::Kw(_)))
        && matches!(t.get(j + 2), Some(Token::Kw("fn")))
}


fn field_ty_in(
    types: &[StructType],
    idx: &std::collections::HashMap<String, usize>,
    owner: &str,
    field: &str,
) -> String {
    idx.get(owner)
        .and_then(|&i| types[i].fields.iter().find(|(n, _)| n == field))
        .map(|(_, ty)| ty.0.clone())
        .unwrap_or_else(|| "int".into())
}


fn elem_ty_of(ty: &str) -> Option<String> {
    for pre in ["list(", "que(", "stack("] {
        if let Some(inner) = ty.strip_prefix(pre).and_then(|s| s.strip_suffix(')')) {
            return Some(inner.to_string());
        }
    }
    if let Some(inner) = ty.strip_prefix("map(").and_then(|s| s.strip_suffix(')')) {
        
        let mut depth = 0i32;
        for (i, c) in inner.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => return Some(inner[i + 1..].trim().to_string()),
                _ => {}
            }
        }
    }
    None
}


fn lit_hint(decl: &Ty, after: &[Token]) -> Option<Ty> {
    match after.first() {
        Some(Token::Sym("[")) | Some(Token::Sym("{")) => Some(decl.clone()),
        
        Some(Token::Kw("que")) | Some(Token::Kw("stack")) => Some(decl.clone()),
        Some(Token::Ident(n)) if n == "que" || n == "stack" => Some(decl.clone()),
        _ => None,
    }
}


fn 序列元素类型(t: &str) -> Option<String> {
    let (head, inner) = t.split_once('(')?;
    let tail = inner.strip_suffix(')')?;
    match head {
        "list" | "que" | "stack" => Some(tail.trim().to_string()),
        _ => None,
    }
}


fn container_ty_at(段: &[Token], i: usize) -> Option<String> {
    container_ty_at_raw(段, i).map(|s| 代换类型(&s))
}


fn token容器载荷(t: &str) -> Option<String> {
    let inner = t.strip_prefix("list(")?.strip_suffix(')')?.trim();
    if inner == "token" {
        return Some("int".into());
    }
    let rest = inner.strip_prefix("token(")?;
    Some(rest.strip_suffix(')').unwrap_or("int").trim().to_string())
}

fn container_ty_at_raw(段: &[Token], i: usize) -> Option<String> {
    let name = match 段.get(i)? {
        Token::Kw(k) => (*k).to_string(),
        _ => return None,
    };
    if name != "list" && name != "que" && name != "stack" && name != "map" {
        return None;
    }
    if !matches!(段.get(i + 1), Some(Token::Sym("("))) {
        return Some(name); 
    }
    let mut depth = 0i32;
    let mut end = None;
    for (k, tk) in 段.iter().enumerate().skip(i + 1) {
        match tk {
            Token::Sym("(") => depth += 1,
            Token::Sym(")") => {
                depth -= 1;
                if depth == 0 {
                    end = Some(k);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let inner = &段[i + 2..end];
    let arg_name = |ts: &[Token]| -> Option<String> {
        if matches!(ts.first(), Some(Token::Kw("fn"))) && matches!(ts.get(1), Some(Token::Sym("("))) {
            return parse_type_ref(ts, 0).ok().map(|(t, _, _)| t.0);
        }
        let base = match ts.first()? {
            Token::Kw(t) => (*t).to_string(),
            Token::Ident(n) | Token::Str(n) => n.to_string(),
            _ => return None,
        };
        if matches!(ts.get(1), Some(Token::Sym("*"))) {
            Some(format!("{}*", base))
        } else {
            Some(base)
        }
    };
    if name == "map" {
        let args = split_args(inner);
        if args.len() != 2 {
            return None;
        }
        let k = arg_name(&args[0])?;
        let v = arg_name(&args[1])?;
        return Some(format!("map({}, {})", k, v));
    }
    let e = arg_name(inner)?;
    Some(format!("{}({})", name, e))
}



#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FnSig {
    params: Vec<Ty>,
    ret: Ty,
    
    is_outs: bool,
    
    hidden: Vec<(String, Ty)>,
}


fn fn_ret_of(s: &str) -> Option<String> {
    let rest = s.strip_prefix("fn(")?;
    let mut 深 = 1i32;
    for (k, c) in rest.char_indices() {
        match c {
            '(' => 深 += 1,
            ')' => {
                深 -= 1;
                if 深 == 0 {
                    let ret = rest[k + 1..].trim();
                    return if ret.is_empty() { None } else { Some(ret.to_string()) };
                }
            }
            _ => {}
        }
    }
    None
}

struct Builder {
    f: Function,
    
    block: String,
    
    module: String,
    names: BTreeMap<String, LocalId>,
    
    fns: BTreeMap<String, FnSig>,
    
    global_fns: std::rc::Rc<BTreeMap<String, FnSig>>,
    
    
    cross_vals: std::rc::Rc<BTreeMap<String, BTreeMap<String, Ty>>>,
    
    exposed: BTreeMap<String, LocalId>,
    
    lit_ty: Option<Ty>,
    
    cur_line: usize,
    cur_col: usize,
    cur_len: usize,
    
    循环深: usize,
    
    

    pub_blocks: std::rc::Rc<BTreeSet<String>>,
    
    hidden_items: std::rc::Rc<BTreeSet<String>>,
    
    block_types: std::rc::Rc<BTreeMap<String, BTreeSet<String>>>,
    sum_variants: std::rc::Rc<BTreeMap<String, (String, usize, Vec<Ty>)>>,
    
    
    期望类型: Option<String>,
    
    常量值: std::rc::Rc<RefCell<BTreeMap<String, crate::consts::Const>>>,
    
    串字面量: BTreeMap<LocalId, String>,
    
    语句体返回: bool,
    
    types: std::rc::Rc<Vec<StructType>>,
    
    type_idx: std::rc::Rc<std::collections::HashMap<String, usize>>,
    
    mod_by_tail: std::rc::Rc<BTreeMap<String, Vec<String>>>,
    
    extra: Vec<Function>,
    
    共享序: Vec<(String, LocalId, Ty)>,
    
    任务上下文: Option<(LocalId, Vec<(String, Ty)>)>,
    
    externs: Vec<ExternFn>,
    blocks: Vec<Block>,
    cur: usize,
    next_id: BlockId,
}


fn 解析泛型语句头(
    head: &[Token],
) -> Option<(String, Vec<(String, Vec<String>)>, Vec<(String, Ty)>, Ty)> {
    let h: Vec<Token> = head.iter().cloned().filter(|t| *t != Token::Sym(":")).collect();
    let is_rebind = matches!(h.first(), Some(Token::Kw("fn")));
    let is_hide_fn = matches!(h.first(), Some(Token::Kw("add")))
        && matches!(h.get(1), Some(Token::Kw("hide")))
        && matches!(h.get(2), Some(Token::Kw("fn")));
    let name_at = if is_rebind { 1 } else if is_hide_fn { 3 } else { 2 };
    let name = match h.get(name_at) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
        _ => return None,
    };
    if !matches!(h.get(name_at + 1), Some(Token::Sym("("))) {
        return None;
    }
    let 类型参数 = 读类型参数表(&h, name_at + 1)?;
    let 止 = 括号止(&h, name_at + 1)? + 1;
    let (形参, 帕止) = parse_params(&h[止..]).ok()?;
    let (ret, _, _) = parse_type_ref(&h[止..], 帕止).ok()?;
    Some((name.to_string(), 类型参数, 形参, ret))
}

impl Builder {
    
    fn mod_has(&self, given: &str) -> bool {
        self.mod_by_tail.values().any(|v| v.iter().any(|m| m == given))
    }


    
    fn resolve_module(&self, given: &str) -> Result<Option<String>, BuildError> {
        if self.mod_by_tail.values().any(|v| v.iter().any(|m| m == given)) {
            return Ok(Some(given.to_string()));
        }
        let cands = match self.mod_by_tail.get(given) {
            Some(c) => c,
            None => return Ok(None),
        };
        if cands.len() == 1 {
            return Ok(Some(cands[0].clone()));
        }
        Err(BuildError {
            message: format!(
                "module \"{}\" is ambiguous - {} all use this name; write the full path (e.g. `##:{}`)",
                given,
                cands.join("、"),
                cands[0]
            ),
            code: prion_error::Code::E304,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })
    }


    fn check_pub(&self, blk_mod: &str, blk: &str) -> Result<(), BuildError> {
        if blk_mod == self.module {
            return Ok(());
        }
        if self.pub_blocks.contains(&format!("{}|{}", blk_mod, blk)) {
            return Ok(());
        }
        Err(BuildError {
            message: format!(
                "block \"{}\" (module {}) is private - only pub blocks can be called across modules",
                blk, blk_mod
            ),
            code: prion_error::Code::E201,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })
    }

    #[allow(clippy::too_many_arguments)] 
    fn new(
        name: String,
        file: String,
        is_block: bool,
        is_pub: bool,
        is_sle: bool,
        is_abi: bool,
        is_naked: bool,
        asm_lines: Vec<String>,
        block: String,
        module: String,
        types: std::rc::Rc<Vec<StructType>>,
        type_idx: std::rc::Rc<std::collections::HashMap<String, usize>>,
        mod_by_tail: std::rc::Rc<BTreeMap<String, Vec<String>>>,
        global_fns: std::rc::Rc<BTreeMap<String, FnSig>>,
        cross_vals: std::rc::Rc<BTreeMap<String, BTreeMap<String, Ty>>>,
        sum_variants: std::rc::Rc<BTreeMap<String, (String, usize, Vec<Ty>)>>,
        pub_blocks: std::rc::Rc<BTreeSet<String>>,
        hidden_items: std::rc::Rc<BTreeSet<String>>,
        block_types: std::rc::Rc<BTreeMap<String, BTreeSet<String>>>,
        常量值: std::rc::Rc<RefCell<BTreeMap<String, crate::consts::Const>>>,
    ) -> Self {
        Self {
            f: Function {
                name,
                file,
                ret: None,
                locals: Vec::new(),
                blocks: Vec::new(),
                is_block,
                is_pub,
                is_sle,
                is_abi,
                is_naked,
                asm_lines,
            },
            block,
            module,
            names: BTreeMap::new(),
            fns: BTreeMap::new(),
            global_fns,
            cross_vals,
            exposed: BTreeMap::new(),
            lit_ty: None,
            期望类型: None,
            语句体返回: false,
            常量值,
            串字面量: BTreeMap::new(),
            cur_line: CUR_LINE.with(|c| c.get()),
            cur_col: 0,
            cur_len: 0,
            循环深: 0,
            sum_variants,
            mod_by_tail,
            pub_blocks,
            hidden_items,
            block_types,
            type_idx,
            types,
            extra: Vec::new(),
            共享序: Vec::new(),
            任务上下文: None,
            externs: Vec::new(),
            blocks: vec![Block {
                id: 0,
                params: Vec::new(),
                stmts: Vec::new(),
                spans: Vec::new(),
                exit: Exit::Unreachable,
                max: None,
            }],
            cur: 0,
            next_id: 1,
        }
    }

    
    fn nb(&mut self) -> BlockId {
        let id = self.next_id;
        self.next_id += 1;
        self.blocks.push(Block {
            id,
            params: Vec::new(),
            stmts: Vec::new(),
            spans: Vec::new(),
            exit: Exit::Unreachable,
            max: None,
        });
        id
    }

    fn switch_to(&mut self, id: BlockId) {
        self.cur = self.blocks.iter().position(|b| b.id == id).expect("block should already exist");
    }

    fn emit(&mut self, s: Stmt) {
        self.记录串字面量(&s);
        let c = self.cur;
        let sp = Span { line: self.cur_line, col: self.cur_col, len: self.cur_len };
        self.blocks[c].stmts.push(s);
        self.blocks[c].spans.push(sp);
    }

    
    fn 记录串字面量(&mut self, s: &Stmt) {
        let (lhs, rhs) = match s {
            Stmt::Assign { lhs, rhs } | Stmt::AssignRelease { lhs, rhs } => (lhs, rhs),
            _ => return,
        };
        if !lhs.proj.is_empty() {
            return;
        }
        let 源 = match rhs {
            Rvalue::Const(Value::Str(t)) => Some(t.clone()),
            Rvalue::Use(Place { base, proj }) if proj.is_empty() => self.串字面量.get(base).cloned(),
            _ => None,
        };
        match 源 {
            Some(t) => {
                self.串字面量.insert(lhs.base, t);
            }
            None => {
                self.串字面量.remove(&lhs.base);
            }
        }
    }

    
    fn 查串下标(&self, base: &Value, k: i64) -> Result<(), BuildError> {
        let Value::Local(id) = *base else { return Ok(()) };
        let Some(t) = self.串字面量.get(&id) else { return Ok(()) };
        if k < 1 || (k as usize) > t.len() {
            return Err(BuildError {
                message: format!("index {} is out of bounds - literal \"{}\" has only {} byte(s)", k, t, t.len()),
                code: prion_error::Code::E302,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            });
        }
        Ok(())
    }

    fn set_exit(&mut self, e: Exit) {
        self.blocks[self.cur].exit = e;
    }

    
    fn goto(&mut self, target: BlockId) {
        self.set_exit(Exit::Branch { cond: Value::Int(1), targets: vec![Succ::new(target)] });
    }

    fn new_local(&mut self, name: Option<String>, ty: Ty, is_param: bool, bare_ok: bool) -> LocalId {
        let (line, col) = (self.cur_line, self.cur_col);
        let loop_local = self.循环深 > 0;
        self.f.locals.push(Local { name, ty, is_param, bare_ok, line, col, loop_local, fn_owned: false, obj: None });
        self.f.locals.len() - 1
    }

    
    fn field_ty_of(&self, owner: &str, field: &str) -> String {
        field_ty_in(&self.types, &self.type_idx, owner, field)
    }

    
    
    fn 槽或建(&mut self, slot_key: &str) -> Option<LocalId> {
        if let Some(&id) = self.names.get(slot_key) {
            return Some(id);
        }
        let (blk_full, vn) = slot_key.rsplit_once('#')?;
        let vty = self.cross_vals.get(blk_full)?.get(vn)?.clone();
        let id = self.new_local(Some(slot_key.to_string()), vty, false, false);
        self.names.insert(slot_key.to_string(), id);
        Some(id)
    }

    fn temp(&mut self, ty: Ty) -> LocalId {
        self.new_local(None, ty, false, false)
    }

    
    fn ty_of_value(&self, v: Value) -> Ty {
        match v {
            Value::Local(id) => {
                self.f.locals.get(id).map(|l| l.ty.clone()).unwrap_or_else(|| Ty("int".into()))
            }
            Value::Float(_) => Ty("float".into()),
            Value::Str(_) => Ty("str".into()),
            Value::Bool(_) => Ty("bool".into()),
            Value::Null => Ty("null".into()),
            Value::Int(_) => Ty("int".into()),
        }
    }

    
    fn is_str_value(&self, v: &Value) -> bool {
        match v {
            Value::Str(_) => true,
            Value::Local(id) => self.f.locals.get(*id).map(|l| l.ty.0 == "str").unwrap_or(false),
            _ => false,
        }
    }

    
    
    
    fn field_chain(
        &mut self,
        mut cur: usize,
        mut cur_ty: String,
        t: &[Token],
        i: &mut usize,
    ) -> Result<Value, BuildError> {
        while matches!(t.get(*i), Some(Token::Sym("."))) {
            let field = match t.get(*i + 1) {
                Some(Token::Ident(f)) | Some(Token::Str(f)) => f.to_string(),
                
                Some(Token::Int(n)) if *n >= 1 => n.to_string(),
                _ => return err("field access must be followed by a field name"),
            };
            *i += 2;
            
            let owner0 = cur_ty.trim_end_matches('*').to_string();
            if !self.type_idx.contains_key(&owner0) {
                return Err(BuildError {
                    message: format!(
                        "{} has no fields - field access (.name) only applies to product types defined with add type; for sum types use match destructuring",
                        cur_ty
                    ),
                    code: prion_error::Code::E303,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                });
            }
            if cur_ty.ends_with('*') {
                let c = self.bin(BinOp::Ne, Value::Local(cur), Value::Null);
                self.emit(Stmt::Assert {
                    cond: c,
                    msg: "null reference - the handle is null, cannot read a field".into(),
                    fix: "check for null first (x != null) before reading a field, or catch the empty case with a case branch".into(),
                });
            }
            let owner = cur_ty.trim_end_matches('*').to_string();
            let ft = self.field_ty_of(&owner, &field);
            let tmp = self.temp(Ty(ft.clone()));
            self.emit(Stmt::Assign {
                lhs: Place::local(tmp),
                rhs: Rvalue::Use(Place { base: cur, proj: vec![Proj::Field(field.to_string())] }),
            });
            cur = tmp;
            cur_ty = ft;
        }
        Ok(Value::Local(cur))
    }

    
    fn is_float_value(&self, v: &Value) -> bool {
        match v {
            Value::Float(_) => true,
            Value::Local(id) => self.f.locals.get(*id).map(|l| l.ty.0 == "float").unwrap_or(false),
            _ => false,
        }
    }

    
    
    fn 槽实参(
        &mut self,
        sig: &FnSig,
        call_key: &str,
        _caller_is_fn: bool,
    ) -> Result<Vec<Value>, BuildError> {
        let mut out = Vec::new();
        for (vn, _ty) in &sig.hidden {
            let mut it = call_key.splitn(3, '|');
            let _m = it.next().unwrap_or("");
            let blk = it.next().unwrap_or("");
            if blk == self.block {
                
                match self.exposed.get(vn) {
                    Some(&pid) => out.push(Value::Local(pid)),
                    None => match self.names.get(vn) {
                        Some(&lid) => {
                            let tmp = self.temp(Ty(format!("{}*", _ty.0)));
                            self.emit(Stmt::Assign {
                                lhs: Place::local(tmp),
                                rhs: Rvalue::Ref(Place::local(lid)),
                            });
                            out.push(Value::Local(tmp));
                        }
                        None => {
                            return Err(BuildError {
                                message: format!("the fn must use this block's val \"{}\" - it is not defined in this block yet", vn),
                                code: prion_error::Code::E304,
                                line: CUR_LINE.with(|c| c.get()),
                                file: String::new(),
                            })
                        }
                    },
                }
            } else {
                match self.exposed.get(vn) {
                    Some(&pid) => out.push(Value::Local(pid)),
                    None => {
                        let slot_key = format!("{}|{}#{}", self.module, blk, vn);
                        match self.槽或建(&slot_key) {
                            Some(sid) => {
                                let tmp = self.temp(Ty(format!("{}*", _ty.0)));
                                self.emit(Stmt::Assign {
                                    lhs: Place::local(tmp),
                                    rhs: Rvalue::Ref(Place::local(sid)),
                                });
                                out.push(Value::Local(tmp));
                            }
                            None => {
                                return Err(BuildError {
                                    message: format!(
                                        "val \"{}\" of block \"{}\" does not exist - define it with add val first; for an sle block, call #:{} first too",
                                        blk, vn, blk
                                    ),
                                    code: prion_error::Code::E304,
                                    line: CUR_LINE.with(|c| c.get()),
                                    file: String::new(),
                                })
                            }
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    
    fn is_container_ty(t: &str) -> bool {
        matches!(t.split('(').next().unwrap_or("").trim(), "list" | "que" | "stack" | "map")
    }

    
    fn map_val_ty(ty: &str) -> Option<String> {
        let inner = ty.strip_prefix("map(")?.strip_suffix(')')?;
        let mut depth = 0i32;
        for (i, c) in inner.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => return Some(inner[i + 1..].trim().to_string()),
                _ => {}
            }
        }
        None
    }

    
    fn is_container_value(&self, v: &Value) -> bool {
        match v {
            Value::Local(id) => self
                .f
                .locals
                .get(*id)
                .map(|l| Self::is_container_ty(&l.ty.0))
                .unwrap_or(false),
            _ => false,
        }
    }

    
    fn is_bool_value(&self, v: &Value) -> bool {
        match v {
            Value::Bool(_) => true,
            Value::Local(id) => self.f.locals.get(*id).map(|l| l.ty.0 == "bool").unwrap_or(false),
            _ => false,
        }
    }

    
    fn 容器实参深拷(&mut self, vals: &mut Vec<Value>, sig: &FnSig) -> Vec<usize> {
        let mut dc_tmps: Vec<usize> = Vec::new();
        if sig.is_outs {
            return dc_tmps;
        }
        for (k, v) in vals.iter_mut().enumerate() {
            let Some(pty) = sig.params.get(k).cloned() else { break };
            let tn = pty.0.clone();
            let 是容器 = tn.starts_with("list") || tn.starts_with("que")
                || tn.starts_with("stack") || tn.starts_with("map");
            if !是容器 {
                continue;
            }
            let dcn = if tn.starts_with("map") { "#dc_map" } else { "#dc_list" };
            let str_flag = if elem_ty_of(&tn).as_deref() == Some("str") { 1 } else { 0 };
            let t = self.temp(pty);
            self.emit(Stmt::Assign {
                lhs: Place::local(t),
                rhs: Rvalue::Call {
                    name: dcn.to_string(),
                    args: vec![v.clone(), Value::Int(str_flag)],
                },
            });
            *v = Value::Local(t);
            dc_tmps.push(t);
        }
        dc_tmps
    }

    fn take_复制(&mut self, v: Value) -> Result<Value, BuildError> {
        let ty = self.ty_of_value(v.clone());
        let n = ty.0.clone();
        let 标量 = matches!(
            n.as_str(),
            "int" | "float" | "bool" | "null" | "int_8" | "int_16" | "int_32" | "int_64"
                | "uint_8" | "uint_16" | "uint_32" | "uint_64"
        );
        if 标量 || n.ends_with('*') {
            return Ok(v);
        }
        if n == "str" {
            let t = self.temp(ty);
            self.emit(Stmt::Assign {
                lhs: Place::local(t),
                rhs: Rvalue::Call { name: "#str_copy".into(), args: vec![v] },
            });
            return Ok(Value::Local(t));
        }
        let 内 = |p: &str| -> Option<String> {
            n.strip_prefix(p)?.strip_suffix(')').map(|s| s.trim().to_string())
        };
        let mut dcn: Option<(&'static str, i64)> = None;
        for 前 in ["list(", "que(", "stack("] {
            if let Some(e) = 内(前) {
                let 串元 = e == "str";
                let 可 = 串元 || matches!(
                    e.as_str(),
                    "int" | "float" | "bool" | "int_8" | "int_16" | "int_32" | "int_64"
                        | "uint_8" | "uint_16" | "uint_32" | "uint_64"
                );
                if 可 {
                    dcn = Some(("#dc_list", if 串元 { 1 } else { 0 }));
                }
                break;
            }
        }
        if dcn.is_none() {
            if let Some(kv) = 内("map(") {
                let vv = kv.rsplit(',').next().unwrap_or("").trim().to_string();
                let 串值 = vv == "str";
                let 可 = 串值 || matches!(
                    vv.as_str(),
                    "int" | "float" | "bool" | "int_8" | "int_16" | "int_32" | "int_64"
                        | "uint_8" | "uint_16" | "uint_32" | "uint_64"
                );
                if 可 {
                    dcn = Some(("#dc_map", if 串值 { 1 } else { 0 }));
                }
            }
        }
        if let Some((名, 旗)) = dcn {
            let t = self.temp(ty);
            self.emit(Stmt::Assign {
                lhs: Place::local(t),
                rhs: Rvalue::Call { name: 名.into(), args: vec![v, Value::Int(旗)] },
            });
            return Ok(Value::Local(t));
        }
        if 可深拷(&n, &self.types[..]) {
            let t = self.temp(ty);
            self.emit(Stmt::Assign {
                lhs: Place::local(t),
                rhs: Rvalue::Call { name: format!("#dcdeep_{}", n), args: vec![v] },
            });
            return Ok(Value::Local(t));
        }
        err(format!("take cannot copy this type \"{}\" yet", n))
    }

    fn bin(&mut self, op: BinOp, lhs: Value, rhs: Value) -> Value {
        if matches!(op, BinOp::Eq | BinOp::Ne) && (matches!(lhs, Value::Null) || matches!(rhs, Value::Null)) {
            let other = if matches!(lhs, Value::Null) { &rhs } else { &lhs };
            let ty = self.ty_of_value(other.clone());
            if ty.0.starts_with('?') && self.type_idx.contains_key(&ty.0) {
                let tag = self.temp(Ty("int".into()));
                self.emit(Stmt::Assign {
                    lhs: Place::local(tag),
                    rhs: Rvalue::VariantTag(other.clone()),
                });
                return self.bin(op, Value::Local(tag), Value::Int(0));
            }
        }
        if op == BinOp::Add && matches!(rhs, Value::Int(0)) {
            let t0 = self.ty_of_value(lhs.clone());
            let n = t0.0.as_str();
            let 容器 = n.starts_with("list(") || n.starts_with("que(")
                || n.starts_with("stack(") || n.starts_with("map(");
            if 容器 {
                let dcn = if n.starts_with("map(") { "#dc_map" } else { "#dc_list" };
                let 串元 = elem_ty_of(n).as_deref() == Some("str");
                let t = self.temp(t0.clone());
                self.emit(Stmt::Assign {
                    lhs: Place::local(t),
                    rhs: Rvalue::Call {
                        name: dcn.to_string(),
                        args: vec![lhs, Value::Int(if 串元 { 1 } else { 0 })],
                    },
                });
                return Value::Local(t);
            }
        }
        let ty = match op {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::And
            | BinOp::Or => Ty("bool".into()),
            
            BinOp::Add if self.is_str_value(&lhs) || self.is_str_value(&rhs) => Ty("str".into()),
            
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div
                if self.is_float_value(&lhs) || self.is_float_value(&rhs) =>
            {
                Ty("float".into())
            }
            _ => Ty("int".into()),
        };
        let t = self.temp(ty);
        self.emit(Stmt::Assign { lhs: Place::local(t), rhs: Rvalue::BinOp { op, lhs, rhs } });
        Value::Local(t)
    }

    fn finish(mut self) -> Result<Function, BuildError> {
        
        if matches!(self.blocks[self.cur].exit, Exit::Unreachable) {
            self.set_exit(Exit::Return(None));
        }
        self.f.blocks = self.blocks;
        Ok(self.f)
    }


    
    fn 条件常量(&self, head: &[Token]) -> Option<bool> {
        let 体: Vec<Token> = head.iter().filter(|t| **t != Token::Sym(":")).cloned().collect();
        let 条 = match 体.first() {
            Some(Token::Kw("if")) => &体[1..],
            _ => &体[..],
        };
        let mut 换: Vec<Token> = Vec::with_capacity(条.len());
        let mut k = 0usize;
        while k < 条.len() {
            if matches!(条.get(k), Some(Token::Kw("val"))) {
                if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 条.get(k + 1) {
                    if let Some(c) = self.常量值.borrow().get(n.as_ref()).cloned() {
                        换.push(c.as_token());
                        k += 2;
                        continue;
                    }
                }
            }
            换.push(条[k].clone());
            k += 1;
        }
        crate::consts::折叠条件(&换)
    }

    
    fn 反解实例(&self, 名: &str, args: &[Vec<Token>]) -> Result<String, BuildError> {
        let 模板 = self
            .types
            .get(self.type_idx[名])
            .cloned()
            .ok_or_else(|| BuildError {
                message: format!("type \"{}\" is not registered", 名),
                code: prion_error::Code::E304,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            })?;
        let mut 绑定: Vec<Option<String>> = vec![None; 模板.params.len()];
        for (k, a) in args.iter().enumerate() {
            if k >= 模板.fields.len() {
                break;
            }
            let got = match self.推断类型(a) {
                Some(g) => g,
                None => continue,
            };
            let pat = 模板.fields[k].1 .0.clone();
            if let Some(pi) = 模板.params.iter().position(|p| *p == pat) {
                绑定[pi] = Some(got);
            } else if let Some(pi) = 模板.params.iter().position(|p| pat == format!("{}*", p)) {
                if let Some(内) = got.strip_suffix('*') {
                    绑定[pi] = Some(内.to_string());
                }
            }
        }
        if 绑定.iter().any(|x| x.is_none()) {
            return Err(BuildError {
                message: format!(
                    "cannot infer type parameters of generic type \"{}\" - give each argument an explicit type (a literal or a typed val)",
                    名
                ),
                code: prion_error::Code::E201,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            });
        }
        let 实参: Vec<String> = 绑定.into_iter().map(|x| x.unwrap()).collect();
        let 实例名 = format!("{}({})", 名, 实参.join(", "));
        if !self.type_idx.contains_key(&实例名) {
            return Err(BuildError {
                message: format!(
                    "generic instance \"{}\" has not been generated - first write a typed declaration `add val \"x\" {}(...) = ...`",
                    实例名, 实例名
                ),
                code: prion_error::Code::E201,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            });
        }
        Ok(实例名)
    }

    
    fn 推断类型(&self, 段: &[Token]) -> Option<String> {
        match 段 {
            [Token::Int(_)] => Some("int".to_string()),
            [Token::Float(_)] => Some("float".to_string()),
            [Token::Str(_)] | [Token::RawStr(_)] => Some("str".to_string()),
            [Token::Kw("re")] | [Token::Kw("fa")] => Some("bool".to_string()),
            [Token::Kw("val"), Token::Str(n)] | [Token::Kw("val"), Token::Ident(n)]
            | [Token::Kw("call"), Token::Kw("val"), Token::Str(n)]
            | [Token::Kw("call"), Token::Kw("val"), Token::Ident(n)] => {
                let id = *self.names.get(n.as_ref())?;
                self.f.locals.get(id).map(|l| l.ty.0.clone())
            }
            [Token::Kw("call"), Token::Kw("fix"), Token::Str(n)]
            | [Token::Kw("call"), Token::Kw("fix"), Token::Ident(n)] => {
                self.常量值.borrow().get(n.as_ref()).map(|c| match c {
                    crate::consts::Const::Int(_) => "int".to_string(),
                    crate::consts::Const::Float(_) => "float".to_string(),
                    crate::consts::Const::Str(_) => "str".to_string(),
                    crate::consts::Const::Bool(_) => "bool".to_string(),
                })
            }
            [Token::Kw("fn"), Token::Str(n)] | [Token::Kw("fn"), Token::Ident(n)] => {
                let key = format!("{}|{}|{}", self.module, self.block, n);
                let sig = self.fns.get(&key).or_else(|| self.global_fns.get(&key))?;
                let ps: Vec<String> = sig.params.iter().map(|t| t.0.clone()).collect();
                Some(format!("fn({}) {}", ps.join(", "), sig.ret.0))
            }
            
            [Token::Kw("val"), Token::Str(n), Token::Sym("."), Token::Ident(f)]
            | [Token::Kw("val"), Token::Ident(n), Token::Sym("."), Token::Ident(f)]
            | [Token::Kw("call"), Token::Kw("val"), Token::Str(n), Token::Sym("."), Token::Ident(f)]
            | [Token::Kw("call"), Token::Kw("val"), Token::Ident(n), Token::Sym("."), Token::Ident(f)] => {
                let id = *self.names.get(n.as_ref())?;
                let 主 = self.f.locals.get(id)?.ty.0.clone();
                Some(self.field_ty_of(&主, f))
            }
            
            [Token::Ident(n), Token::Sym("."), Token::Ident(f)] => {
                let id = *self.names.get(n.as_ref())?;
                let 主 = self.f.locals.get(id)?.ty.0.clone();
                Some(self.field_ty_of(&主, f))
            }
            [Token::Ident(n)] => {
                let id = *self.names.get(n.as_ref())?;
                self.f.locals.get(id).map(|l| l.ty.0.clone())
            }
            _ => None,
        }
    }

    
    fn 建asm(&mut self, 内区: &[Token]) -> Result<AsmForm, BuildError> {
        let 段 = prion_lexer::asm::解析隔离区(内区).map_err(|e| BuildError {
            message: e,
            code: prion_error::Code::E201,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        let mut outs = Vec::new();
        for (约束, 名, 读写) in 段.outs {
            let id = *self.names.get(&名).ok_or_else(|| BuildError {
                message: format!("asm output binding name \"{}\" is not in this block - define it with add val first", 名),
                code: prion_error::Code::E304,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            })?;
            outs.push((约束, id, 读写));
        }
        let mut ins = Vec::new();
        for (约束, 表达) in 段.ins {
            let v = self.expr(&表达)?;
            ins.push((约束, v));
        }
        Ok(AsmForm {
            tmpl: 段.tmpl,
            outs,
            ins,
            clobbers: 段.clobbers,
        })
    }

    
    
    fn expr_tree(&mut self, e: &Expr) -> Result<Value, BuildError> {
        use prion_parser::expr::{ExprKind, Target, Verb, UnOp};
        match &e.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Float(x) => Ok(Value::Float(*x)),
            ExprKind::Str(s) => Ok(Value::Str(s.clone())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Null => {
                if let Some(exp) = self.期望类型.clone() {
                    if exp.starts_with('?') && self.type_idx.contains_key(&exp) {
                        let tmp = self.temp(Ty(exp.clone()));
                        self.emit(Stmt::Assign {
                            lhs: Place::local(tmp),
                            rhs: Rvalue::VariantLit { ty: exp, tag: 0, args: vec![], tys: vec![] },
                        });
                        return Ok(Value::Local(tmp));
                    }
                }
                Ok(Value::Null)
            }
            ExprKind::Name { target, name } => match target {
                Target::Val => self.expr_tree_val(name),
                Target::Fn => {
                    let key = format!("{}|{}|{}", self.module, self.block, name);
                    
                    let letfty = self.fns.get(&key).or_else(|| self.global_fns.get(&key)).map(|sg| {
                        let ps: Vec<String> = sg.params.iter().map(|t| t.0.clone()).collect();
                        format!("fn({}) {}", ps.join(", "), sg.ret.0)
                    }).unwrap_or_else(|| format!("fn {}", name));
                    let tmp = self.temp(Ty(letfty));
                    self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::FnRef { name: key } });
                    Ok(Value::Local(tmp))
                }
                _ => err(format!("bare name \"{}\" is not usable here", name)),
            },
            ExprKind::Call { verb, target, module, block, name, args } => {
                if *verb == Verb::Delta {
                    return err("delta does not appear in the expression tree");
                }
                if *verb == Verb::Take {
                    let src = if let Some(tb) = block {
                        let blk_mod = if let Some(m) = module {
                            if self.mod_has(m) {
                                m.clone()
                            } else {
                                match self.resolve_module(m)? {
                                    Some(f) => f,
                                    None => return err(format!("module \"{}\" not found", m)),
                                }
                            }
                        } else {
                            self.module.clone()
                        };
                        self.check_pub(&blk_mod, tb)?;
                        let slot_key = format!("{}|{}#{}", blk_mod, tb, name);
                        match self.槽或建(&slot_key) {
                            Some(id) => Value::Local(id),
                            None => return err(format!("val \"{}\" of block \"{}\" does not exist", tb, name)),
                        }
                    } else {
                        self.expr_tree_val(name)?
                    };
                    return self.take_复制(src);
                }
                let vals: Vec<Value> = args.iter().map(|a| self.expr_tree(a)).collect::<Result<_, _>>()?;
                match target {
                    Target::Fn => self.expr_tree_call_fn(name, block.as_deref(), vals, module.as_deref()),
                    Target::Val => {
                        if let Some(tb) = block {
                            let blk_mod = if let Some(m) = module {
                                if self.mod_has(m) {
                                    m.clone()
                                } else {
                                    match self.resolve_module(m)? {
                                        Some(f) => f,
                                        None => return err(format!("module \"{}\" not found", m)),
                                    }
                                }
                            } else {
                                self.module.clone()
                            };
                            self.check_pub(&blk_mod, tb)?;
                            let slot_key = format!("{}|{}#{}", blk_mod, tb, name);
                            match self.槽或建(&slot_key) {
                                Some(id) => Ok(Value::Local(id)),
                                None => err(format!("val \"{}\" of block \"{}\" does not exist", tb, name)),
                            }
                        } else if vals.is_empty() {
                            self.expr_tree_val(name)
                        } else {
                            self.expr_tree_indirect(name, vals)
                        }
                    }
                    
                    Target::Builtin if self.sum_variants.contains_key(name) => self.expr_tree_variant(name, &vals, e.id()),
                    Target::Builtin if self.type_idx.contains_key(name) => self.expr_tree_construct(name, &vals, e.id()),
                    Target::Builtin => self.expr_tree_builtin(name, &vals),
                    Target::Type | Target::Variant => {
                        if self.sum_variants.contains_key(name) {
                            self.expr_tree_variant(name, &vals, e.id())
                        } else if self.type_idx.contains_key(name) {
                            self.expr_tree_construct(name, &vals, e.id())
                        } else {
                            self.expr_tree_builtin(name, &vals)
                        }
                    }
                }
            }
            ExprKind::Index { base, index } => {
                let b = self.expr_tree(base)?;
                let i = self.expr_tree(index)?;
                
                let bty = self.value_ty(&b).trim_end_matches('*').to_string();
                if bty == "str" {
                    if let ExprKind::Int(k) = index.kind {
                        self.查串下标(&b, k)?;
                    }
                    let tmp = self.temp(Ty("int".into()));
                    self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Call { name: "#str_byte_at".into(), args: vec![b, i] } });
                    return Ok(Value::Local(tmp));
                }
                let is_map = bty == "map" || bty.starts_with("map(");
                let ety = if is_map { Self::map_val_ty(&bty).unwrap_or_else(|| "int".into()) } else { elem_ty_of(&bty).unwrap_or_else(|| "int".into()) };
                let tmp = self.temp(Ty(ety));
                self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: if is_map { Rvalue::MapGet { map: b, key: i } } else { Rvalue::ListGet { list: b, index: i } } });
                Ok(Value::Local(tmp))
            }
            ExprKind::Slice { base, from, to } => {
                let b = self.expr_tree(base)?;
                let a = self.expr_tree(from)?;
                let b2 = self.expr_tree(to)?;
                let bty = self.value_ty(&b);
                let tmp = self.temp(Ty(bty));
                self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Call { name: "slice".into(), args: vec![b, a, b2] } });
                Ok(Value::Local(tmp))
            }
            ExprKind::Field { base, name } => {
                let b = self.expr_tree(base)?;
                let bty = self.value_ty(&b);
                let owner = bty.trim_end_matches('*').to_string();
                if bty.ends_with('*') {
                    let c = self.bin(BinOp::Ne, b.clone(), Value::Null);
                    self.emit(Stmt::Assert { cond: c, msg: "null reference - the handle is null, cannot read a field".into(), fix: "check for null first (x != null) before reading a field, or catch the empty case with a case branch".into() });
                }
                let ft = self.field_ty_of(&owner, name);
                let tmp = self.temp(Ty(ft));
                let base_local = match b { Value::Local(id) => id, _ => return err("the base of a field access must be a local") };
                self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Use(Place { base: base_local, proj: vec![Proj::Field(name.clone())] }) });
                Ok(Value::Local(tmp))
            }
            ExprKind::Deref(inner) => {
                let v = self.expr_tree(inner)?;
                match v { Value::Local(id) => Ok(Value::Local(id)), _ => err("the base of a dereference must be a local") }
            }
            ExprKind::Unary { op, operand } => match op {
                UnOp::Not => {
                    let v = self.expr_tree(operand)?;
                    let t2 = self.temp(Ty("bool".into()));
                    self.emit(Stmt::Assign { lhs: Place::local(t2), rhs: Rvalue::BinOp { op: BinOp::Eq, lhs: v, rhs: Value::Bool(false) } });
                    Ok(Value::Local(t2))
                }
                UnOp::Neg => {
                    let v = self.expr_tree(operand)?;
                    let t2 = self.temp(Ty("int".into()));
                    self.emit(Stmt::Assign { lhs: Place::local(t2), rhs: Rvalue::BinOp { op: BinOp::Sub, lhs: Value::Int(0), rhs: v } });
                    Ok(Value::Local(t2))
                }
                UnOp::BitNot => {
                    let v = self.expr_tree(operand)?;
                    let t2 = self.temp(Ty("int".into()));
                    self.emit(Stmt::Assign { lhs: Place::local(t2), rhs: Rvalue::BinOp { op: BinOp::BitXor, lhs: Value::Int(-1), rhs: v } });
                    Ok(Value::Local(t2))
                }
                _ => err("tree walker: unsupported unary operation"),
            },
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.expr_tree(lhs)?;
                let r = self.expr_tree(rhs)?;
                let ir_op = match op {
                    prion_parser::expr::BinOp::Add => BinOp::Add,
                    prion_parser::expr::BinOp::Sub => BinOp::Sub,
                    prion_parser::expr::BinOp::Mul => BinOp::Mul,
                    prion_parser::expr::BinOp::Div => BinOp::Div,
                    prion_parser::expr::BinOp::Mod => BinOp::Mod,
                    prion_parser::expr::BinOp::Eq => BinOp::Eq,
                    prion_parser::expr::BinOp::Ne => BinOp::Ne,
                    prion_parser::expr::BinOp::Lt => BinOp::Lt,
                    prion_parser::expr::BinOp::Le => BinOp::Le,
                    prion_parser::expr::BinOp::Gt => BinOp::Gt,
                    prion_parser::expr::BinOp::Ge => BinOp::Ge,
                    prion_parser::expr::BinOp::And => BinOp::And,
                    prion_parser::expr::BinOp::Or => BinOp::Or,
                    prion_parser::expr::BinOp::BitAnd => BinOp::BitAnd,
                    prion_parser::expr::BinOp::BitOr => BinOp::BitOr,
                    prion_parser::expr::BinOp::BitXor => BinOp::BitXor,
                    prion_parser::expr::BinOp::Shl => BinOp::Shl,
                    prion_parser::expr::BinOp::Shr => BinOp::Shr,
                    prion_parser::expr::BinOp::Range => return err("the range operator is not a binary operator"),
                };
                Ok(self.bin(ir_op, l, r))
            }
            ExprKind::Try(inner) => {
                let v = self.expr_tree(inner)?;
                let tag = self.temp(Ty("int".into()));
                self.emit(Stmt::Assign { lhs: Place::local(tag), rhs: Rvalue::VariantTag(v.clone()) });
                let bb_fail = self.nb();
                let bb_ok = self.nb();
                let iszero = self.bin(BinOp::Eq, Value::Local(tag), Value::Int(0));
                self.set_exit(Exit::Branch { cond: iszero, targets: vec![
                    Succ { target: bb_fail, args: vec![] },
                    Succ { target: bb_ok, args: vec![] },
                ] });
                self.switch_to(bb_fail);
                self.set_exit(Exit::Return(Some(v.clone())));
                let dead = self.nb();
                self.switch_to(dead);
                self.switch_to(bb_ok);
                let payload = self.temp(Ty("int".into()));
                self.emit(Stmt::Assign {
                    lhs: Place::local(payload),
                    rhs: Rvalue::VariantGet { value: v, idx: 0, ty: Ty("int".into()) },
                });
                Ok(Value::Local(payload))
            }
            ExprKind::IfExpr { cond, then, otherwise } => {
                let c = self.expr_tree(cond)?;
                let bb_then = self.nb();
                let bb_else = self.nb();
                let bb_done = self.nb();
                self.set_exit(Exit::Branch { cond: c, targets: vec![
                    Succ { target: bb_then, args: vec![] },
                    Succ { target: bb_else, args: vec![] },
                ] });
                self.switch_to(bb_then);
                let tv = self.expr_tree(then)?;
                let mut ty = self.ty_of_value(tv.clone());
                if ty.0 == "null" {
                    if let Some(exp) = self.期望类型.clone() {
                        ty = Ty(exp);
                    }
                }
                let t2 = self.temp(ty.clone());
                self.emit(Stmt::Assign { lhs: Place::local(t2), rhs: Rvalue::Const(tv) });
                self.goto(bb_done);
                self.switch_to(bb_else);
                let ev = self.expr_tree(otherwise)?;
                let ety = self.ty_of_value(ev.clone());
                if ety.0 != ty.0 && ety.0 != "null" {
                    return err(format!(
                        "the two branches of an if expression must have the same type - one is \"{}\", the other is \"{}\"",
                        ty.0, ety.0
                    ));
                }
                self.emit(Stmt::Assign { lhs: Place::local(t2), rhs: Rvalue::Const(ev) });
                self.goto(bb_done);
                self.switch_to(bb_done);
                Ok(self.if结果把手(t2, &ty))
            }
            ExprKind::ListLit(elems) => {
                let vals: Vec<Value> = elems.iter().map(|x| self.expr_tree(x)).collect::<Result<_, _>>()?;
                for v in &vals {
                    if self.is_container_value(v) {
                        return err("a list cannot directly hold containers - to group, use a key-value table or struct fields");
                    }
                }
                let lty = self.lit_ty.take().unwrap_or_else(|| Ty("list".into()));
                let tmp = self.temp(lty);
                self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::ListLit { elems: vals } });
                Ok(Value::Local(tmp))
            }
            ExprKind::MapLit(pairs) => {
                let mut ps: Vec<(MapKey, Value)> = Vec::new();
                for (k, v) in pairs {
                    let kv = match &k.kind {
                        ExprKind::Str(s) => MapKey::Str(s.clone()),
                        ExprKind::Int(n) => MapKey::Int(*n),
                        _ => return err("map literal keys must be string or integer literals"),
                    };
                    let vv = self.expr_tree(v)?;
                    ps.push((kv, vv));
                }
                let lty = self.lit_ty.take().unwrap_or_else(|| Ty("map".into()));
                let tmp = self.temp(lty);
                self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::MapLit { pairs: ps } });
                Ok(Value::Local(tmp))
            }
        }
    }

    
    fn expr_tree_val(&mut self, name: &str) -> Result<Value, BuildError> {
        if let Some(&pid) = self.exposed.get(name) {
            let pty = self.f.locals.get(pid).map(|l| l.ty.0.clone()).unwrap_or_default();
            let vty = Ty(pty.trim_end_matches('*').to_string());
            let tmp = self.temp(vty);
            self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Use(Place { base: pid, proj: vec![Proj::Deref] }) });
            return Ok(Value::Local(tmp));
        }
        if let Some(&id) = self.names.get(name) {
            return Ok(Value::Local(id));
        }
        let slot_key = format!("{}|{}#{}", self.module, self.block, name);
        if let Some(id) = self.槽或建(&slot_key) {
            let pty = self.f.locals.get(id).map(|l| l.ty.0.clone()).unwrap_or_default();
            let vty = Ty(pty.trim_end_matches('*').to_string());
            let tmp = self.temp(vty);
            self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Use(Place { base: id, proj: vec![Proj::Deref] }) });
            return Ok(Value::Local(tmp));
        }
        err(format!("val \"{}\" has not been defined", name))
    }

    
    
    fn 推泛型实参(&self, name: &str, args: &[Value], 带能力: &[(String, Vec<String>)], 形参: &[(String, Ty)]) -> Result<(String, Vec<String>, Vec<String>), BuildError> {
        let 类型参数: Vec<String> = 带能力.iter().map(|(n, _)| n.clone()).collect();
        let mut 绑定: Vec<Option<String>> = vec![None; 类型参数.len()];
        for (k, a) in args.iter().enumerate() {
            if k >= 形参.len() { break; }
            let mut got = self.ty_of_value(a.clone()).0.clone();
            
            if let Some(名) = got.strip_prefix("fn ") {
                let fk = format!("{}|{}|{}", self.module, self.block, 名);
                let fs = self.fns.get(&fk).or_else(|| self.global_fns.get(&fk));
                if let Some(sig) = fs {
                    let ps: Vec<String> = sig.params.iter().map(|t| t.0.clone()).collect();
                    got = format!("fn({}) {}", ps.join(", "), sig.ret.0);
                }
            }
            let pat = 形参[k].1 .0.clone();
            let _ = 统一类型(&pat, &got, &类型参数, &mut 绑定);
        }
        if 绑定.iter().any(|x| x.is_none()) {
            return Err(BuildError {
                message: format!("cannot infer type parameters when calling generic fn \"{}\" - write arguments as literals or typed vals", name),
                code: prion_error::Code::E304,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            });
        }
        核能力(name, 带能力, &绑定.iter().map(|x| x.clone().unwrap()).collect::<Vec<_>>())?;
        let 实参类型: Vec<String> = 绑定.into_iter().map(|x| x.unwrap()).collect();
        Ok((format!("{}@{}", name, 实参类型.join(",")), 类型参数, 实参类型))
    }

    fn expr_tree_call_fn(&mut self, name: &str, blk: Option<&str>, args: Vec<Value>, mod_path: Option<&str>) -> Result<Value, BuildError> {
        if let Some(tb) = blk {
            
            let 指定模 = mod_path.unwrap_or(&self.module);
            
            let 属模 = if mod_path.is_some() {
                self.resolve_module(指定模)?.unwrap_or_else(|| 指定模.to_string())
            } else {
                指定模.to_string()
            };
            self.check_pub(&属模, tb)?;
            let key = format!("{}|{}|{}", 属模, tb, name);
            let sig = match self.fns.get(&key).or_else(|| self.global_fns.get(&key)).cloned() {
                Some(s) => s,
                None => {
                    
                    if let Some((带能力, 形参, 返回, 体, _, _)) = 泛型fn表.with(|g| g.borrow().get(name).cloned()) {
                        let (实例名, 类型参数, 实参类型) = match self.推泛型实参(name, &args, &带能力, &形参) {
                            Ok(v) => v,
                            Err(e) => return Err(e),
                        };
                        let 实例键 = format!("{}|{}|{}", 属模, tb, 实例名);
                        if self.fns.get(&实例键).is_none() && self.global_fns.get(&实例键).is_none() {
                            let 旧模块 = self.module.clone();
                            let 旧块 = self.block.clone();
                            self.module = 属模.clone();
                            self.block = tb.to_string();
                            let r = 建fn实例(self, &实例名, &类型参数, &实参类型, &形参, &返回, &体);
                            self.module = 旧模块;
                            self.block = 旧块;
                            r?;
                        }
                        return self.expr_tree_call_fn(&实例名, Some(tb), args, Some(&属模));
                    }
                    if let Some((带能力, 形参, 返回, 体, _, _)) = 泛型语句体fn表.with(|g| g.borrow().get(name).cloned()) {
                        let (实例名, 类型参数, 实参类型) = match self.推泛型实参(name, &args, &带能力, &形参) {
Ok(v) => v,
                            Err(e) => return Err(e),
                        };
                        let 实例键 = format!("{}|{}|{}", 属模, tb, 实例名);
                        if self.fns.get(&实例键).is_none() && self.global_fns.get(&实例键).is_none() {
                            let 旧模块 = self.module.clone();
                            let 旧块 = self.block.clone();
                            self.module = 属模.clone();
                            self.block = tb.to_string();
                            let r = self.建语句体实例(&实例名, &类型参数, &实参类型, &形参, &返回, &体);
                            self.module = 旧模块;
                            self.block = 旧块;
                            r?;
                        }
                        return self.expr_tree_call_fn(&实例名, Some(tb), args, Some(&属模));
                    }
                    return Err(BuildError {
                        message: format!("fn \"{}\" has not been defined yet - define it with add fn first", name),
                        code: prion_error::Code::E304,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
            };
            let mut vals = args;
            let _dc = self.容器实参深拷(&mut vals, &sig);
            self.实参把手(&mut vals, &sig);
            if !sig.hidden.is_empty() {
                let caller_is_fn = self.f.name.contains('#');
                vals.extend(self.槽实参(&sig, &key, caller_is_fn)?);
            }
            let tmp = self.temp(sig.ret.clone());
            self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Call { name: key, args: vals } });
            return Ok(Value::Local(tmp));
        }
        let key = format!("{}|{}|{}", self.module, self.block, name);
        if !self.fns.contains_key(&key) && !self.global_fns.contains_key(&key) {
            
            let 模板 = 泛型fn表.with(|g| g.borrow().get(name).cloned());
            if let Some((带能力, 形参, 返回, 体, _模, _块)) = 模板 {
                let 类型参数: Vec<String> = 带能力.iter().map(|(n, _)| n.clone()).collect();
                let mut 绑定: Vec<Option<String>> = vec![None; 类型参数.len()];
                for (k, a) in args.iter().enumerate() {
                    if k >= 形参.len() {
                        break;
                    }
                    let got = self.ty_of_value(a.clone()).0.clone();
                    let pat = 形参[k].1 .0.clone();
                    let _ = 统一类型(&pat, &got, &类型参数, &mut 绑定);
                }
                if 绑定.iter().any(|x| x.is_none()) {
                    return err(format!(
                        "cannot infer type parameters when calling generic fn \"{}\" - write arguments as literals or typed vals",
                        name
                    ));
                }
                let 实参类型: Vec<String> = 绑定.into_iter().map(|x| x.unwrap()).collect();
                核能力(name, &带能力, &实参类型)?;
                let 实例名 = format!("{}@{}", name, 实参类型.join(","));
                if !self.fns.contains_key(&format!("{}|{}|{}", self.module, self.block, 实例名))
                    && !self.global_fns.contains_key(&format!("{}|{}|{}", self.module, self.block, 实例名))
                {
                    建fn实例(self, &实例名, &类型参数, &实参类型, &形参, &返回, &体)?;
                }
                return self.expr_tree_call_fn(&实例名, None, args, None);
            }
            
            let 语句模板 = 泛型语句体fn表.with(|g| g.borrow().get(name).cloned());
            if let Some((带能力, 形参, 返回, 体, _模, _块)) = 语句模板 {
                let 类型参数: Vec<String> = 带能力.iter().map(|(n, _)| n.clone()).collect();
                let mut 绑定: Vec<Option<String>> = vec![None; 类型参数.len()];
                for (k, a) in args.iter().enumerate() {
                    if k >= 形参.len() {
                        break;
                    }
                    let got = self.ty_of_value(a.clone()).0.clone();
                    let pat = 形参[k].1 .0.clone();
                    let _ = 统一类型(&pat, &got, &类型参数, &mut 绑定);
                }
                if 绑定.iter().any(|x| x.is_none()) {
                    return err(format!(
                        "cannot infer type parameters when calling generic fn \"{}\" - write arguments as literals or typed vals",
                        name
                    ));
                }
                let 实参类型: Vec<String> = 绑定.into_iter().map(|x| x.unwrap()).collect();
                let 实例名 = format!("{}@{}", name, 实参类型.join(","));
                if !self.fns.contains_key(&format!("{}|{}|{}", self.module, self.block, 实例名))
                    && !self.global_fns.contains_key(&format!("{}|{}|{}", self.module, self.block, 实例名))
                {
                    self.建语句体实例(&实例名, &类型参数, &实参类型, &形参, &返回, &体)?;
                }
                return self.expr_tree_call_fn(&实例名, None, args, None);
            }
        }
        let sig = self.fns.get(&key).or_else(|| self.global_fns.get(&key)).cloned().ok_or_else(|| BuildError {
            message: format!("fn \"{}\" has not been defined yet - define it with add fn first", name),
            code: prion_error::Code::E304,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        let mut vals = args;
        let _dc = self.容器实参深拷(&mut vals, &sig);
        self.实参把手(&mut vals, &sig);
        if !sig.hidden.is_empty() {
            let caller_is_fn = self.f.name.contains('#');
            vals.extend(self.槽实参(&sig, &key, caller_is_fn)?);
        }
        let tmp = self.temp(sig.ret.clone());
        self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Call { name: key, args: vals } });
        Ok(Value::Local(tmp))
    }

    
    fn expr_tree_indirect(&mut self, name: &str, args: Vec<Value>) -> Result<Value, BuildError> {
        let id = *self.names.get(name).ok_or_else(|| BuildError {
            message: format!("val \"{}\" has not been defined", name),
            code: prion_error::Code::E304,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        let fty = self.f.locals.get(id).map(|l| l.ty.0.clone()).unwrap_or_default();
        let ret = fn_ret_of(&fty).ok_or_else(|| BuildError {
            message: format!("val \"{}\" is not a function type - it cannot be called directly", name),
            code: prion_error::Code::E201,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        let tmp = self.temp(Ty(ret));
        self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::CallIndirect { callee: Value::Local(id), args } });
        Ok(Value::Local(tmp))
    }

    
    fn expr_tree_builtin(&mut self, name: &str, vals: &[Value]) -> Result<Value, BuildError> {
        let vals = vals.to_vec();
        let mut name = name.to_string();
        let mut join_all: Option<String> = None;
        if name == "join" {
            name = "prion_join".into();
            let 内 = match vals.first() {
                Some(Value::Local(id)) => self.f.locals.get(*id).and_then(|l| token容器载荷(&l.ty.0)),
                _ => None,
            };
            if let Some(内) = 内 {
                name = "prion_join_all".into();
                join_all = Some(内);
            }
        }
        if name == "index" {
            let 首参串 = match vals.first() {
                Some(Value::Local(id)) => self.f.locals.get(*id).map(|l| l.ty.0 == "str").unwrap_or(false),
                Some(Value::Str(_)) => true,
                _ => false,
            };
            if 首参串 { name = "str_index".into(); }
        }
        let 返型 = if let Some(内) = &join_all {
            format!("list({})", 内)
        } else if name == "slice" {
            match vals.first() {
                Some(Value::Local(id)) => self.f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_else(|| "int".into()),
                _ => "int".into(),
            }
        } else {
            builtin_ret_ty(&name).0
        };
        let tmp = self.temp(Ty(返型));
        self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Call { name, args: vals } });
        Ok(Value::Local(tmp))
    }

    
    
    fn expr_tree_variant(&mut self, name: &str, vals: &[Value], root_id: u32) -> Result<Value, BuildError> {
        let (模板型, tag0, 模板载荷) = self.sum_variants.get(name).cloned().ok_or_else(|| BuildError {
            message: format!("variant \"{}\" is not registered", name),
            code: prion_error::Code::E201,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        let 期望 = match self.期望类型.clone() {
            Some(t) if t != 模板型 => Some(t),
            _ => {
                
                let 位表型 = TYPECK.with(|m| m.borrow().get(&root_id).cloned());
                match 位表型 {
                    Some(t) if t != 模板型 => Some(t),
                    _ => None,
                }
            }
        };
        let (tyname, tag, ftypes) = match 期望 {
            Some(t) => {
                let 实例载荷 = self
                    .type_idx
                    .get(&t)
                    .and_then(|i| self.types.get(*i))
                    .and_then(|st| {
                        st.variants
                            .iter()
                            .find(|v| &v.name == name)
                            .map(|v| v.fields.iter().map(|(_, ty)| ty.clone()).collect())
                    });
                match 实例载荷 {
                    Some(载) => (t, tag0, 载),
                    None => (模板型, tag0, 模板载荷),
                }
            }
            _ => (模板型, tag0, 模板载荷),
        };
        let tmp = self.temp(Ty(tyname.clone()));
        self.emit(Stmt::Assign {
            lhs: Place::local(tmp),
            rhs: Rvalue::VariantLit { ty: tyname, tag, args: vals.to_vec(), tys: ftypes },
        });
        Ok(Value::Local(tmp))
    }

    fn expr_tree_construct(&mut self, name: &str, vals: &[Value], root_id: u32) -> Result<Value, BuildError> {
        
        let 键 = if 泛型判别(name) {
            match self.期望类型.clone() {
                Some(t) if self.type_idx.contains_key(&t) => t,
                _ => {
                    
                    let 位表型 = TYPECK.with(|m| m.borrow().get(&root_id).cloned());
                    if let Some(t) = 位表型 {
                        if self.type_idx.contains_key(&t) {
                            return self.construct命名(&t, name, vals);
                        }
                    }
                    
                    let 实参: Vec<String> = vals.iter().map(|v| self.value_ty(v)).collect();
                    let 实例名 = format!("{}({})", name, 实参.join(", "));
                    if !self.type_idx.contains_key(&实例名) {
                        return err(format!(
                            "generic instance \"{}\" has not been generated - first write a typed declaration `add val \"x\" {}(...) = ...`",
                            实例名, 实例名
                        ));
                    }
                    实例名
                }
            }
        } else {
            name.to_string()
        };
        self.construct命名(&键, name, vals)
    }

    
    fn construct命名(&mut self, 实例: &str, name: &str, vals: &[Value]) -> Result<Value, BuildError> {
        let idx = *self.type_idx.get(实例).ok_or_else(|| BuildError {
            message: format!("type \"{}\" is not registered", 实例),
            code: prion_error::Code::E201,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        let st = self.types[idx].clone();
        if vals.len() != st.fields.len() {
            return err(format!("wrong number of constructor arguments - {} requires {} field(s)", name, st.fields.len()));
        }
        let sid = self.new_local(None, Ty(st.name.clone()), false, false);
        for (k, a) in vals.iter().enumerate() {
            self.emit(Stmt::Assign {
                lhs: Place { base: sid, proj: vec![Proj::Field(st.fields[k].0.clone())] },
                rhs: Rvalue::Const(a.clone()),
            });
        }
        Ok(Value::Local(sid))
    }

    fn if结果把手(&mut self, tmp: LocalId, ty: &Ty) -> Value {
        if ty.0 == "null" || ty.0.ends_with('*') {
            return Value::Local(tmp);
        }
        let Some(exp) = self.期望类型.clone() else {
            return Value::Local(tmp);
        };
        if exp != format!("{}*", ty.0) {
            return Value::Local(tmp);
        }
        let ptr = self.temp(Ty(exp));
        if let Some(l) = self.f.locals.get_mut(ptr) {
            l.fn_owned = true;
        }
        self.emit(Stmt::Assign {
            lhs: Place::local(ptr),
            rhs: Rvalue::Const(Value::Local(tmp)),
        });
        Value::Local(ptr)
    }

    fn 实参把手(&mut self, vals: &mut [Value], sig: &FnSig) {
        for (k, v) in vals.iter_mut().enumerate() {
            let Some(p) = sig.params.get(k) else { continue };
            if !p.0.ends_with('*') {
                continue;
            }
            let base = p.0.trim_end_matches('*');
            let id = match v {
                Value::Local(id) => *id,
                _ => continue,
            };
            let ty = self.f.locals.get(id).map(|l| l.ty.0.clone()).unwrap_or_default();
            if ty != base || !self.type_idx.contains_key(base) {
                continue;
            }
            let ptr = self.temp(Ty(p.0.clone()));
            if let Some(l) = self.f.locals.get_mut(ptr) {
                l.fn_owned = true;
            }
            self.emit(Stmt::Assign {
                lhs: Place::local(ptr),
                rhs: Rvalue::Const(Value::Local(id)),
            });
            *v = Value::Local(ptr);
        }
    }

    
    fn value_ty(&self, v: &Value) -> String {
        match v {
            Value::Local(id) => self.f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_default(),
            Value::Int(_) => "int".into(),
            Value::Float(_) => "float".into(),
            Value::Str(_) => "str".into(),
            Value::Bool(_) => "bool".into(),
            _ => String::new(),
        }
    }

    fn expr(&mut self, 段: &[Token]) -> Result<Value, BuildError> {
        if 段.is_empty() {
            return err("the expression is empty");
        }
        
        if matches!(段.first(), Some(Token::Kw("take"))) {
            let mut syn = 段.to_vec();
            syn[0] = Token::Kw("call");
            let v = self.expr(&syn)?;
            return self.take_复制(v);
        }
        
        if matches!(段.first(), Some(Token::Kw("call")))
            && matches!(段.get(1), Some(Token::Kw("val")))
        {
            if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 段.get(2) {
                let 是常量 = self.常量值.borrow().contains_key(n.as_ref());
                if 是常量 {
                    return Err(BuildError {
                        message: format!(
                            "\"{}\" is a compile-time constant; read it with `call fix \"{}\"`, not call val",
                            n, n
                        ),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
            }
        }
        
        if matches!(段.first(), Some(Token::Kw("call"))) {
            let mut 定位: Option<usize> = None;
            let mut 跨模块目标: Option<(String, String)> = None;
            if matches!(段.get(1), Some(Token::Kw("fix"))) {
                定位 = Some(1);
            } else if matches!(段.get(1), Some(Token::HashColon))
                && matches!(段.get(3), Some(Token::Kw("fix")))
            {
                定位 = Some(3);
            } else if matches!(段.get(1), Some(Token::HashHashColon)) {
                let mut m = String::new();
                let mut j = 2usize;
                while let Some(tok) = 段.get(j) {
                    match tok {
                        Token::Ident(n) | Token::Str(n) => {
                            if !m.is_empty() {
                                m.push('.');
                            }
                            m.push_str(n);
                            j += 1;
                            if matches!(段.get(j), Some(Token::Sym("."))) {
                                j += 1;
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
                if matches!(段.get(j), Some(Token::HashColon))
                    && matches!(段.get(j + 2), Some(Token::Kw("fix")))
                {
                    let blk = match 段.get(j + 1) {
                        Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
                        Some(Token::Kw(k)) => (*k).to_string().into(),
                        _ => return err("`#:` must be followed by a block name"),
                    };
                    let full = if self.mod_has(&m) {
                        m
                    } else {
                        match self.resolve_module(&m)? {
                            Some(f) => f,
                            None => return err(format!("module \"{}\" not found", m)),
                        }
                    };
                    self.check_pub(&full, &blk)?;
                    跨模块目标 = Some((full, blk.to_string()));
                    定位 = Some(j + 2);
                }
            }
            if let Some(k) = 定位 {
                let n = match 段.get(k + 1) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => return err("`call fix` must be followed by a constant name"),
                };
                if let Some((m, b)) = &跨模块目标 {
                    if self.hidden_items.contains(&format!("{}|{}|fix|{}", m, b, n)) {
                        return Err(BuildError {
                            message: format!("constant \"{}\" is removed by hide and cannot be taken across modules", n),
                            code: prion_error::Code::E201,
                            line: CUR_LINE.with(|c| c.get()),
                            file: String::new(),
                        });
                    }
                }
                let c = self.常量值.borrow().get::<str>(n.as_ref()).cloned().ok_or_else(|| BuildError {
                    message: format!("no constant \"{}\" - declare it with add fix first", n),
                    code: prion_error::Code::E304,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
                let mut 新 = vec![c.as_token()];
                新.extend_from_slice(&段[k + 2..]);
                return self.expr(&新);
            }
        }
        
        if matches!(段.first(), Some(Token::Kw("call")))
            && matches!(段.get(1), Some(Token::Kw("val")))
            && !matches!(段.get(3), Some(Token::Sym("(")))
        {
            return self.expr(&段[1..]);
        }
        
        if 段.first().map(prion_lexer::asm::是asm词).unwrap_or(false)
            && matches!(段.get(1), Some(Token::Sym("(")))
        {
            let 内区 = prion_lexer::asm::括号内(段, 1)
                .ok_or_else(|| BuildError {
                    message: "asm parentheses are not closed".into(),
                    code: prion_error::Code::E301,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?
                .to_vec();
            let form = self.建asm(&内区)?;
            let 首 = match form.outs.first() {
                Some((_, id, _)) => *id,
                None => return err("asm has no output constraint and cannot be assigned to a variable as a value - it is an action statement"),
            };
            self.emit(Stmt::Asm(form));
            return Ok(Value::Local(首));
        }
        let mut i = 0usize;
        let v = self.parse_or(段, &mut i)?;
        
        if matches!(段.get(i), Some(Token::Sym("?"))) {
            let colon_opt = 段
                .iter()
                .enumerate()
                .skip(i + 1)
                .find(|(_, t)| matches!(t, Token::Sym(":")))
                .map(|(k, _)| k);
            if colon_opt.is_some() {
                return Err(BuildError {
                    message: "the ternary `cond ? a : b` is retired - use an if expression: `if cond { a } else { b }`".into(),
                    code: prion_error::Code::E301,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                });
            }
            {
                let tag = self.temp(Ty("int".into()));
                self.emit(Stmt::Assign { lhs: Place::local(tag), rhs: Rvalue::VariantTag(v.clone()) });
                let bb_fail = self.nb();
                let bb_ok = self.nb();
                let iszero = self.bin(BinOp::Eq, Value::Local(tag), Value::Int(0));
                self.set_exit(Exit::Branch { cond: iszero, targets: vec![
                    Succ { target: bb_fail, args: vec![] },
                    Succ { target: bb_ok, args: vec![] },
                ] });
                self.switch_to(bb_fail);
                self.set_exit(Exit::Return(Some(v.clone())));
                let dead = self.nb();
                self.switch_to(dead);
                self.switch_to(bb_ok);
                let payload = self.temp(Ty("int".into()));
                self.emit(Stmt::Assign {
                    lhs: Place::local(payload),
                    rhs: Rvalue::VariantGet { value: v, idx: 0, ty: Ty("int".into()) },
                });
                return Ok(Value::Local(payload));
            }
        }
        if i != 段.len() {
            return err(format!("expression not fully consumed (extra: {:?})", &段[i..]));
        }
        Ok(v)
    }

    fn parse_or(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.parse_and(t, i)?;
        while matches!(t.get(*i), Some(Token::Kw("or"))) {
            *i += 1;
            let r = self.parse_and(t, i)?;
            l = self.bin(BinOp::Or, l, r);
        }
        Ok(l)
    }

    fn parse_and(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.parse_cmp(t, i)?;
        while matches!(t.get(*i), Some(Token::Kw("and"))) {
            *i += 1;
            let r = self.parse_cmp(t, i)?;
            l = self.bin(BinOp::And, l, r);
        }
        Ok(l)
    }

    fn parse_cmp(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.parse_bitor(t, i)?;
        while let Some(op) = match t.get(*i) {
            Some(Token::Sym("==")) => Some(BinOp::Eq),
            Some(Token::Sym("!=")) => Some(BinOp::Ne),
            Some(Token::Sym("<=")) => Some(BinOp::Le),
            Some(Token::Sym(">=")) => Some(BinOp::Ge),
            Some(Token::Sym("<")) => Some(BinOp::Lt),
            Some(Token::Sym(">")) => Some(BinOp::Gt),
            _ => None,
        } {
            *i += 1;
            let r = self.parse_bitor(t, i)?;

            if self.is_bool_value(&l) != self.is_bool_value(&r) {
                return err("chained comparison (e.g. `1 < x < 5`) is not supported; split it into two comparisons joined with `and` (e.g. `1 < x and x < 5`)");
            }
            l = self.bin(op, l, r);
        }
        Ok(l)
    }


    fn parse_bitor(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.parse_bitxor(t, i)?;
        while matches!(t.get(*i), Some(Token::Sym("|"))) {
            *i += 1;
            let r = self.parse_bitxor(t, i)?;
            l = self.bin(BinOp::BitOr, l, r);
        }
        Ok(l)
    }

    fn parse_bitxor(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.parse_bitand(t, i)?;
        while matches!(t.get(*i), Some(Token::Sym("^"))) {
            *i += 1;
            let r = self.parse_bitand(t, i)?;
            l = self.bin(BinOp::BitXor, l, r);
        }
        Ok(l)
    }

    fn parse_bitand(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.parse_shift(t, i)?;
        while matches!(t.get(*i), Some(Token::Sym("&"))) {
            *i += 1;
            let r = self.parse_shift(t, i)?;
            l = self.bin(BinOp::BitAnd, l, r);
        }
        Ok(l)
    }

    fn parse_shift(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.parse_add(t, i)?;
        while let Some(op) = match t.get(*i) {
            Some(Token::Sym("<<")) => Some(BinOp::Shl),
            Some(Token::Sym(">>")) => Some(BinOp::Shr),
            _ => None,
        } {
            *i += 1;
            let r = self.parse_add(t, i)?;
            l = self.bin(op, l, r);
        }
        Ok(l)
    }

    fn parse_add(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.parse_mul(t, i)?;
        while let Some(op) = match t.get(*i) {
            Some(Token::Sym("+")) => Some(BinOp::Add),
            Some(Token::Sym("-")) => Some(BinOp::Sub),
            _ => None,
        } {
            *i += 1;
            let r = self.parse_mul(t, i)?;
            l = self.bin(op, l, r);
        }
        Ok(l)
    }

    fn parse_mul(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        let mut l = self.operand(t, i)?;
        while let Some(op) = match t.get(*i) {
            Some(Token::Sym("*")) => Some(BinOp::Mul),
            Some(Token::Sym("/")) => Some(BinOp::Div),
            Some(Token::Sym("%")) => Some(BinOp::Mod),
            _ => None,
        } {
            *i += 1;
            let r = self.operand(t, i)?;
            l = self.bin(op, l, r);
        }
        Ok(l)
    }

    
    fn index_read(
        &mut self,
        id: LocalId,
        vty: String,
        t: &[Token],
        i: &mut usize,
    ) -> Result<Value, BuildError> {
        let mut depth = 0i32;
        let mut close = None;
        for (k, tk) in t.iter().enumerate().skip(*i) {
            match tk {
                Token::Sym("[") => depth += 1,
                Token::Sym("]") => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(k);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close.ok_or_else(|| BuildError { message: "index is missing ]".into() , code: prion_error::Code::E302 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
        if t[*i + 1..close].iter().any(|tk| *tk == Token::Sym("~")) {
            let pos = t[*i + 1..close].iter().position(|tk| *tk == Token::Sym("~")).unwrap();
            let a = self.expr(&t[*i + 1..*i + 1 + pos])?;
            let b2 = self.expr(&t[*i + 1 + pos + 1..close])?;
            *i = close + 1;
            let st = self.temp(Ty(vty.clone()));
            self.emit(Stmt::Assign {
                lhs: Place::local(st),
                rhs: Rvalue::Call {
                    name: "slice".into(),
                    args: vec![Value::Local(id), a, b2],
                },
            });
            return Ok(Value::Local(st));
        }
        let 下标段 = &t[*i + 1..close];
        let idx = self.expr(下标段)?;
        *i = close + 1;
        if vty == "str" {
            if let [Token::Int(k)] = 下标段 {
                self.查串下标(&Value::Local(id), *k)?;
            }
            let tmp = self.temp(Ty("int".into()));
            self.emit(Stmt::Assign {
                lhs: Place::local(tmp),
                rhs: Rvalue::Call { name: "#str_byte_at".into(), args: vec![Value::Local(id), idx] },
            });
            return Ok(Value::Local(tmp));
        }
        let is_map = vty == "map" || vty.starts_with("map(");
        let tmp = self.temp(Ty(if is_map {
            Self::map_val_ty(&vty).unwrap_or_else(|| "int".into())
        } else {
            elem_ty_of(&vty).unwrap_or_else(|| "int".into())
        }));
        self.emit(Stmt::Assign {
            lhs: Place::local(tmp),
            rhs: if is_map {
                Rvalue::MapGet { map: Value::Local(id), key: idx }
            } else {
                Rvalue::ListGet { list: Value::Local(id), index: idx }
            },
        });
        if matches!(t.get(*i), Some(Token::Sym("."))) {
            let ety = self.f.locals.get(tmp).map(|l| l.ty.0.clone()).unwrap_or_default();
            return self.field_chain(tmp, ety, t, i);
        }
        Ok(Value::Local(tmp))
    }

    fn operand(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        match t.get(*i) {
            
            Some(Token::Sym("-")) => {
                
                if let Some(Token::Int(n)) = t.get(*i + 1) {
                    *i += 2;
                    Ok(Value::Int(-n))
                } else if let Some(Token::Float(x)) = t.get(*i + 1) {
                    *i += 2;
                    Ok(Value::Float(-x))
                } else {
                    err("a unary minus must be followed by a numeric literal".to_string())
                }
            }
            Some(Token::Int(n)) => {
                *i += 1;
                Ok(Value::Int(*n))
            }
            Some(Token::Float(x)) => {
                *i += 1;
                Ok(Value::Float(*x))
            }
            Some(Token::Str(s)) => {
                
                if matches!(t.get(*i + 1), Some(Token::Sym("(")))
                    && (self.type_idx.contains_key(s.as_ref())
                        || self.sum_variants.contains_key(s.as_ref())
                        || 泛型判别(s))
                {
                    let mut 新: Vec<Token> = t.to_vec();
                    新[*i] = Token::Ident(s.clone());
                    return self.operand(&新, i);
                }
                *i += 1;
                Ok(Value::Str(s.to_string()))
            }

            Some(Token::RawStr(s)) => {
                *i += 1;
                Ok(Value::Str(s.to_string()))
            }

            
            Some(Token::Kw("not")) => {
                *i += 1;
                let v = self.operand(t, i)?;
                let t2 = self.temp(Ty("bool".into()));
                self.emit(Stmt::Assign {
                    lhs: Place::local(t2),
                    rhs: Rvalue::BinOp { op: BinOp::Eq, lhs: v, rhs: Value::Bool(false) },
                });
                Ok(Value::Local(t2))
            }
            Some(Token::Kw("re")) => {
                *i += 1;
                Ok(Value::Bool(true))
            }
            Some(Token::Kw("fa")) => {
                *i += 1;
                Ok(Value::Bool(false))
            }
            Some(Token::Kw("null")) => {
                *i += 1;
                if let Some(exp) = self.期望类型.clone() {
                    if exp.starts_with('?') && self.type_idx.contains_key(&exp) {
                        let tmp = self.temp(Ty(exp.clone()));
                        self.emit(Stmt::Assign {
                            lhs: Place::local(tmp),
                            rhs: Rvalue::VariantLit { ty: exp, tag: 0, args: vec![], tys: vec![] },
                        });
                        return Ok(Value::Local(tmp));
                    }
                }
                Ok(Value::Null)
            }
            
            Some(Token::Sym("(")) => {
                let mut d = 0i32;
                let mut end = None;
                for (k, tk) in t.iter().enumerate().skip(*i) {
                    match tk {
                        Token::Sym("(") => d += 1,
                        Token::Sym(")") => {
                            d -= 1;
                            if d == 0 {
                                end = Some(k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let end = end.ok_or_else(|| BuildError { message: "parentheses are not closed".into() , code: prion_error::Code::E301 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
                let inner: Vec<Token> = t[*i + 1..end].to_vec();
                *i = end + 1;
                self.expr(&inner)
            }
            
            Some(Token::Kw("call")) if matches!(t.get(*i + 1), Some(Token::Kw("fix"))) => {
                let vname = match t.get(*i + 2) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => return err("`call fix` must be followed by a constant name"),
                };
                let Some(c) = self.常量值.borrow().get(&vname).cloned() else {
                    return Err(BuildError {
                        message: format!("constant \"{}\" has not been defined yet - define it with add fix first", vname),
                        code: prion_error::Code::E304,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                };
                *i += 3;
                return Ok(c.转值());
            }
            

            Some(Token::Kw("call")) if matches!(t.get(*i + 1), Some(Token::Kw("val"))) => {
                let vname = match t.get(*i + 2) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => return err("`call val` must be followed by a val name"),
                };
                if self.常量值.borrow().contains_key(&vname) {
                    return Err(BuildError {
                        message: format!(
                            "\"{}\" is a compile-time constant; read it with `call fix \"{}\"`, not call val",
                            vname, vname
                        ),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
                
                if let Some(&pid) = self.exposed.get(&vname) {
                    if !matches!(t.get(*i + 3), Some(Token::Sym("("))) {
                        let pty = self
                            .f
                            .locals
                            .get(pid)
                            .map(|l| l.ty.0.clone())
                            .unwrap_or_default();
                        let vty = Ty(pty.trim_end_matches('*').to_string());
                        let tmp = self.new_local(None, vty.clone(), false, false);
                        self.emit(Stmt::Assign {
                            lhs: Place::local(tmp),
                            rhs: Rvalue::Use(Place { base: pid, proj: vec![Proj::Deref] }),
                        });
                        *i += 3;
                        if matches!(t.get(*i), Some(Token::Sym("."))) {
                            return self.field_chain(tmp, vty.0.clone(), t, i);
                        }
                        if matches!(t.get(*i), Some(Token::Sym("["))) {
                            return self.index_read(tmp, vty.0.clone(), t, i);
                        }
                        return Ok(Value::Local(tmp));
                    }
                }
                if self.常量值.borrow().contains_key(&vname) {
                    return Err(BuildError {
                        message: format!(
                            "\"{}\" is a compile-time constant; read it with `call fix \"{}\"`, not call val",
                            vname, vname
                        ),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
                let id = *self.names.get(&vname).ok_or_else(|| BuildError {
                    message: format!("this block has no val \"{}\" - a same-block val read takes this block's", vname),
                    code: prion_error::Code::E304,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
                
                if matches!(t.get(*i + 3), Some(Token::Sym("("))) {
                    let fty = self
                        .f
                        .locals
                        .get(id)
                        .map(|l| l.ty.0.clone())
                        .unwrap_or_default();
                    let ret = fn_ret_of(&fty).ok_or_else(|| BuildError {
                        message: format!("val \"{}\" is not a function type - it cannot be called directly", vname),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    })?;
                    *i += 4;
                    let mut depth = 1i32;
                    let mut inner: Vec<Token> = Vec::new();
                    while *i < t.len() {
                        match &t[*i] {
                            Token::Sym("(") => depth += 1,
                            Token::Sym(")") => {
                                depth -= 1;
                                if depth == 0 {
                                    *i += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        inner.push(t[*i].clone());
                        *i += 1;
                    }
                    if depth != 0 {
                        return err("call parentheses are not closed");
                    }
                    let mut vals = Vec::new();
                    for a in split_args(&inner) {
                        vals.push(self.expr(&a)?);
                    }
                    let tmp = self.temp(Ty(ret));
                    self.emit(Stmt::Assign {
                        lhs: Place::local(tmp),
                        rhs: Rvalue::CallIndirect { callee: Value::Local(id), args: vals },
                    });
                    return Ok(Value::Local(tmp));
                }
                *i += 3;
                let vty = self
                    .f
                    .locals
                    .get(id)
                    .map(|l| l.ty.0.clone())
                    .unwrap_or_default();
                if matches!(t.get(*i), Some(Token::Sym("."))) {
                    return self.field_chain(id, vty, t, i);
                }
                if matches!(t.get(*i), Some(Token::Sym("["))) {
                    return self.index_read(id, vty, t, i);
                }
                return Ok(Value::Local(id));
            }
            
            
            
            
            Some(Token::Kw("call"))
                if (matches!(t.get(*i + 1), Some(Token::HashColon))
                    && matches!(t.get(*i + 3), Some(Token::Kw("val"))))
                    || (matches!(t.get(*i + 1), Some(Token::HashHashColon))
                        && take_kind(t) == Some("val")) =>
            {
                let mut colon_at: Option<usize> = None; 
                let (blk_mod, blk) = if matches!(t.get(*i + 1), Some(Token::HashColon)) {
                    let blk = match t.get(*i + 2) {
                        Some(Token::Ident(n)) => n.to_string(),
                        Some(Token::Kw(k)) => (*k).to_string().into(),
                        _ => return err("`call #:` must be followed by a block name"),
                    };
                    (self.module.clone(), blk)
                } else {
                    
                    let mut m = String::new();
                    let mut j = 2usize;
                    while let Some(tok) = t.get(j) {
                        match tok {
                            Token::Ident(n) | Token::Str(n) => {
                                if !m.is_empty() {
                                    m.push('.');
                                }
                                m.push_str(n);
                                j += 1;
                                if matches!(t.get(j), Some(Token::Sym("."))) {
                                    j += 1;
                                } else {
                                    break;
                                }
                            }
                            _ => break,
                        }
                    }
                    if m.is_empty() || !matches!(t.get(j), Some(Token::HashColon)) {
                        return err("`##:` must be followed by a module name then `#:block` - e.g. `##:frontend.tokenizer #:split`");
                    }
                    let blk = match t.get(j + 1) {
                        Some(Token::Ident(n)) => n.to_string(),
                        Some(Token::Kw(k)) => (*k).to_string().into(),
                        _ => return err("`##:module #:` must be followed by a block name"),
                    };
                    colon_at = Some(j);
                    
                    let full = if self.mod_has(&m) {
                        m.clone()
                    } else {
                        match self.resolve_module(&m)? {
                            Some(f) => f,
                            None => return err(format!("module \"{}\" not found", m)),
                        }
                    };
                    (full, blk)
                };
                
                let (vname, adv) = match colon_at {
                    None => (
                        match t.get(*i + 4) {
                            Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                            _ => return err("`call #:block val` must be followed by a val name"),
                        },
                        5,
                    ),
                    Some(j) => (
                        match t.get(j + 3) {
                            Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                            _ => return err("`call ##:module #:block val` must be followed by a val name"),
                        },
                        j + 4,
                    ),
                };

                self.check_pub(&blk_mod, &blk)?;
                let slot_key = format!("{}|{}#{}", blk_mod, blk, vname);
                let sid = match self.槽或建(&slot_key) {
                    Some(id) => id,
                    None => {
                        return Err(BuildError {
                            message: format!(
                                "val \"{}\" of block \"{}\" does not exist - define it with add val first; for an sle block, call #:{} first too",
                                blk, vname, blk
                            ),
                            code: prion_error::Code::E309,
                            line: CUR_LINE.with(|c| c.get()),
                            file: String::new(),
                        })
                    }
                };
                *i += adv;
                Ok(Value::Local(sid))
            }
            
            Some(Token::Kw("call")) if matches!(t.get(*i + 1), Some(Token::Kw("fn"))) => {
                let fname = match t.get(*i + 2) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => return err("`call fn` must be followed by a function name"),
                };
                if !matches!(t.get(*i + 3), Some(Token::Sym("("))) {
                    return err("calls require an argument list `(...)`");
                }
                let key = format!("{}|{}|{}", self.module, self.block, fname);
                
                if !self.fns.contains_key(&key) && !self.global_fns.contains_key(&key) {
                    let 模板 = 泛型fn表.with(|g| g.borrow().get(&fname).cloned());
                    if let Some((带能力, 形参, 返回, 体, _模, _块)) = 模板 {
                        let 类型参数: Vec<String> = 带能力.iter().map(|(n, _)| n.clone()).collect();
                        let 实参段 = 取实参段(t, *i + 3);
                        let mut 绑定: Vec<Option<String>> = vec![None; 类型参数.len()];
                        for (k, a) in 实参段.iter().enumerate() {
                            if k >= 形参.len() {
                                break;
                            }
                            let got = match self.推断类型(a) {
                                Some(g) => g,
                                None => continue,
                            };
                            let pat = 形参[k].1 .0.clone();
                            let _ = 统一类型(&pat, &got, &类型参数, &mut 绑定);
                        }
                        if 绑定.iter().any(|x| x.is_none()) {
                            return err(format!(
                                "cannot infer type parameters when calling generic fn \"{}\" - write arguments as literals or typed vals",
                                fname
                            ));
                        }
                        let 实参类型: Vec<String> = 绑定.into_iter().map(|x| x.unwrap()).collect();
                        核能力(&fname, &带能力, &实参类型)?;
                        let 实例名 = format!("{}@{}", fname, 实参类型.join(","));
                        if !self.fns.contains_key(&format!("{}|{}|{}", self.module, self.block, 实例名))
                            && !self.global_fns.contains_key(&format!("{}|{}|{}", self.module, self.block, 实例名))
                        {
                            建fn实例(self, &实例名, &类型参数, &实参类型, &形参, &返回, &体)?;
                        }
                        let 实例键 = format!("{}|{}|{}", self.module, self.block, 实例名);
                        if let Some(s) = self
                            .fns
                            .get(&实例键)
                            .or_else(|| self.global_fns.get(&实例键))
                            .cloned()
                        {
                            let _ = s;
                            
                            let mut 新: Vec<Token> = t.to_vec();
                            if let Some(slot) = 新.get_mut(*i + 2) {
                                *slot = Token::Ident(实例名.into());
                            }
                            return self.operand(&新, i);
                        }
                    }
                    
                    let 语句模板 = 泛型语句体fn表.with(|g| g.borrow().get(&fname).cloned());
                    if let Some((带能力, 形参, 返回, 体, _模, _块)) = 语句模板 {
                        let 类型参数: Vec<String> = 带能力.iter().map(|(n, _)| n.clone()).collect();
                        let 实参段 = 取实参段(t, *i + 3);
                        let mut 绑定: Vec<Option<String>> = vec![None; 类型参数.len()];
                        for (k, a) in 实参段.iter().enumerate() {
                            if k >= 形参.len() {
                                break;
                            }
                            let got = match self.推断类型(a) {
                                Some(g) => g,
                                None => continue,
                            };
                            let pat = 形参[k].1 .0.clone();
                            let _ = 统一类型(&pat, &got, &类型参数, &mut 绑定);
                        }
                        if 绑定.iter().any(|x| x.is_none()) {
                            return err(format!(
                                "cannot infer type parameters when calling generic fn \"{}\" - write arguments as literals or typed vals",
                                fname
                            ));
                        }
                        let 实参类型: Vec<String> = 绑定.into_iter().map(|x| x.unwrap()).collect();
                        核能力(&fname, &带能力, &实参类型)?;
                        let 实例名 = format!("{}@{}", fname, 实参类型.join(","));
                        let 实例键 = format!("{}|{}|{}", self.module, self.block, 实例名);
                        if !self.fns.contains_key(&实例键) && !self.global_fns.contains_key(&实例键) {
                            self.建语句体实例(&实例名, &类型参数, &实参类型, &形参, &返回, &体)?;
                        }
                        if self
                            .fns
                            .get(&实例键)
                            .or_else(|| self.global_fns.get(&实例键))
                            .is_some()
                        {
                            let mut 新: Vec<Token> = t.to_vec();
                            if let Some(slot) = 新.get_mut(*i + 2) {
                                *slot = Token::Ident(实例名.into());
                            }
                            return self.operand(&新, i);
                        }
                    }
                }
                
                let sig = self
                    .fns
                    .get(&key)
                    .or_else(|| self.global_fns.get(&key))
                    .cloned()
                    .ok_or_else(|| BuildError {
                    message: format!("fn \"{}\" has not been defined yet - define it with add fn first", fname),
                    code: prion_error::Code::E304,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
                *i += 4;
                let mut depth = 1i32;
                let mut inner: Vec<Token> = Vec::new();
                while *i < t.len() {
                    match &t[*i] {
                        Token::Sym("(") => depth += 1,
                        Token::Sym(")") => {
                            depth -= 1;
                            if depth == 0 {
                                *i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    inner.push(t[*i].clone());
                    *i += 1;
                }
                if depth != 0 {
                    return err("call parentheses are not closed");
                }
                let mut vals = Vec::new();
                for a in split_args(&inner) {
                    vals.push(self.expr(&a)?);
                }
                let dc_tmps = self.容器实参深拷(&mut vals, &sig);
                
                if !sig.hidden.is_empty() {
                    let caller_is_fn = self.f.name.contains('#');
                    vals.extend(self.槽实参(&sig, &key, caller_is_fn)?);
                }
                let tmp = self.temp(sig.ret.clone());
                
                let call_name = if sig.is_outs { fname.to_string() } else { key };
                self.emit(Stmt::Assign {
                    lhs: Place::local(tmp),
                    rhs: Rvalue::Call { name: call_name, args: vals },
                });
                
                for t in dc_tmps {
                    self.emit(Stmt::Assign {
                        lhs: Place::local(t),
                        rhs: Rvalue::Const(Value::Int(0)),
                    });
                }
                Ok(Value::Local(tmp))
            }
            
            Some(Token::Kw("call")) if matches!(t.get(*i + 1), Some(Token::HashColon)) => {
                *i += 1; 
                let blk = match t.get(*i + 1) {
                    Some(Token::Ident(n)) => n.to_string(),
                    Some(Token::Kw(k)) => (*k).to_string().into(),
                    _ => return err("`#:` must be followed by a block name"),
                };
                if !matches!(t.get(*i + 2), Some(Token::Kw("fn"))) {
                    return err("cross-block use currently only supports `#:block fn \"name\"(...)`");
                }
                let fname = match t.get(*i + 3) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => return err("`#:block fn` must be followed by a function name"),
                };
                if !matches!(t.get(*i + 4), Some(Token::Sym("("))) {
                    return err("cross-block use requires an argument list `(...)`");
                }
                let key = format!("{}|{}|{}", self.module, blk, fname);
                let sig = self.global_fns.get(&key).cloned().ok_or_else(|| BuildError {
                    message: format!("block \"{}\" has no fn \"{}\"", blk, fname),
                    code: prion_error::Code::E304,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
                *i += 5;
                let mut depth = 1i32;
                let mut inner: Vec<Token> = Vec::new();
                while *i < t.len() {
                    match &t[*i] {
                        Token::Sym("(") => depth += 1,
                        Token::Sym(")") => {
                            depth -= 1;
                            if depth == 0 {
                                *i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    inner.push(t[*i].clone());
                    *i += 1;
                }
                if depth != 0 {
                    return err("cross-block call parentheses are not closed");
                }
                let mut vals = Vec::new();
                for a in split_args(&inner) {
                    vals.push(self.expr(&a)?);
                }
                
                if !sig.hidden.is_empty() {
                    vals.extend(self.槽实参(&sig, &key, true)?);
                }
                let tmp = self.temp(sig.ret.clone());
                let call_name = if sig.is_outs { fname.to_string() } else { key };
                self.emit(Stmt::Assign {
                    lhs: Place::local(tmp),
                    rhs: Rvalue::Call { name: call_name, args: vals },
                });
                Ok(Value::Local(tmp))
            }
            

            
            Some(Token::Kw("call"))
                if matches!(t.get(*i + 1), Some(Token::HashHashColon))
                    && hh_is_fn_form(t, *i + 2) =>
            {
                *i += 1; 
                
                let mut module = String::new();
                let mut j = *i + 1;
                while let Some(tok) = t.get(j) {
                    match tok {
                        Token::Ident(n) | Token::Str(n) => {
                            if !module.is_empty() {
                                module.push('.');
                            }
                            module.push_str(n);
                            j += 1;
                            if matches!(t.get(j), Some(Token::Sym("."))) {
                                j += 1;
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
                if module.is_empty() || !matches!(t.get(j), Some(Token::HashColon)) {
                    return err("`##:` must be followed by a module name then `#:block` - e.g. `##:frontend.tokenizer #:split`");
                }
                let blk = match t.get(j + 1) {
                    Some(Token::Ident(n)) => n.to_string(),
                    Some(Token::Kw(k)) => (*k).to_string().into(),
                    _ => return err("`##:module #:` must be followed by a block name"),
                };
                if !matches!(t.get(j + 2), Some(Token::Kw("fn"))) {
                    return err("cross-module use currently only supports `##:module #:block fn \"name\"(...)`");
                }
                let fname = match t.get(j + 3) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => return err("`##:module #:block fn` must be followed by a function name"),
                };
                if !matches!(t.get(j + 4), Some(Token::Sym("("))) {
                    return err("cross-module use requires an argument list `(...)`");
                }
                let key = format!("{}|{}|{}", module, blk, fname);

                let key = if self.global_fns.contains_key(&key) {
                    key
                } else if let Some(full) = self.resolve_module(&module)? {
                    format!("{}|{}|{}", full, blk, fname)
                } else {
                    
                    return err(format!(
                        "module \"{}\" not found - single-file compilation does not load modules; build as a directory project (entry in main.an), or check that the module file exists",
                        module
                    ));
                };

                let full_mod = key.split('|').next().unwrap_or(&module).to_string();
                self.check_pub(&full_mod, &blk)?;
                if self
                    .hidden_items
                    .contains(&format!("{}|{}|fn|{}", full_mod, blk, fname))
                {
                    return Err(BuildError {
                        message: format!(
                            "fn \"{}\" is hidden in block \"{}\" (module {}) - it is not part of its public surface",
                            fname, blk, full_mod
                        ),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
                let mut 调用键 = key.clone();
                let sig = match self.global_fns.get(&key).cloned().or_else(|| self.fns.get(&key).cloned()) {
                    Some(s) => s,
                    None => {
                        let 模板 = 泛型语句体fn表.with(|g| g.borrow().get(&fname).cloned());
                        let Some((带能力, 形参, 返回, 体, _模, _块)) = 模板 else {
                            return Err(BuildError {
                                message: format!("block \"{}\" of module \"{}\" has no fn \"{}\"", module, blk, fname),
                                code: prion_error::Code::E304,
                                line: CUR_LINE.with(|c| c.get()),
                                file: String::new(),
                            });
                        };
                        let 类型参数: Vec<String> = 带能力.iter().map(|(n, _)| n.clone()).collect();
                        let 实参段 = 取实参段(t, j + 4);
                        let mut 绑定: Vec<Option<String>> = vec![None; 类型参数.len()];
                        for (k, a) in 实参段.iter().enumerate() {
                            if k >= 形参.len() {
                                break;
                            }
                            let got = match self.推断类型(a) {
                                Some(g) => g,
                                None => continue,
                            };
                            let pat = 形参[k].1 .0.clone();
                            let _ = 统一类型(&pat, &got, &类型参数, &mut 绑定);
                        }
                        if 绑定.iter().any(|x| x.is_none()) {
                            return err(format!(
                                "cannot infer type parameters when calling generic fn \"{}\" - write arguments as literals or typed vals",
                                fname
                            ));
                        }
                        let 实参类型: Vec<String> = 绑定.into_iter().map(|x| x.unwrap()).collect();
                        核能力(&fname, &带能力, &实参类型)?;
                        let 实例名 = format!("{}@{}", fname, 实参类型.join(","));
                        let 实例键 = format!("{}|{}|{}", full_mod, blk, 实例名);
                        if self.fns.get(&实例键).is_none() && self.global_fns.get(&实例键).is_none() {
                            let 旧模块 = self.module.clone();
                            let 旧块 = self.block.clone();
                            self.module = full_mod.clone();
                            self.block = blk.to_string();
                            let r = self.建语句体实例(
                                &实例名, &类型参数, &实参类型, &形参, &返回, &体,
                            );
                            self.module = 旧模块;
                            self.block = 旧块;
                            r?;
                        }
                        调用键 = 实例键.clone();
                        self.fns.get(&实例键).cloned().ok_or_else(|| BuildError {
                            message: format!("block \"{}\" of module \"{}\" has no fn \"{}\"", module, blk, fname),
                            code: prion_error::Code::E304,
                            line: CUR_LINE.with(|c| c.get()),
                            file: String::new(),
                        })?
                    }
                };
                *i = j + 5;
                let mut depth = 1i32;
                let mut inner: Vec<Token> = Vec::new();
                while *i < t.len() {
                    match &t[*i] {
                        Token::Sym("(") => depth += 1,
                        Token::Sym(")") => {
                            depth -= 1;
                            if depth == 0 {
                                *i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    inner.push(t[*i].clone());
                    *i += 1;
                }
                if depth != 0 {
                    return err("cross-module call parentheses are not closed");
                }
                let mut vals = Vec::new();
                for a in split_args(&inner) {
                    vals.push(self.expr(&a)?);
                }
                
                let dc_tmps = self.容器实参深拷(&mut vals, &sig);
                
                if !sig.hidden.is_empty() {
                    vals.extend(self.槽实参(&sig, &调用键, true)?);
                }
                let tmp = self.temp(sig.ret.clone());
                let call_name = if sig.is_outs { fname.to_string() } else { 调用键 };
                self.emit(Stmt::Assign {
                    lhs: Place::local(tmp),
                    rhs: Rvalue::Call { name: call_name, args: vals },
                });
                
                for t in dc_tmps {
                    self.emit(Stmt::Assign {
                        lhs: Place::local(t),
                        rhs: Rvalue::Const(Value::Int(0)),
                    });
                }
                Ok(Value::Local(tmp))
            }
            
            Some(Token::HashColon) => {
                err("cross-block use requires `call` - the form is `call #:block fn \"name\"(...)`")
            }

            Some(Token::Kw(k)) if matches!(t.get(*i + 1), Some(Token::Sym("("))) => {
                let name = (*k).to_string();
                *i += 2;
                let mut depth = 1i32;
                let mut inner: Vec<Token> = Vec::new();
                while *i < t.len() {
                    match &t[*i] {
                        Token::Sym("(") => depth += 1,
                        Token::Sym(")") => {
                            depth -= 1;
                            if depth == 0 {
                                *i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    inner.push(t[*i].clone());
                    *i += 1;
                }
                if depth != 0 {
                    return err("call parentheses are not closed");
                }
                let mut vals = Vec::new();
                
                let pending = if name == "que" || name == "stack" {
                    self.lit_ty.take()
                } else {
                    None
                };
                if let Some(src) = pending.as_ref() {
                    if let Some(elem) = 序列元素类型(&src.0) {
                        self.lit_ty = Some(Ty(format!("list({})", elem)));
                    }
                }
                for a in split_args(&inner) {
                    vals.push(self.expr(&a)?);
                }
                if pending.is_some() {
                    let _ = self.lit_ty.take();
                }
                let tmp_ty = pending.unwrap_or_else(|| builtin_ret_ty(&name));
                let tmp = self.temp(tmp_ty);
                self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Call { name, args: vals } });
                Ok(Value::Local(tmp))
            }
            
            Some(Token::Sym("{")) => {
                let mut depth = 0i32;
                let mut end = None;
                for (k, tk) in t.iter().enumerate().skip(*i) {
                    match tk {
                        Token::Sym("{") => depth += 1,
                        Token::Sym("}") => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let end = end.ok_or_else(|| BuildError { message: "map literal is missing }".into() , code: prion_error::Code::E302 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
                let inner = t[*i + 1..end].to_vec();
                *i = end + 1;
                let mut pairs: Vec<(MapKey, Value)> = Vec::new();
                for seg in split_args(&inner) {
                    
                    let k = match seg.first() {
                        Some(Token::Str(k)) => MapKey::Str(k.to_string()),
                        Some(Token::Int(n)) => MapKey::Int(*n),
                        _ => return err("map literal keys must be string or integer literals"),
                    };
                    if !matches!(seg.get(1), Some(Token::Sym(":"))) {
                        return err("map literal is missing : between key and value");
                    }
                    pairs.push((k, self.expr(&seg[2..])?));
                }
                let lty = self.lit_ty.take().unwrap_or_else(|| Ty("map".into()));
                let tmp = self.temp(lty);
                self.emit(Stmt::Assign {
                    lhs: Place::local(tmp),
                    rhs: Rvalue::MapLit { pairs },
                });
                Ok(Value::Local(tmp))
            }
            
            Some(Token::Sym("[")) => {
                let start = *i;
                let mut depth = 0i32;
                let mut end = None;
                for (k, tk) in t.iter().enumerate().skip(start) {
                    match tk {
                        Token::Sym("[") => depth += 1,
                        Token::Sym("]") => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let end = end.ok_or_else(|| BuildError { message: "list literal is missing ]".into() , code: prion_error::Code::E302 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
                let inner = t[start + 1..end].to_vec();
                *i = end + 1;
                let mut elems = Vec::new();
                for a in split_args(&inner) {
                    let v = self.expr(&a)?;

                    if self.is_container_value(&v) {
                        return err("a list cannot directly hold containers (`[[1], [2]]` is invalid) - to group, use a key-value table (e.g. map(str, list(int))) or struct fields");
                    }
                    elems.push(v);
                }
                let lty = self.lit_ty.take().unwrap_or_else(|| Ty("list".into()));
                let tmp = self.temp(lty);
                self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::ListLit { elems } });
                Ok(Value::Local(tmp))
            }

            
            Some(Token::Kw("val")) => {
                let name = match t.get(*i + 1) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => return err("`val` must be followed by a name"),
                };
                if self.常量值.borrow().contains_key(&name) {
                    return Err(BuildError {
                        message: format!(
                            "\"{}\" is a compile-time constant; read it with `call fix \"{}\"`, not call val",
                            name, name
                        ),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
                *i += 2;
                
                
                let id = match self.names.get(&name) {
                    Some(id) => *id,
                    None => {
                        if let Some(&pid) = self.exposed.get(&name) {
                            let pty = self.f.locals.get(pid).map(|l| l.ty.0.clone()).unwrap_or_default();
                            let vty = Ty(pty.trim_end_matches('*').to_string());
                            let tmp = self.temp(vty.clone());
                            self.emit(Stmt::Assign {
                                lhs: Place::local(tmp),
                                rhs: Rvalue::Use(Place { base: pid, proj: vec![Proj::Deref] }),
                            });
                            tmp
                        } else {
                            return err(format!("val \"{}\" has not been defined", name));
                        }
                    }
                };
                
                let vty = self
                    .f
                    .locals
                    .get(id)
                    .map(|l| l.ty.0.clone())
                    .unwrap_or_default();
                if matches!(t.get(*i), Some(Token::Sym("."))) {
                    
                    return self.field_chain(id, vty, t, i);
                }
                
                if matches!(t.get(*i), Some(Token::Sym("["))) {
                    return self.index_read(id, vty, t, i);
                }
                
                if matches!(t.get(*i), Some(Token::Sym("("))) {
                    let mut depth = 0i32;
                    let mut close = None;
                    for (k, tk) in t.iter().enumerate().skip(*i) {
                        match tk {
                            Token::Sym("(") => depth += 1,
                            Token::Sym(")") => {
                                depth -= 1;
                                if depth == 0 {
                                    close = Some(k);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    let close = close.ok_or_else(|| BuildError { message: "delta parentheses are not closed".into() , code: prion_error::Code::E302 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
                    let op = match t.get(*i + 1) {
                        Some(Token::Sym("+")) => BinOp::Add,
                        Some(Token::Sym("-")) => BinOp::Sub,
                        Some(Token::Sym("*")) => BinOp::Mul,
                        Some(Token::Sym("/")) => BinOp::Div,
                        _ => return err("delta parentheses must start with an operator (e.g. `(+1)`, `(+ expression)`)"),
                    };
                    let rhs = self.expr(&t[*i + 2..close])?;
                    let nv = self.bin(op, Value::Local(id), rhs);
                    *i = close + 1;
                    return Ok(nv);
                }
                Ok(Value::Local(id))
            }
            Some(Token::Kw("fn")) if matches!(t.get(*i + 1), Some(Token::Str(_) | Token::Ident(_)))
                && !matches!(t.get(*i + 2), Some(Token::Sym("("))) =>
            {
                let fname = match t.get(*i + 1) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => return err("`fn` must be followed by a function name"),
                };
                *i += 2;
                let key = format!("{}|{}|{}", self.module, self.block, fname);
                let sig = self
                    .fns
                    .get(&key)
                    .or_else(|| self.global_fns.get(&key))
                    .cloned()
                    .ok_or_else(|| BuildError {
                        message: format!("fn \"{}\" has not been defined yet - define it with add fn first", fname),
                        code: prion_error::Code::E304,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    })?;
                let ps: Vec<String> = sig.params.iter().map(|t| t.0.clone()).collect();
                let tmp = self.temp(Ty(format!("fn({}) {}", ps.join(", "), sig.ret.0)));
                self.emit(Stmt::Assign {
                    lhs: Place::local(tmp),
                    rhs: Rvalue::FnRef { name: key },
                });
                Ok(Value::Local(tmp))
            }
            Some(Token::Ident(n)) => {
                
                if let Some(bid) = self.names.get(n.as_ref()) {
                    if !self.f.locals.get(*bid).map(|l| l.bare_ok).unwrap_or(false) {
                        return err(format!(
                            "bare name \"{}\" - read the val as val \"{}\" (fully explicit: fn and val may share a name, so a bare name cannot resolve ambiguity)",
                            n, n
                        ));
                    }
                }
                
                if matches!(t.get(*i + 1), Some(Token::Sym("["))) {
                    let base = *self
                        .names
                        .get(n.as_ref())
                        .ok_or_else(|| BuildError { message: format!("\"{}\" has not been defined yet", n) , code: prion_error::Code::E304 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
                    let open = *i + 1;
                    let mut depth = 0i32;
                    let mut close = None;
                    for (k, tk) in t.iter().enumerate().skip(open) {
                        match tk {
                            Token::Sym("[") => depth += 1,
                            Token::Sym("]") => {
                                depth -= 1;
                                if depth == 0 {
                                    close = Some(k);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    let close = close.ok_or_else(|| BuildError { message: "index is missing ]".into() , code: prion_error::Code::E302 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
                    let idx_toks = t[open + 1..close].to_vec();
                    *i = close + 1;
                    
                    let base_is_map = self
                        .f
                        .locals
                        .get(base)
                        .map(|l| l.ty.0 == "map" || l.ty.0.starts_with("map("))
                        .unwrap_or(false);
                    if base_is_map {
                        let key = self.expr(&idx_toks)?;
                        

                        let vty = self
                            .f
                            .locals
                            .get(base)
                            .and_then(|l| Self::map_val_ty(&l.ty.0))
                            .unwrap_or_else(|| "int".into());
                        let tmp = self.temp(Ty(vty));
                        self.emit(Stmt::Assign {
                            lhs: Place::local(tmp),
                            rhs: Rvalue::MapGet { map: Value::Local(base), key },
                        });
                        return Ok(Value::Local(tmp));
                    }
                    
                    let base_is_str = self
                        .f
                        .locals
                        .get(base)
                        .map(|l| l.ty.0 == "str")
                        .unwrap_or(false);
                    if base_is_str {
                        if idx_toks.contains(&Token::Sym("~")) {
                            let pos = idx_toks.iter().position(|tk| *tk == Token::Sym("~")).unwrap();
                            let a = self.expr(&idx_toks[..pos])?;
                            let b = self.expr(&idx_toks[pos + 1..])?;
                            let st = self.temp(Ty("str".into()));
                            self.emit(Stmt::Assign {
                                lhs: Place::local(st),
                                rhs: Rvalue::Call {
                                    name: "sub_str".into(),
                                    args: vec![Value::Local(base), a, b],
                                },
                            });
                            return Ok(Value::Local(st));
                        }
                        let idx = self.expr(&idx_toks)?;
                        let st = self.temp(Ty("str".into()));
                        self.emit(Stmt::Assign {
                            lhs: Place::local(st),
                            rhs: Rvalue::Call {
                                name: "nth_char".into(),
                                args: vec![Value::Local(base), idx],
                            },
                        });
                        return Ok(Value::Local(st));
                    }
                    
                    
                    let slice_base_filo = self
                        .f
                        .locals
                        .get(base)
                        .map(|l| l.ty.0.starts_with("que") || l.ty.0.starts_with("stack"))
                        .unwrap_or(false);
                    if slice_base_filo {
                        return err("queues/stacks are push-pop only - slicing applies to lists only");
                    }
                    if idx_toks.contains(&Token::Sym("~")) {
                        let pos = idx_toks.iter().position(|tk| *tk == Token::Sym("~")).unwrap();
                        let a = self.expr(&idx_toks[..pos])?;
                        let b = self.expr(&idx_toks[pos + 1..])?;
                        

                        let base_ty = self
                            .f
                            .locals
                            .get(base)
                            .map(|l| l.ty.0.clone())
                            .unwrap_or_else(|| "list".into());
                        let st = self.temp(Ty(base_ty));
                        self.emit(Stmt::Assign {
                            lhs: Place::local(st),
                            rhs: Rvalue::Call {
                                name: "slice".into(),
                                args: vec![Value::Local(base), a, b],
                            },
                        });
                        return Ok(Value::Local(st));
                    }
                    let base_ty = self
                        .f
                        .locals
                        .get(base)
                        .map(|l| l.ty.0.clone())
                        .unwrap_or_default();
                    let ety = elem_ty_of(&base_ty).unwrap_or_else(|| "int".into());
                    let tmp = self.temp(Ty(ety.clone()));
                    let idx = self.expr(&idx_toks)?;
                    self.emit(Stmt::Assign {
                        lhs: Place::local(tmp),
                        rhs: Rvalue::ListGet { list: Value::Local(base), index: idx },
                    });
                    
                    if matches!(t.get(*i), Some(Token::Sym("."))) {
                        return self.field_chain(tmp, ety, t, i);
                    }
                    return Ok(Value::Local(tmp));
                }

                
                if matches!(t.get(*i + 1), Some(Token::Sym("."))) {
                    let cur = *self
                        .names
                        .get(n.as_ref())
                        .ok_or_else(|| BuildError { message: format!("\"{}\" has not been defined yet", n) , code: prion_error::Code::E304 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
                    let cur_ty = self
                        .f
                        .locals
                        .get(cur)
                        .map(|l| l.ty.0.clone())
                        .unwrap_or_default();
                    *i += 1; 
                    return self.field_chain(cur, cur_ty, t, i);
                }
                
                if matches!(t.get(*i + 1), Some(Token::Sym("(")))
                    && (self.type_idx.contains_key(n.as_ref()) || 泛型判别(n))
                {
                    *i += 2; 
                    let mut depth = 1i32;
                    let mut inner: Vec<Token> = Vec::new();
                    while *i < t.len() {
                        match &t[*i] {
                            Token::Sym("(") => depth += 1,
                            Token::Sym(")") => {
                                depth -= 1;
                                if depth == 0 {
                                    *i += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        inner.push(t[*i].clone());
                        *i += 1;
                    }
                    if depth != 0 {
                        return err("construction parentheses are not closed");
                    }
                    let args = split_args(&inner);
                    
                    let 键 = if 泛型判别(n) {
                        self.反解实例(n, &args)?
                    } else {
                        n.to_string()
                    };
                    let st = self.types[self.type_idx[&键]].clone();
                    if args.len() != st.fields.len() {
                        return err(format!(
                            "type \"{}\" has {} field(s), construction gave {}",
                            st.name,
                            st.fields.len(),
                            args.len()
                        ));
                    }
                    let sid = self.new_local(None, Ty(st.name.clone()), false, false);
                    for (k, a) in args.iter().enumerate() {
                        let v = self.expr(a)?;
                        self.emit(Stmt::Assign {
                            lhs: Place { base: sid, proj: vec![Proj::Field(st.fields[k].0.clone())] },
                            rhs: Rvalue::Const(v),
                        });
                    }
                    return Ok(Value::Local(sid));
                }
                
                const BUILTIN_EXPRS: &[&str] = &[
                    "len", "char_len", "nth_char", "empty", "has", "index", "concat",
                    "code", "char", "que", "stack", "args", "read_file", "keys", "vals", "slice",
                    "read_8", "read_16", "read_32", "read_64",
                    
                    "rnd", "str_to_c", "str_from_c", "save_file", "now", "sqrt",
                ];
                if matches!(t.get(*i + 1), Some(Token::Sym("(")))
                    && BUILTIN_EXPRS.iter().any(|u| *u == n.as_ref())
                {
                    let name = n.clone();
                    *i += 2;
                    let mut depth = 1i32;
                    let mut inner: Vec<Token> = Vec::new();
                    while *i < t.len() {
                        match &t[*i] {
                            Token::Sym("(") => depth += 1,
                            Token::Sym(")") => {
                                depth -= 1;
                                if depth == 0 {
                                    *i += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        inner.push(t[*i].clone());
                        *i += 1;
                    }
                    if depth != 0 {
                        return err("call parentheses are not closed");
                    }
                    let mut vals = Vec::new();
                    for a in split_args(&inner) {
                        vals.push(self.expr(&a)?);
                    }
                    
                    let mut name = name.clone();
                    if name == "index" {
                        let 首参串 = match vals.first() {
                            Some(Value::Local(id)) => self
                                .f
                                .locals
                                .get(*id)
                                .map(|l| l.ty.0 == "str")
                                .unwrap_or(false),
                            Some(Value::Str(_)) => true,
                            _ => false,
                        };
                        if 首参串 {
                            name = "str_index".into();
                        }
                    }
                    
                    let 返型 = if name == "slice" {
                        match vals.first() {
                            Some(Value::Local(id)) => self
                                .f
                                .locals
                                .get(*id)
                                .map(|l| l.ty.0.clone())
                                .unwrap_or_else(|| "int".into()),
                            _ => "int".into(),
                        }
                    } else {
                        builtin_ret_ty(&name).0
                    };
                    let tmp = self.temp(Ty(返型));
                    self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Call { name: name.to_string(), args: vals } });
                    return Ok(Value::Local(tmp));
                }
                
                if matches!(t.get(*i + 1), Some(Token::Sym("("))) && !self.sum_variants.contains_key(n.as_ref()) {
                    let key = format!("{}|{}", self.block, n);
                    let outs_sig = self
                        .fns
                        .get(&key)
                        .filter(|s| s.is_outs)
                        .cloned()
                        .or_else(|| self.global_fns.get(&key).filter(|s| s.is_outs).cloned());
                    if let Some(sig) = outs_sig {
                        *i += 2;
                        let mut depth = 1i32;
                        let mut inner: Vec<Token> = Vec::new();
                        while *i < t.len() {
                            match &t[*i] {
                                Token::Sym("(") => depth += 1,
                                Token::Sym(")") => {
                                    depth -= 1;
                                    if depth == 0 {
                                        *i += 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            inner.push(t[*i].clone());
                            *i += 1;
                        }
                        if depth != 0 {
                            return err("call parentheses are not closed");
                        }
                        let mut vals = Vec::new();
                        for a in split_args(&inner) {
                            vals.push(self.expr(&a)?);
                        }
                        
                        let tmp = self.temp(sig.ret.clone());
                        let call_name = if n.as_ref() == "join" { "prion_join".to_string() } else { n.to_string() };
                        self.emit(Stmt::Assign {
                            lhs: Place::local(tmp),
                            rhs: Rvalue::Call { name: call_name, args: vals },
                        });
                        return Ok(Value::Local(tmp));
                    }
                    
                    
                    if matches!(t.get(*i + 1), Some(Token::Sym("("))) {
                        *i += 2;
                        let mut depth = 1i32;
                        let mut inner: Vec<Token> = Vec::new();
                        while *i < t.len() {
                            match &t[*i] {
                                Token::Sym("(") => depth += 1,
                                Token::Sym(")") => {
                                    depth -= 1;
                                    if depth == 0 {
                                        *i += 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            inner.push(t[*i].clone());
                            *i += 1;
                        }
                        if depth != 0 {
                            return err("call parentheses are not closed");
                        }
                        let mut vals = Vec::new();
                        for a in split_args(&inner) {
                            vals.push(self.expr(&a)?);
                        }
                        
                        
                        
                        
                        const BUILTIN_VALUE_NAMES: &[&str] = &[
                            "len", "char_len", "nth_char", "index", "empty", "has", "pop", "run", "remove",
                            
                            "code", "char", "concat", "read_file", "str", "join",
                        ];
                        if BUILTIN_VALUE_NAMES.iter().any(|u| *u == n.as_ref()) {
                            let mut n = n.clone();
                            let mut join_all: Option<String> = None;
                            if n == "join" {
                                n = "prion_join".into();
                                let 内 = match vals.first() {
                                    Some(Value::Local(id)) => self
                                        .f
                                        .locals
                                        .get(*id)
                                        .and_then(|l| token容器载荷(&l.ty.0)),
                                    _ => None,
                                };
                                if let Some(内) = 内 {
                                    n = "prion_join_all".into();
                                    join_all = Some(内);
                                }
                            }
                            
                            if n == "index" {
                                let 首参串 = match vals.first() {
                                    Some(Value::Local(id)) => self
                                        .f
                                        .locals
                                        .get(*id)
                                        .map(|l| l.ty.0 == "str")
                                        .unwrap_or(false),
                                    Some(Value::Str(_)) => true,
                                    _ => false,
                                };
                                if 首参串 {
                                    n = "str_index".into();
                                }
                            }
                            let mut rt = builtin_ret_ty(&n);
                            if let Some(内) = &join_all {
                                rt = Ty(format!("list({})", 内));
                            }
                            
                            if n == "pop" {
                                if let Some(Value::Local(bid)) = vals.first() {
                                    if let Some(l) = self.f.locals.get(*bid) {
                                        rt = Ty(elem_ty_of(&l.ty.0).unwrap_or_else(|| "int".into()));
                                    }
                                }
                            }
                            let tmp = self.temp(rt);
                            self.emit(Stmt::Assign {
                                lhs: Place::local(tmp),
                                rhs: Rvalue::Call { name: n.to_string(), args: vals },
                            });
                            return Ok(Value::Local(tmp));
                        }
                    }
                    return err(format!(
                        "calls require `call` - the form is `call fn \"{}\"(...)` (same-block calls require it too)",
                        n
                    ));
                }
                
                if matches!(t.get(*i + 1), Some(Token::Sym("("))) {
                    if let Some((模板型, tag0, 模板载荷)) = self.sum_variants.get(n.as_ref()).cloned() {
                        
                        let (tyname, tag, ftypes) = match self.期望类型.clone() {
                            Some(t) if t != 模板型 => {
                                let 实例载荷 = self
                                    .type_idx
                                    .get(&t)
                                    .and_then(|i| self.types.get(*i))
                                    .and_then(|st| {
                                        st.variants
                                            .iter()
                                            .find(|v| *n == v.name)
                                            .map(|v| v.fields.iter().map(|(_, ty)| ty.clone()).collect())
                                    });
                                match 实例载荷 {
                                    Some(载) => (t, tag0, 载),
                                    None => (模板型, tag0, 模板载荷),
                                }
                            }
                            _ => (模板型, tag0, 模板载荷),
                        };
                        *i += 2;
                        let mut depth = 1i32;
                        let mut inner: Vec<Token> = Vec::new();
                        while *i < t.len() {
                            match &t[*i] {
                                Token::Sym("(") => depth += 1,
                                Token::Sym(")") => {
                                    depth -= 1;
                                    if depth == 0 {
                                        *i += 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            inner.push(t[*i].clone());
                            *i += 1;
                        }
                        if depth != 0 {
                            return err("variant construction parentheses are not closed");
                        }
                        let mut vals = Vec::new();
                        for a in split_args(&inner) {
                            vals.push(self.expr(&a)?);
                        }
                        let tmp = self.temp(Ty(tyname.clone()));
                        self.emit(Stmt::Assign {
                            lhs: Place::local(tmp),
                            rhs: Rvalue::VariantLit { ty: tyname, tag, args: vals, tys: ftypes },
                        });
                        return Ok(Value::Local(tmp));
                    }
                }
                
                if let Some(&pid) = self.exposed.get(n.as_ref()) {
                    let pty = self.f.locals.get(pid).map(|l| l.ty.0.clone()).unwrap_or_default();
                    let inner = pty.strip_suffix('*').unwrap_or("int").to_string();
                    let tmp = self.temp(Ty(inner));
                    self.emit(Stmt::Assign {
                        lhs: Place::local(tmp),
                        rhs: Rvalue::Use(Place { base: pid, proj: vec![Proj::Deref] }),
                    });
                    return Ok(Value::Local(tmp));
                }
                *i += 1;
                match self.names.get(n.as_ref()) {
                    Some(id) => Ok(Value::Local(*id)),
                    None => err(format!("\"{}\" has not been defined yet - define it with add val first (fn={})", n, self.f.name)),
                }
            }
            
            Some(Token::Kw("match")) => self.match_expr(t, i),
            Some(Token::Kw("if")) => self.if_expr(t, i),
            other => err(format!("unrecognized token in expression: {:?}", other)),
        }
    }


    
    fn 行右值(&mut self, exprs: &[Expr], 段: &[Token], eq: usize) -> Result<Value, BuildError> {
        match exprs.first() {
            
            Some(e) => match &e.kind {
                prion_parser::expr::ExprKind::Call { verb: prion_parser::expr::Verb::Delta, .. } => {
                    self.expr(&段[eq + 1..])
                }
                _ => self.expr_tree(e),
            },
            None => self.expr(&段[eq + 1..]),
        }
    }

    
    fn 行实参(&mut self, exprs: &[Expr], inner: &[Token]) -> Result<Vec<Value>, BuildError> {
        if let Some(prion_parser::expr::ExprKind::Call { args, .. }) = exprs.first().map(|e| &e.kind) {
            let mut vals = Vec::new();
            for a in args {
                vals.push(self.expr_tree(a)?);
            }
            return Ok(vals);
        }
        let mut vals = Vec::new();
        for a in split_args(inner) {
            vals.push(self.expr(&a)?);
        }
        Ok(vals)
    }

    fn line(&mut self, kind: &prion_parser::LineKind, 段: &[Token], exprs: &[Expr]) -> Result<(), BuildError> {
        if 段.is_empty() {
            return Ok(());
        }

        
        let 原始段 = 段;
        let 规范;
        let 段: &[Token] = if matches!(kind, prion_parser::LineKind::AddVal(_)) {
            规范 = prion_parser::建val规范(段);
            &规范
        } else {
            段
        };

        
        if self.语句体返回 && matches!(kind, prion_parser::LineKind::Give(_)) {
            let v = match exprs.first() {
                Some(e) => self.expr_tree(e)?,
                None => self.expr(&段[1..])?,
            };
            self.set_exit(Exit::Return(Some(v)));
            let 死块 = self.nb();
            self.switch_to(死块);
            self.set_exit(Exit::Unreachable);
            return Ok(());
        }

        
        
        if matches!(kind, prion_parser::LineKind::AddVal(_))
        {
            if let (Some(Token::Ident(名)), Some(eq)) =
                (段.get(3), 段.iter().position(|t| *t == Token::Sym("=")))
            {
                if 泛型判别(名) && matches!(段.get(4), Some(Token::Sym("("))) {
                    if let Some(闭) = 段[5..eq].iter().position(|t| *t == Token::Sym(")")) {
                        let 实参: Vec<String> = 段[5..5 + 闭]
                            .split(|t| *t == Token::Sym(","))
                            .map(|段| {
                                段.iter()
                                    .filter_map(|t| match t {
                                        Token::Ident(x) => Some(x.to_string()),
                                        Token::Kw(k) => Some((*k).to_string().into()),
                                        Token::Sym("*") => Some("*".to_string().into()),
                                        _ => None,
                                    })
                                    .collect::<String>()
                            })
                            .filter(|s| !s.is_empty())
                            .collect();
                        let 实例名 = format!("{}({})", 名, 实参.join(", "));
                        let 右值同模板 = matches!(段.get(eq + 1), Some(Token::Ident(x)) if x == 名)
                            && matches!(段.get(eq + 2), Some(Token::Sym("(")));
                        if 右值同模板 {
                            let mut 新: Vec<Token> = 段.to_vec();
                            新[eq + 1] = Token::Ident(实例名.into());
                            return self.line(kind, &新, exprs);
                        }
                    }
                }
            }
        }

        
        if matches!(kind, prion_parser::LineKind::AddType(_)) {
            return Ok(());
        }

        
        
        if matches!(kind, prion_parser::LineKind::AddHide(_))
        {
            return Ok(());
        }

        
        

        let is_rebind = matches!(段.first(), Some(Token::Kw("fn")));
        let is_hide_fn = matches!(段.first(), Some(Token::Kw("add")))
            && matches!(段.get(1), Some(Token::Kw("hide")))
            && matches!(段.get(2), Some(Token::Kw("fn")));
        if matches!(kind, prion_parser::LineKind::AddFn(_)) {
            let name_at = if is_rebind {
                1
            } else if is_hide_fn {
                3
            } else {
                2
            };
            let name = match 段.get(name_at) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err(format!("expected a name after fn: {:?}", 段)),
            };
            
            if matches!(段.get(name_at + 1), Some(Token::Sym("("))) {
                if let Some(类型参数) = 读类型参数表(段, name_at + 1) {
                    let 类型参数表止 = 括号止(段, name_at + 1)
                        .ok_or_else(|| BuildError {
                            message: "the type parameter list of the generic fn is not closed".into(),
                            code: prion_error::Code::E301,
                            line: CUR_LINE.with(|c| c.get()),
                            file: String::new(),
                        })?
                        + 1;
                    let (形参, 帕止) = parse_params(&段[类型参数表止..])?;
                    let (ret, _限, _体起) = parse_type_ref(&段[类型参数表止..], 帕止)?;
                    let 体 = match 段.iter().position(|t| *t == Token::Sym("=")) {
                        Some(eq) => 段[eq + 1..].to_vec(),
                        None => Vec::new(),
                    };
                    泛型fn表.with(|g| {
                        g.borrow_mut().insert(
                            name.to_string(),
                            (类型参数, 形参, ret, 体, self.module.clone(), self.block.clone()),
                        );
                    });
                    return Ok(());
                }
            }
            let (params, after) = parse_params(段)?;
            
            if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                let 体 = 段[eq + 1..].to_vec();
                let 形参名: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
                常量fn表.with(|t| {
                    t.borrow_mut().insert(name.to_string(), (形参名, 体));
                });
            }
            
            let (ret, _限定, _) = parse_type_ref(段, after)
                .map_err(|_| BuildError {
                    message: "add fn is missing a return type (boundary types on functions are mandatory)".into(),
                    code: prion_error::Code::E301,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
            
            
            let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) else {
                return Ok(());
            };

            
            let mut fb = Builder::new(
                format!("{}|{}|{}", self.module, self.block, name),
                self.f.file.clone(),
                false, 
                false,
                false,
                false,
                false, 
                Vec::new(),
                self.block.clone(),
                self.module.clone(),
                std::rc::Rc::clone(&self.types),
                std::rc::Rc::clone(&self.type_idx),
                std::rc::Rc::clone(&self.mod_by_tail),
                std::rc::Rc::clone(&self.global_fns),
                std::rc::Rc::clone(&self.cross_vals),
                std::rc::Rc::clone(&self.sum_variants),
                std::rc::Rc::clone(&self.pub_blocks),
                std::rc::Rc::clone(&self.hidden_items),
                std::rc::Rc::clone(&self.block_types),
                std::rc::Rc::clone(&self.常量值),
            );
            fb.cur_col = self.cur_col;
            fb.cur_len = self.cur_len;
            for (pname, pty) in &params {
                let id = fb.new_local(Some(pname.clone()), pty.clone(), true, true);
                fb.names.insert(pname.clone(), id);
            }
            
            let hidden = 注入槽参数_prep(&mut fb, &format!("{}|{}|{}", self.module, self.block, name));
            let sig = FnSig {
                params: params.iter().map(|(_, t)| t.clone()).collect(),
                ret: ret.clone(),
                is_outs: false,
                hidden,
            };
            self.fns.insert(
                format!("{}|{}|{}", self.module, self.block, name),
                sig,
            );
            fb.期望类型 = Some(ret.0.clone());
            let v = fb.expr(&段[eq + 1..])?;
            
            fb.set_exit(Exit::Return(Some(v)));
            let mut f = fb.finish()?;
            f.ret = Some(ret.clone());

            self.extra.push(f);
            return Ok(());
        }


        
        if matches!(kind, prion_parser::LineKind::AddOutsFn(_))
        {
            let name = match 段.get(3) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err("expected a name after add outs fn"),
            };
            if 段.iter().any(|t| *t == Token::Sym("=")) {
                return err("an external function declaration has no body - the body lives in C");
            }
            let (params, after) = parse_params(段)?;
            let ret = match 段.get(after) {
                Some(Token::Kw(t)) => Ty((*t).to_string()),
                _ => return err("add outs fn is missing a return type (external function boundary types are mandatory)"),
            };
            let param_tys: Vec<Ty> = params.iter().map(|(_, t)| t.clone()).collect();
            self.fns.insert(
                format!("{}|{}", self.block, name.clone()),
                FnSig { params: param_tys.clone(), ret: ret.clone(), is_outs: true, hidden: Vec::new() },
            );
            self.externs.push(ExternFn { name: name.to_string(), params: param_tys, ret });
            return Ok(());
        }

        
        if matches!(kind, prion_parser::LineKind::CallFn(_)) {
            let name = match 段.get(2) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err(format!("expected a name after call fn: {:?}", 段)),
            };
            let key = format!("{}|{}|{}", self.module, self.block, name);
            let sig = self
                .fns
                .get(&key)
                .cloned()
                .or_else(|| self.global_fns.get(&key).cloned())
                .ok_or_else(|| BuildError {
                    message: format!("fn \"{}\" has not been defined yet - define it with add fn first", name),
                    code: prion_error::Code::E304,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
            let inner = between_parens(段)?;
            let mut vals = self.行实参(exprs, &inner)?;
            
            if !sig.hidden.is_empty() {
                let caller_is_fn = self.f.name.contains('#');
                vals.extend(self.槽实参(&sig, &key, caller_is_fn)?);
            }
            let t = self.temp(sig.ret.clone());
            let call_name = if sig.is_outs { name.to_string() } else { key };
            self.emit(Stmt::Assign { lhs: Place::local(t), rhs: Rvalue::Call { name: call_name, args: vals } });
            return Ok(());
        }

        
        
        
        
        let (blk, blk_mod) = if matches!(kind, prion_parser::LineKind::CallBlock(_))
            && matches!(段.get(1), Some(Token::HashColon))
            && 段.get(3).is_none()
        {
            let blk = match 段.get(2) {
                Some(Token::Ident(n)) => n.to_string(),
                Some(Token::Kw(k)) => (*k).to_string().into(),
                _ => return err("`call #:` must be followed by a block name"),
            };
            (blk, self.module.clone())
        } else if matches!(kind, prion_parser::LineKind::CallBlock(_))
            && matches!(段.get(1), Some(Token::HashHashColon))
            && !is_take_form(段) 
        {
            let mut m = String::new();
            let mut j = 2usize;
            while let Some(tok) = 段.get(j) {
                match tok {
                    Token::Ident(n) | Token::Str(n) => {
                        if !m.is_empty() {
                            m.push('.');
                        }
                        m.push_str(n);
                        j += 1;
                        if matches!(段.get(j), Some(Token::Sym("."))) {
                            j += 1;
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            if m.is_empty() || !matches!(段.get(j), Some(Token::HashColon)) {
                return err("`##:` must be followed by a module name then `#:block` - e.g. `##:frontend.tokenizer #:split`");
            }
            let blk = match 段.get(j + 1) {
                Some(Token::Ident(n)) => n.to_string(),
                Some(Token::Kw(k)) => (*k).to_string().into(),
                _ => return err("`##:module #:` must be followed by a block name"),
            };
            
            if 段.get(j + 2).is_some() {
                return err("to run a whole block across modules write `call ##:module #:block`; take fn / val with their respective forms");
            }
            let full = if self.mod_has(&m) {
                m.clone()
            } else {
                match self.resolve_module(&m)? {
                    Some(f) => f,
                    None => return err(format!("module \"{}\" not found", m)),
                }            };
            (blk, full)
        } else {
            (String::new(), String::new())
        };
        if !blk.is_empty() {

            self.check_pub(&blk_mod, &blk)?;
            let mut args: Vec<Value> = Vec::new();
            if let Some(vals) = self.cross_vals.get(&format!("{}|{}", blk_mod, blk)).cloned() {
                for (vname, vty) in vals {
                    let slot_key = format!("{}|{}#{}", blk_mod, blk, vname);
                    let sid = match self.names.get(&slot_key) {
                        Some(&id) => id,
                        None => {
                            let id = self.new_local(Some(slot_key.clone()), vty.clone(), false, false);
                            self.names.insert(slot_key, id);
                            id
                        }
                    };
                    let ptmp = self.temp(Ty(format!("{}*", vty.0)));
                    self.emit(Stmt::Assign {
                        lhs: Place::local(ptmp),
                        rhs: Rvalue::Ref(Place::local(sid)),
                    });
                    args.push(Value::Local(ptmp));
                }
            }
            self.emit(Stmt::Call {
                name: format!("{}|{}", blk_mod, blk),
                args,
            });
            return Ok(());
        }


        if matches!(kind, prion_parser::LineKind::CallFn(_))
        {
            self.expr(段)?;
            return Ok(());
        }


        if matches!(kind, prion_parser::LineKind::CallBlock(_))
        {
            let vname = match 段.get(2) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err("`call val` must be followed by a val name"),
            };
            if !matches!(段.get(3), Some(Token::Sym("="))) {
                return err(
                    "`call val \"name\"` as a statement must be followed by `= newvalue`; for a pure read put it in an expression (`= call val \"name\"`)",
                );
            }
            let id = *self.names.get(&vname).ok_or_else(|| BuildError {
                message: format!("this block has no val \"{}\" - a same-block val write targets this block's", vname),
                code: prion_error::Code::E304,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            })?;
            let v = self.expr(&段[4..])?;
            self.emit(Stmt::Assign { lhs: Place::local(id), rhs: Rvalue::Const(v) });
            return Ok(());
        }

        
        if matches!(kind, prion_parser::LineKind::CallBlock(_))
            && (matches!(段.get(1), Some(Token::HashColon))
                || matches!(段.get(1), Some(Token::HashHashColon)))
        {
            
            let (blk_mod, blk_idx) = if matches!(段.get(1), Some(Token::HashHashColon)) {
                let mut m = String::new();
                let mut j = 2usize;
                while let Some(tok) = 段.get(j) {
                    match tok {
                        Token::Ident(n) | Token::Str(n) => {
                            if !m.is_empty() {
                                m.push('.');
                            }
                            m.push_str(n);
                            j += 1;
                            if matches!(段.get(j), Some(Token::Sym("."))) {
                                j += 1;
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
                if m.is_empty() || !matches!(段.get(j), Some(Token::HashColon)) {
                    return err("`##:` must be followed by a module name then `#:block` - e.g. `##:frontend.tokenizer #:split`");
                }
                let full = if self.mod_has(&m) {
                    m.clone()
                } else {
                    match self.resolve_module(&m)? {
                        Some(f) => f,
                        None => return err(format!("module \"{}\" not found", m)),
                    }
                };
                (full, j + 1)
            } else {
                (self.module.clone(), 2usize)
            };
            
            if matches!(段.get(blk_idx + 1), Some(Token::Kw("fn"))) {
                self.expr(段)?;
                return Ok(());
            }
            if !matches!(段.get(blk_idx + 1), Some(Token::Kw("val"))) {
                return err("writing a val across blocks requires `#:block val \"name\"`");
            }
            let blk = match 段.get(blk_idx) {
                Some(Token::Ident(n)) => n.to_string(),
                Some(Token::Kw(k)) => (*k).to_string().into(),
                _ => return err("`call #:` must be followed by a block name"),
            };
            let vname = match 段.get(blk_idx + 2) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err("`#:block val` must be followed by a val name"),
            };

            self.check_pub(&blk_mod, &blk)?;
            let slot_key = format!("{}|{}#{}", blk_mod, blk, vname);
            let sid = match self.槽或建(&slot_key) {
                Some(id) => id,
                None => {
                    return Err(BuildError {
                        message: format!(
                            "val \"{}\" of block \"{}\" does not exist - define it with add val first; for an sle block, call #:{} first too",
                            blk, vname, blk
                        ),
                        code: prion_error::Code::E309,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    })
                }
            };
            if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                let v = self.行右值(exprs, 段, eq)?;
                self.emit(Stmt::Assign { lhs: Place::local(sid), rhs: Rvalue::Const(v) });
            }
            return Ok(());
        }

        
        
        if matches!(kind, prion_parser::LineKind::Other(_))
            && matches!(段.first(), Some(Token::Ident(_)))
            && matches!(段.get(1), Some(Token::Sym("[")))
        {
            return Err(BuildError {
                message: "the index write target must be explicit - write `val \"name\"[index] = value`".into(),
                code: prion_error::Code::E303,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            });
        }
        let val_lhs = matches!(kind, prion_parser::LineKind::Assign(_))
            && matches!(段.get(2), Some(Token::Sym("[")));
        if val_lhs {
            let (base_name, open) = {
                let n = match 段.get(1) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => unreachable!(),
                };
                (n, 2usize) 
            };
            let base = *self
                .names
                .get(&base_name)
                .ok_or_else(|| BuildError { message: format!("\"{}\" has not been defined yet", base_name) , code: prion_error::Code::E304 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
            let mut depth = 0i32;
            let mut close = None;
            for (k, tk) in 段.iter().enumerate().skip(open) {
                match tk {
                    Token::Sym("[") => depth += 1,
                    Token::Sym("]") => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(k);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let close = close.ok_or_else(|| BuildError { message: "index is missing ]".into() , code: prion_error::Code::E302 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
            let eq = 段
                .iter()
                .position(|t| *t == Token::Sym("="))
                .ok_or_else(|| BuildError { message: "index write is missing = expression".into() , code: prion_error::Code::E301 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
            let idx = self.expr(&段[open + 1..close])?;
            
            if matches!(段.get(close + 1), Some(Token::Sym("."))) {
                let field = match 段.get(close + 2) {
                    Some(Token::Ident(f)) | Some(Token::Str(f)) => f.to_string(),
                    _ => return err("the `.` after an index must be followed by a field name"),
                };
                let val = self.行右值(exprs, 段, eq)?;
                let base_ty = self
                    .f
                    .locals
                    .get(base)
                    .map(|l| l.ty.0.clone())
                    .unwrap_or_default();
                let ety = elem_ty_of(&base_ty).unwrap_or_else(|| "int".into());
                let h = self.temp(Ty(ety.clone()));
                self.emit(Stmt::Assign {
                    lhs: Place::local(h),
                    rhs: Rvalue::ListGet { list: Value::Local(base), index: idx.clone() },
                });
                
                if ety.ends_with('*') {
                    let c = self.bin(BinOp::Ne, Value::Local(h), Value::Null);
                    self.emit(Stmt::Assert {
                        cond: c,
                        msg: "null reference - the handle is null, cannot write a field".into(),
                        fix: "check for null first (x != null) before writing a field".into(),
                    });
                }
                self.emit(Stmt::Assign {
                    lhs: Place { base: h, proj: vec![Proj::Field(field.to_string())] },
                    rhs: Rvalue::Const(val),
                });
                
                let 按值结构 = !ety.ends_with('*')
                    && ety != "str"
                    && self.types.iter().any(|t| t.name == ety);
                if 按值结构 {
                    self.emit(Stmt::ListSet {
                        list: Value::Local(base),
                        index: idx,
                        value: Value::Local(h),
                    });
                }
                return Ok(());
            }
            let v = self.行右值(exprs, 段, eq)?;
            
            let base_is_map = self
                .f
                .locals
                .get(base)
                .map(|l| l.ty.0 == "map" || l.ty.0.starts_with("map("))
                .unwrap_or(false);
            if base_is_map {
                self.emit(Stmt::MapSet {
                    map: Value::Local(base),
                    key: idx,
                    value: v,
                });
                return Ok(());
            }

            let base_is_filo = self
                .f
                .locals
                .get(base)
                .map(|l| l.ty.0.starts_with("que") || l.ty.0.starts_with("stack"))
                .unwrap_or(false);
            if base_is_filo {
                return err("queues/stacks are push-pop only - index assignment applies to lists only");
            }
            self.emit(Stmt::ListSet {
                list: Value::Local(base),
                index: idx,
                value: v,
            });
            return Ok(());
        }

        
        
        if matches!(kind, prion_parser::LineKind::Other(_))
            && matches!(段.first(), Some(Token::Ident(_)))
            && matches!(段.get(1), Some(Token::Sym(".")))
        {
            return Err(BuildError {
                message: "the field write target must be explicit - write `val \"name\".field = value`".into(),
                code: prion_error::Code::E303,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            });
        }
        let 字段写名位 = match 段.first() {
            Some(Token::Kw("val")) if matches!(段.get(2), Some(Token::Sym("."))) => Some(1usize),
            _ => None,
        };
        if let Some(ni) = 字段写名位 {
            let base_name = match 段.get(ni) {
                Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
                _ => return err("the field write target must be a name"),
            };
            let base = *self
                .names
                .get(&base_name)
                .ok_or_else(|| BuildError { message: format!("\"{}\" has not been defined yet", base_name) , code: prion_error::Code::E304 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
            let field = match 段.get(ni + 2) {
                Some(Token::Ident(f)) | Some(Token::Str(f)) => f.to_string(),
                _ => return err("field access must be followed by a field name"),
            };
            
            let w_ty = self
                .f
                .locals
                .get(base)
                .map(|l| l.ty.0.trim_end_matches('*').to_string())
                .unwrap_or_default();
            if !self.type_idx.contains_key(&w_ty) {
                return Err(BuildError {
                    message: format!(
                        "{} has no fields - field write (.name = value) only applies to product types defined with add type",
                        w_ty
                    ),
                    code: prion_error::Code::E303,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                });
            }
            let eq = 段
                .iter()
                .position(|t| *t == Token::Sym("="))
                .ok_or_else(|| BuildError { message: "field write is missing = expression".into() , code: prion_error::Code::E301 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
            let v = self.行右值(exprs, 段, eq)?;
            self.emit(Stmt::Assign {
                lhs: Place { base, proj: vec![Proj::Field(field.to_string())] },
                rhs: Rvalue::Const(v),
            });
            return Ok(());
        }

        

        if matches!(kind, prion_parser::LineKind::Assign(_))
            && matches!(段.get(2), Some(Token::Sym("(")))
        {
            let vname = match 段.get(1) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err("`val` must be followed by a name"),
            };
            let id = *self.names.get(&vname).ok_or_else(|| {
                BuildError {
                message: format!("val \"{}\" has not been defined yet - define it with add val first (fn={})", vname, self.f.name),
                code: prion_error::Code::E304,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            }
            })?;
            
            let mut depth = 0i32;
            let mut close = None;
            for (k, tk) in 段.iter().enumerate().skip(2) {
                match tk {
                    Token::Sym("(") => depth += 1,
                    Token::Sym(")") => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(k);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let close = close.ok_or_else(|| BuildError { message: "delta parentheses are not closed".into() , code: prion_error::Code::E302 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
            let op = match 段.get(3) {
                Some(Token::Sym("+")) => BinOp::Add,
                Some(Token::Sym("-")) => BinOp::Sub,
                Some(Token::Sym("*")) => BinOp::Mul,
                Some(Token::Sym("/")) => BinOp::Div,
                _ => return err("delta parentheses must start with an operator (e.g. `(+1)`, `(+ expression)`)"),
            };
            let rhs = self.expr(&段[4..close])?;
            let nv = self.bin(op, Value::Local(id), rhs);
            let store = match nv {
                Value::Local(src) => Rvalue::Use(Place { base: src, proj: vec![] }),
                other => Rvalue::Const(other),
            };
            self.emit(Stmt::Assign { lhs: Place::local(id), rhs: store });
            return Ok(());
        }

        


        
        
        if 段.first().map(prion_lexer::asm::是asm词).unwrap_or(false)
            && matches!(段.get(1), Some(Token::Sym("(")))
        {
            let 内区 = prion_lexer::asm::括号内(段, 1)
                .ok_or_else(|| BuildError {
                    message: "asm parentheses are not closed".into(),
                    code: prion_error::Code::E301,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?
                .to_vec();
            let form = self.建asm(&内区)?;
            self.emit(Stmt::Asm(form));
            return Ok(());
        }

        
        if matches!(kind, prion_parser::LineKind::BareCall(_)) {
            let name = match 段.first() {
                Some(Token::Ident(n)) => n.to_string(),
                _ => unreachable!(),
            };
            let inner = between_parens(段)?;
            let vals = self.行实参(exprs, &inner)?;

            if (name == "push" && vals.len() >= 2 && self.is_container_value(&vals[1]))
                || (name == "insert" && vals.len() >= 3 && self.is_container_value(&vals[2]))
            {
                return err("a container cannot hold a container - a value inserted into a list cannot be a container; to group, use a key-value table (e.g. map(str, list(int))) or struct fields");
            }
            self.emit(Stmt::Call { name: name.to_string(), args: vals });
            return Ok(());
        }

        
        if matches!(kind, prion_parser::LineKind::BareCall(_))
            && matches!(段.first(), Some(Token::Ident(n)) if n == "print")
        {
            let inner = between_parens(段)?;
            let vals = self.行实参(exprs, &inner)?;
            self.emit(Stmt::Call { name: "print".into(), args: vals });
            return Ok(());
        }

        
        if matches!(kind, prion_parser::LineKind::AddVal(_))
        {
            let name = match 段.get(2) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err(format!("expected a name after add val: {:?}", 段)),
            };
            let 可空前缀 = matches!(段.get(3), Some(Token::Sym("?")));
            let explicit_ty: Option<Ty> = if 可空前缀 {
                match 段.get(4) {
                    Some(Token::Kw(t)) => Some(Ty(format!("?{}", t))),
                    Some(Token::Ident(n)) | Some(Token::Str(n)) => Some(Ty(format!("?{}", n))),
                    _ => None,
                }
            } else { match container_ty_at(段, 3) {
                Some(t) => Some(Ty(t)),
                None => match 段.get(3) {
                    Some(Token::Kw(t)) => {
                        if *t == "fn" && matches!(段.get(4), Some(Token::Sym("("))) {
                            match parse_type_ref(段, 3) {
                                Ok((ty, _, _)) => Some(ty),
                                Err(_) => Some(Ty((*t).to_string())),
                            }
                        } else {
                            Some(Ty((*t).to_string()))
                        }
                    }
                    
                    
                    Some(Token::Ident(n)) | Some(Token::Str(n)) => {
                        if matches!(段.get(4), Some(Token::Sym("("))) {
                            match parse_type_ref(段, 3) {
                                Ok((t, _, _)) => Some(t),
                                Err(_) => Some(Ty(n.to_string())),
                            }
                        } else {
                            Some(Ty(if matches!(段.get(4), Some(Token::Sym("*"))) {
                                format!("{}*", n)
                            } else {
                                n.to_string()
                            }))
                        }
                    }

                    
                    Some(Token::HashColon) | Some(Token::HashHashColon) => {
                        let cross_mod = matches!(段.get(3), Some(Token::HashHashColon));
                        
                        let (blk_mod, colon_at) = if cross_mod {
                            let mut m = String::new();
                            let mut j = 4usize;
                            while let Some(tok) = 段.get(j) {
                                match tok {
                                    Token::Ident(n) | Token::Str(n) => {
                                        if !m.is_empty() { m.push('.'); }
                                        m.push_str(n);
                                        j += 1;
                                        if matches!(段.get(j), Some(Token::Sym("."))) { j += 1; } else { break; }
                                    }
                                    _ => break,
                                }
                            }
                            if m.is_empty() || !matches!(段.get(j), Some(Token::HashColon)) {
                                return err("`##:` must be followed by a module name then `#:block` - e.g. `##:frontend.tokenizer #:point type \"node\"`");
                            }
                            let full = if self.mod_has(&m) { m.clone() } else {
                                match self.resolve_module(&m)? {
                                    Some(f) => f,
                                    None => return err(format!("module \"{}\" not found", m)),
                                }
                            };
                            (full, j)
                        } else {
                            (self.module.clone(), 3usize)
                        };
                        let blk = match 段.get(colon_at + 1) {
                            Some(Token::Ident(n)) => n.to_string(),
                            Some(Token::Kw(k)) => (*k).to_string().into(),
                            _ => return err("`#:` must be followed by a block name"),
                        };
                        if !matches!(段.get(colon_at + 2), Some(Token::Kw("type"))) {
                            return err("to take a type across blocks write `#:block type \"name\"`");
                        }
                        let tname = match 段.get(colon_at + 3) {
                            Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                            _ => return err("`#:block type` must be followed by a type name"),
                        };
                        
                        if cross_mod {
                            self.check_pub(&blk_mod, &blk)?;
                            if self.hidden_items.contains(&format!("{}|{}|type|{}", blk_mod, blk, tname)) {
                                return Err(BuildError {
                                    message: format!("type \"{}\" is hidden in block \"{}\" (module {}) - it is not part of its public surface", tname, blk, blk_mod),
                                    code: prion_error::Code::E201,
                                    line: CUR_LINE.with(|c| c.get()),
                                    file: String::new(),
                                });
                            }
                        }
                        let has = self
                            .block_types
                            .get(&format!("{}|{}", blk_mod, blk))
                            .map(|m| m.contains(&tname))
                            .unwrap_or(false);
                        if !has {
                            return err(format!("block \"{}\" has no type \"{}\"", blk, tname));
                        }
                        
                        Some(Ty(tname.to_string()))
                    }
                    
                    _ => None,
                },
            } };
            let explicit_ty = explicit_ty.map(|t| Ty(代换类型(&t.0)));
            let eq0 = 段.iter().position(|t| *t == Token::Sym("="));
            let pre = match (&explicit_ty, eq0) {
                (None, Some(e)) => Some(self.expr(&段[e + 1..])?),
                _ => None,
            };
            let ty = match explicit_ty {
                Some(t) => t,
                None => self.ty_of_value(pre.clone().expect("a missing type must have a value")),
            };
        
        let lit = 段
            .iter()
            .position(|t| *t == Token::Sym("="))
            .and_then(|eq| lit_hint(&ty, &段[eq + 1..]));
        
        let is_exposed = self
            .cross_vals
            .get(&format!("{}|{}", self.module, self.block))
            .map(|m| m.contains_key(&name))
            .unwrap_or(false);
        
        
        if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
            let after = &段[eq + 1..];
            if let Some(v) = int_literal_val(after) {
                if let Some((lo, hi)) = width_range(&ty.0) {
                    if v < lo || v > hi {
                        return err(format!(
                            "val \"{}\" is declared as {}, literal {} is out of range ({}..={})",
                            name, ty.0, v, lo, hi
                        ));
                    }
                }
            }
        }
        
        
        let ty0 = ty.0.clone();
        let fn自有 = !self.f.is_block
            && {
                let 是聚合 = ty0.ends_with('*')
                    || matches!(
                        ty0.split('(').next().unwrap_or("").trim(),
                        "list" | "map" | "que" | "stack"
                    )
                    || self.types.iter().any(|t| t.name == ty0);
                let 构造形 = match 段
                    .iter()
                    .position(|t| *t == Token::Sym("="))
                    .and_then(|eq| 段.get(eq + 1))
                {
                    Some(Token::Sym("[")) | Some(Token::Sym("{")) => true,
                    Some(Token::Ident(_)) | Some(Token::Str(_)) => matches!(
                        段.iter().position(|t| *t == Token::Sym("=")).and_then(|eq| 段.get(eq + 2)),
                        Some(Token::Sym("("))
                    ),
                    _ => false,
                };
                是聚合 && 构造形
            };
        let id = if is_exposed {
            match self.exposed.get(&name).copied() {
                Some(pid) => pid,
                None => {
                    let pid = self.new_local(Some(name.to_string()), Ty(format!("{}*", ty.0)), true, false);
                    self.exposed.insert(name.to_string(), pid);
                    pid
                }
            }
        } else {
            self.new_local(Some(name.to_string()), ty, false, false)
        };
            let 是常量声明 = matches!(段.get(1), Some(Token::Kw("fix")));
            if !是常量声明 {
                self.names.insert(name.to_string(), id);
            }
            let (是share, 是send) = prion_parser::建val修饰(原始段);
            if 是share && 是send {
                return err(format!("val \"{}\" cannot be both share and marked (send) - a value is either shared or moved", name));
            }
            if (是share || 是send) && !is_exposed {
                let t = self.f.locals[id].ty.clone();
                self.共享序.push((name.to_string(), id, t));
            }
            if fn自有 {
                if let Some(l) = self.f.locals.get_mut(id) {
                    l.fn_owned = true;
                }
            }
            if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                self.lit_ty = lit;
                
                
                let 期望 = self.f.locals.get(id).map(|l| l.ty.0.clone());
                
                let 是fix = matches!(段.get(1), Some(Token::Kw("fix")));
                
                let 折叠常 = 段.iter().position(|t| *t == Token::Sym("=")).and_then(|e| {
                    let 右 = &段[e + 1..];
                    crate::consts::折叠(右).or_else(|| {
                        let 表 = self.常量值.borrow();
                        crate::consts::折叠带环境(
                            右,
                            &mut |n: &str| 表.get(n).cloned(),
                            &mut |f, a| 编译期执行(f, a, &表, 0),
                        )
                    })
                });
                if 是fix {
                    if let Some(c) = &折叠常 {
                        let 常量名 = match 段.get(2) {
                            Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                            _ => String::new().into(),
                        };
                        if !常量名.is_empty() {
                            self.常量值.borrow_mut().insert(常量名.to_string(), c.clone());
                        }
                    }
                }
                
                if matches!(段.get(1), Some(Token::Kw("fix"))) && 折叠常.is_none() {
                    
                    if let Some((项, 限)) = crate::quota::超限() {
                        return Err(BuildError {
                            message: format!(
                                "compile-time evaluation exceeded the {} quota (limit {}) - this is not a syntax error. Fix: raise --const-depth / --const-steps, or pass 0 for no limit",
                                项, 限
                            ),
                            code: prion_error::Code::E205,
                            line: CUR_LINE.with(|c| c.get()),
                            file: String::new(),
                        });
                    }
                    return Err(BuildError {
                        message: format!(
                            "the value of constant \"{}\" is not a compile-time-evaluable expression - constants require literals, operators, or fn calls whose arguments are all constants",
                            常量名显示(段)
                        ),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
                let 折叠值 = 折叠常.as_ref().map(|c| c.转值());
                let v = match 折叠值 {
                    Some(c) => c,
                    None => match pre {
                        Some(v) => v,
                        None => {
                            let 旧 = self.期望类型.take();
                            self.期望类型 = 期望.clone();
                            
                            let r = match exprs.first() {
                                Some(e) => self.expr_tree(e),
                                None => self.expr(&段[eq + 1..]),
                            };
                            self.期望类型 = 旧;
                            r?
                        }
                    },
                };
                self.lit_ty = None;

                let v = if let Some(exp) = 期望.as_ref() {
                    if exp.starts_with('?') && self.type_idx.contains_key(exp) {
                        match v {
                            Value::Local(id)
                                if self.f.locals.get(id).map(|l| l.ty.0 == *exp).unwrap_or(false) => v,
                            other => {
                                let inner_ty = exp.trim_start_matches('?').to_string();
                                let args = vec![other];
                                let tmp = self.temp(Ty(exp.clone()));
                                self.emit(Stmt::Assign {
                                    lhs: Place::local(tmp),
                                    rhs: Rvalue::VariantLit {
                                        ty: exp.clone(), tag: 1,
                                        args, tys: vec![Ty(inner_ty)],
                                    },
                                });
                                Value::Local(tmp)
                            }
                        }
                    } else { v }
                } else { v };

                let lhs = if is_exposed {
                    Place { base: id, proj: vec![Proj::Deref] }
                } else {
                    Place::local(id)
                };
                self.emit(Stmt::Assign { lhs, rhs: Rvalue::Const(v) });
            }
            return Ok(());
        }

        
        if matches!(kind, prion_parser::LineKind::Assign(_)) {
            let name = match 段.get(1) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err(format!("expected a name after val: {:?}", 段)),
            };
            
            if let Some(&pid) = self.exposed.get(&name) {
                if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                    let v = self.行右值(exprs, 段, eq)?;
                    let 写回 = match v {
                        Value::Local(id) => Place::local(id),
                        other => {
                            
                            let rv = match other {
                                Value::Int(i) => Rvalue::Const(Value::Int(i)),
                                Value::Float(f) => Rvalue::Const(Value::Float(f)),
                                Value::Bool(b) => Rvalue::Const(Value::Bool(b)),
                                Value::Str(s) => Rvalue::Const(Value::Str(s)),
                                Value::Null => Rvalue::Const(Value::Null),
                                _ => return err("internal: this form cannot yet write back a same-block val"),
                            };
                            self.emit(Stmt::Assign {
                                lhs: Place { base: pid, proj: vec![Proj::Deref] },
                                rhs: rv,
                            });
                            Place { base: pid, proj: vec![Proj::Deref] }
                        }
                    };
                    self.emit(Stmt::Assign {
                        lhs: Place { base: pid, proj: vec![Proj::Deref] },
                        rhs: Rvalue::Use(写回),
                    });
                }
                return Ok(());
            }
            let id = *self.names.get(&name).ok_or_else(|| BuildError {
                message: format!("val \"{}\" has not been defined yet (fn={})", name, self.f.name),
                code: prion_error::Code::E304,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            })?;
            if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                let tty = self.f.locals.get(id).map(|l| l.ty.clone());
                self.lit_ty = tty.as_ref().and_then(|t| lit_hint(t, &段[eq + 1..]));
                let v = self.行右值(exprs, 段, eq)?;
                self.lit_ty = None;
                self.emit(Stmt::Assign { lhs: Place::local(id), rhs: Rvalue::Const(v) });
            }
            return Ok(());
        }

        err(format!(
            "unsupported statement (currently supported: add val / add fn / add type / add outs fn / val \"name\" = / call fn / call #:block / call ##:module #:block / print / if / while / for / match / asm / index write / field write): {:?}",
            段
        ))
    }

    
    fn for_list_stmt(&mut self, head: &[Token], body: &[AstStmt]) -> Result<(), BuildError> {
        let var = match head.get(1) {
            Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
            _ => return err("expected a loop variable after for"),
        };
        
        let lst_toks: Vec<Token> = head[3..head.len() - 1].to_vec();
        let lst = self.expr(&lst_toks)?;
        let lid = match lst {
            Value::Local(id) => id,
            other => {

                
                let ty = self.ty_of_value(other.clone());
                let t = self.temp(ty);
                self.emit(Stmt::Assign { lhs: Place::local(t), rhs: Rvalue::Const(other) });
                t
            }
        };
        
        let lst_ty = self.f.locals.get(lid).map(|l| l.ty.0.clone()).unwrap_or_default();
        let elem = if lst_ty == "str" {

            "str".to_string()
        } else if matches!(lst_toks.first(), Some(Token::Ident(n)) if n == "keys") {
            "str".to_string()
        } else {
            self.f
                .locals
                .get(lid)
                .and_then(|l| elem_ty_of(&l.ty.0))
                .unwrap_or_else(|| "int".into())
        };
        let vid = self.new_local(Some(var.to_string()), Ty(elem), false, true);
        self.names.insert(var.to_string(), vid);
        let idx = self.temp(Ty("int".into()));
        self.emit(Stmt::Assign { lhs: Place::local(idx), rhs: Rvalue::Const(Value::Int(1)) });
        
        if lst_ty == "str" {
            self.emit(Stmt::Call {
                name: "#str_iter_start".into(),
                args: vec![Value::Local(lid)],
            });
        }
        
        let len_t = self.temp(Ty("int".into()));
        self.emit(Stmt::Assign {
            lhs: Place::local(len_t),
            rhs: Rvalue::Call { name: "len".into(), args: vec![Value::Local(lid)] },
        });

        let head_id = self.nb();
        let body_id = self.nb();
        let exit_id = self.nb();
        self.goto(head_id);

        self.switch_to(head_id);
        let c = self.bin(BinOp::Le, Value::Local(idx), Value::Local(len_t));
        self.set_exit(Exit::Branch { cond: c, targets: vec![Succ::new(body_id), Succ::new(exit_id)] });

        self.switch_to(body_id);

        if lst_ty == "str" {
            self.emit(Stmt::Assign {
                lhs: Place::local(vid),
                rhs: Rvalue::Call {
                    name: "#str_iter_next".into(),
                    args: vec![Value::Local(lid), Value::Local(idx)],
                },
            });
        } else {
            self.emit(Stmt::Assign {
                lhs: Place::local(vid),
                rhs: Rvalue::ListGet { list: Value::Local(lid), index: Value::Local(idx) },
            });
        }
        self.循环深 += 1;
        self.body(body)?;
        self.循环深 -= 1;
        let nxt = self.bin(BinOp::Add, Value::Local(idx), Value::Int(1));
        self.emit(Stmt::Assign { lhs: Place::local(idx), rhs: Rvalue::Const(nxt) });
        self.goto(head_id);

        self.switch_to(exit_id);
        Ok(())
    }

    
    fn for_range_stmt(&mut self, head: &[Token], body: &[AstStmt]) -> Result<(), BuildError> {
        
        
        if head.len() < 5 || !matches!(head.get(2), Some(Token::Ident(w)) if w == "in") {
            return err("for currently only supports `for var in a~b:` (range loop); iterating containers is not available yet");
        }
        let var = match head.get(1) {
            Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
            _ => return err("expected a loop variable after for"),
        };
        
        let mut 深 = 0i32;
        let mut 波浪 = None;
        for (i, t) in head.iter().enumerate().skip(3) {
            match t {
                Token::Sym("(") | Token::Sym("[") | Token::Sym("{") => 深 += 1,
                Token::Sym(")") | Token::Sym("]") | Token::Sym("}") => 深 -= 1,
                Token::Sym("~") if 深 == 0 => {
                    波浪 = Some(i);
                    break;
                }
                _ => {}
            }
        }
        let 波浪 = match 波浪 {
            Some(i) => i,
            None => return err("range loop is missing `~`"),
        };
        let 止 = if matches!(head.last(), Some(Token::Sym(":"))) {
            head.len() - 1
        } else {
            head.len()
        };
        if 波浪 == 3 || 波浪 + 1 >= 止 {
            return err("both ends of a range loop must be expressions");
        }
        let lo = self.expr(&head[3..波浪])?;
        let hi = self.expr(&head[波浪 + 1..止])?;

        
        let id = self.new_local(Some(var.to_string()), Ty("int".into()), false, true);
        self.names.insert(var.to_string(), id);
        self.emit(Stmt::Assign { lhs: Place::local(id), rhs: Rvalue::Const(lo) });

        let head_id = self.nb();
        let body_id = self.nb();
        let exit_id = self.nb();
        self.goto(head_id);

        self.switch_to(head_id);
        let c = self.bin(BinOp::Le, Value::Local(id), hi);
        self.set_exit(Exit::Branch { cond: c, targets: vec![Succ::new(body_id), Succ::new(exit_id)] });

        self.switch_to(body_id);
        self.循环深 += 1;
        self.body(body)?;
        self.循环深 -= 1;
        let nxt = self.bin(BinOp::Add, Value::Local(id), Value::Int(1));
        self.emit(Stmt::Assign { lhs: Place::local(id), rhs: Rvalue::Const(nxt) });
        self.goto(head_id);

        self.switch_to(exit_id);
        Ok(())
    }

    
    fn head_expr_tokens(head: &[Token]) -> &[Token] {
        if head.len() >= 2 {
            &head[1..head.len() - 1]
        } else {
            &[]
        }
    }

    fn if_stmt(
        &mut self,
        head: &[Token],
        body: &[AstStmt],
        else_body: Option<&Vec<AstStmt>>,
    ) -> Result<(), BuildError> {
        let c = self.expr(Self::head_expr_tokens(head))?;
        let then_id = self.nb();
        let else_id = self.nb();
        let next_id = self.nb();

        self.set_exit(Exit::Branch { cond: c, targets: vec![Succ::new(then_id), Succ::new(else_id)] });

        self.switch_to(then_id);
        self.body(body)?;
        self.goto(next_id);

        self.switch_to(else_id);
        if let Some(eb) = else_body {
            self.body(eb)?;
        }
        self.goto(next_id);

        self.switch_to(next_id);
        Ok(())
    }

    fn while_stmt(&mut self, head: &[Token], body: &[AstStmt]) -> Result<(), BuildError> {
        
        let head_toks = Self::head_expr_tokens(head);
        
        let (cond_toks, max_toks) = match head_toks.iter().position(|t| matches!(t, Token::Ident(w) if w == "max")) {
            Some(p) => (&head_toks[..p], Some(&head_toks[p + 1..])),
            None => (head_toks, None),
        };
        
        let cnt = match max_toks {
            Some(_) => {
                let c = self.new_local(None, Ty("int".into()), false, false);
                self.emit(Stmt::Assign { lhs: Place::local(c), rhs: Rvalue::Const(Value::Int(0)) });
                Some(c)
            }
            None => None,
        };

        let head_id = self.nb();
        let body_id = self.nb();
        let exit_id = self.nb();
        self.goto(head_id);

        self.switch_to(head_id);
        let c = self.expr(cond_toks)?;
        self.set_exit(Exit::Branch { cond: c, targets: vec![Succ::new(body_id), Succ::new(exit_id)] });

        self.switch_to(body_id);
        
        if let (Some(cnt), Some(mx)) = (cnt, max_toks) {
            
            let mv = self.expr(mx)?;
            
            if matches!(mv, Value::Int(0)) {
                return err("the max limit cannot be 0 - if you do not want a limit, omit max");
            }
            let mt = self.ty_of_value(mv.clone());
            if mt.0 != "int" {
                return err(format!(
                    "the while max limit must be an integer - got {} (e.g. `max 100` or an int variable/expression)",
                    mt.0
                ));
            }
            let c1 = self.bin(BinOp::Add, Value::Local(cnt), Value::Int(1));
            self.emit(Stmt::Assign { lhs: Place::local(cnt), rhs: Rvalue::Const(c1) });
            let ok = self.bin(BinOp::Le, Value::Local(cnt), mv.clone());
            self.emit(Stmt::Assert {
                cond: ok,
                msg: "loop exceeded the max limit - possible infinite loop".into(),
                fix: "check whether the loop's exit condition can be reached, or raise the max limit".into(),
            });
            
            if let Value::Int(n) = mv {
                if n > 0 {
                    if let Some(b) = self.blocks.get_mut(head_id) {
                        b.max = Some(n as u64);
                    }
                }
            }
        }
        self.循环深 += 1;
        self.body(body)?;
        self.循环深 -= 1;
        self.goto(head_id);

        self.switch_to(exit_id);
        Ok(())
    }

    
    fn match_stmt(&mut self, head: &[Token], body: &[AstStmt]) -> Result<(), BuildError> {
        let v = self.expr(Self::head_expr_tokens(head))?;
        let exit_id = self.nb();

        let mut cases: Vec<(TokStream, Vec<AstStmt>)> = Vec::new();
        for s in body {
            match s {
                AstStmt::Struct { head: h, body: cb, .. } => {
                    if !matches!(h.first(), Some(Token::Kw("case"))) {
                        return err("a match body may only contain case branches");
                    }
                    cases.push((h.clone(), cb.clone()));
                }
                AstStmt::Line { kind, .. } if kind.tokens().is_empty() => {}
                _ => return err("a match body may only contain case branches"),
            }
        }

        for (h, cb) in &cases {
            let ht_all = Self::head_expr_tokens(h).to_vec();
            let (ht, guard) = split_case_guard(&ht_all);
            
            if let Some(Token::Ident(vname)) | Some(Token::Str(vname)) = ht.first() {
                if let Some((_, tag, ftypes)) = self.sum_variants.get(vname.as_ref()).cloned() {
                    let tagv = self.temp(Ty("int".into()));
                    self.emit(Stmt::Assign {
                        lhs: Place::local(tagv),
                        rhs: Rvalue::VariantTag(v.clone()),
                    });
                    let t = self.bin(BinOp::Eq, Value::Local(tagv), Value::Int(tag as i64));
                    let body_id = self.nb();
                    let next_check = self.nb();
                    self.set_exit(Exit::Branch { cond: t, targets: vec![Succ::new(body_id), Succ::new(next_check)] });
                    self.switch_to(body_id);
                    
                    if matches!(ht.get(1), Some(Token::Sym("("))) {
                        let mut d = 0i32;
                        let mut end = None;
                        for (k, tk) in ht.iter().enumerate().skip(1) {
                            match tk {
                                Token::Sym("(") => d += 1,
                                Token::Sym(")") => {
                                    d -= 1;
                                    if d == 0 {
                                        end = Some(k);
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let Some(end) = end {
                            for (idx, a) in split_args(&ht[2..end]).into_iter().enumerate() {
                                let bname = match a.first() {
                                    Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
                                    _ => continue,
                                };
                                let bty = ftypes.get(idx).cloned().unwrap_or(Ty("int".into()));
                                let bid = self.new_local(Some(bname.to_string()), bty.clone(), false, true);
                                self.names.insert(bname.to_string(), bid);
                                self.emit(Stmt::Assign {
                                    lhs: Place::local(bid),
                                    rhs: Rvalue::VariantGet { value: v.clone(), idx, ty: bty },
                                });
                            }
                        }
                    }
                    if let Some(g) = guard {
                        let gv = self.expr(g)?;
                        let arm_id = self.nb();
                        self.set_exit(Exit::Branch {
                            cond: gv,
                            targets: vec![Succ::new(arm_id), Succ::new(next_check)],
                        });
                        self.switch_to(arm_id);
                    }
                    self.body(cb)?;
                    self.goto(exit_id);
                    self.switch_to(next_check);
                    continue;
                }
            }
            
            if ht.len() == 1 && matches!(ht[0], Token::Ident(ref n) if n == "_") {
                if let Some(g) = guard {
                    let gv = self.expr(g)?;
                    let body_id = self.nb();
                    let next_check = self.nb();
                    self.set_exit(Exit::Branch {
                        cond: gv,
                        targets: vec![Succ::new(body_id), Succ::new(next_check)],
                    });
                    self.switch_to(body_id);
                    self.body(cb)?;
                    self.goto(exit_id);
                    self.switch_to(next_check);
                    continue;
                }
                self.body(cb)?;
                self.goto(exit_id);
                continue;
            }
            let cv = self.expr(ht)?;
            let t = self.bin(BinOp::Eq, v.clone(), cv);
            let body_id = self.nb();
            let next_check = self.nb();
            self.set_exit(Exit::Branch { cond: t, targets: vec![Succ::new(body_id), Succ::new(next_check)] });

            self.switch_to(body_id);
            if let Some(g) = guard {
                let gv = self.expr(g)?;
                let arm_id = self.nb();
                self.set_exit(Exit::Branch {
                    cond: gv,
                    targets: vec![Succ::new(arm_id), Succ::new(next_check)],
                });
                self.switch_to(arm_id);
            }
            self.body(cb)?;
            self.goto(exit_id);

            self.switch_to(next_check);
        }
        
        self.goto(exit_id);
        self.switch_to(exit_id);
        Ok(())
    }

    
    
    fn if_expr(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        *i += 1;
        let mut depth = 0i32;
        let mut brace = None;
        let mut j = *i;
        while j < t.len() {
            match &t[j] {
                Token::Sym("(") | Token::Sym("[") => depth += 1,
                Token::Sym(")") | Token::Sym("]") => depth -= 1,
                Token::Sym("{") if depth == 0 => { brace = Some(j); break; }
                _ => {}
            }
            j += 1;
        }
        let brace = match brace { Some(b) => b, None => return err("if expression is missing {") };
        let cond_toks = &t[*i..brace];
        if cond_toks.is_empty() { return err("if must be followed by a condition"); }
        let mut k = brace + 1;
        let mut d2 = 0i32;
        let then_end;
        loop {
            if k >= t.len() { return err("the if expression's } is not closed"); }
            match &t[k] {
                Token::Sym("{") => d2 += 1,
                Token::Sym("}") if d2 == 0 => { then_end = k; break; }
                Token::Sym("}") => d2 -= 1,
                _ => {}
            }
            k += 1;
        }
        let then_toks = &t[brace + 1..then_end];
        k = then_end + 1;
        if !matches!(t.get(k), Some(Token::Kw("else"))) { return err("an if expression must have else"); }
        k += 1;
        if !matches!(t.get(k), Some(Token::Sym("{"))) { return err("else must be followed by {"); }
        let else_start = k + 1;
        let mut k2 = else_start;
        d2 = 0;
        let else_end;
        loop {
            if k2 >= t.len() { return err("else's } is not closed"); }
            match &t[k2] {
                Token::Sym("{") => d2 += 1,
                Token::Sym("}") if d2 == 0 => { else_end = k2; break; }
                Token::Sym("}") => d2 -= 1,
                _ => {}
            }
            k2 += 1;
        }
        k = else_end;
        let else_toks = &t[else_start..else_end];
        *i = else_end + 1;

        let c = self.expr(cond_toks)?;
        let bb_then = self.nb();
        let bb_else = self.nb();
        let bb_done = self.nb();
        self.set_exit(Exit::Branch { cond: c, targets: vec![
            Succ { target: bb_then, args: vec![] },
            Succ { target: bb_else, args: vec![] },
        ] });
        self.switch_to(bb_then);
        let tv = self.expr(then_toks)?;
        let mut ty = self.ty_of_value(tv.clone());
        if ty.0 == "null" {
            if let Some(exp) = self.期望类型.clone() {
                ty = Ty(exp);
            }
        }
        let tmp = self.temp(ty.clone());
        self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Const(tv) });
        self.goto(bb_done);
        self.switch_to(bb_else);
        let ev = self.expr(else_toks)?;
        let ety = self.ty_of_value(ev.clone());
        if ety.0 != ty.0 && ety.0 != "null" {
            return err(format!(
                "the two branches of an if expression must have the same type - one is \"{}\", the other is \"{}\"",
                ty.0, ety.0
            ));
        }
        self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Const(ev) });
        self.goto(bb_done);
        self.switch_to(bb_done);
        Ok(self.if结果把手(tmp, &ty))
    }

    fn match_expr(&mut self, t: &[Token], i: &mut usize) -> Result<Value, BuildError> {
        *i += 1; 
        
        let mut depth = 0i32;
        let mut brace = None;
        let mut j = *i;
        while j < t.len() {
            match &t[j] {
                Token::Sym("(") | Token::Sym("[") => depth += 1,
                Token::Sym(")") | Token::Sym("]") => depth -= 1,
                Token::Sym("{") => {
                    if depth == 0 {
                        brace = Some(j);
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let brace = match brace {
            Some(b) => b,
            None => return err("the `{` of the expression-form match was not found - the form is `match subject { case variant(binding): expression, ... }`"),
        };
        let obj_toks: Vec<Token> = t[*i..brace].to_vec();
        if obj_toks.is_empty() {
            return err("match must be followed by a subject");
        }
        
        let mut arms: Vec<(Vec<Token>, Vec<Token>)> = Vec::new();
        j = brace + 1;
        loop {
            if j >= t.len() {
                return err("the expression-form match's `}` is not closed");
            }
            if matches!(&t[j], Token::Sym("}")) {
                j += 1;
                break;
            }
            if !matches!(&t[j], Token::Kw("case")) {
                return err("the expression-form match body may only contain case branches (e.g. `case get(v): get(v*2)`)");
            }
            j += 1;
            
            let pat_start = j;
            let mut d2 = 0i32;
            while j < t.len() {
                match &t[j] {
                    Token::Sym("(") => d2 += 1,
                    Token::Sym(")") => d2 -= 1,
                    Token::Sym(":") if d2 == 0 => break,
                    _ => {}
                }
                j += 1;
            }
            if j >= t.len() {
                return err("the expression-form match case is missing `:`");
            }
            let pat: Vec<Token> = t[pat_start..j].to_vec();
            j += 1; 
            
            let ex_start = j;
            let mut d3 = 0i32;
            while j < t.len() {
                match &t[j] {
                    Token::Sym("(") | Token::Sym("[") => d3 += 1,
                    Token::Sym(")") | Token::Sym("]") => d3 -= 1,
                    Token::Sym("{") => d3 += 1,
                    Token::Sym("}") if d3 == 0 => break,
                    Token::Sym("}") => d3 -= 1,
                    Token::Sym(",") if d3 == 0 => break,
                    _ => {}
                }
                j += 1;
            }
            let ex: Vec<Token> = t[ex_start..j].to_vec();
            if matches!(t.get(j), Some(Token::Sym(","))) {
                j += 1;
            }
            arms.push((pat, ex));
        }
        *i = j;
        if arms.is_empty() {
            return err("the expression-form match needs at least one case");
        }
        let v = self.expr(&obj_toks)?;
        let tmp = self.temp(Ty("int".into())); 
        let exit_id = self.nb();
        let mut first_ty: Option<Ty> = None;
        for (pat, ex) in &arms {
            
            if pat.len() == 1 && matches!(&pat[0], Token::Ident(n) if n == "_") {
                let av = self.expr(ex)?;
                if first_ty.is_none() {
                    first_ty = Some(self.ty_of_value(av.clone()));
                }
                self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Const(av) });
                self.goto(exit_id);
                continue;
            }
            let vname = match pat.first() {
                Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
                _ => return err("expression-form match case patterns only support `variant(binding...)` and `_`"),
            };
            let (_, tag, ftypes) = match self.sum_variants.get(&vname).cloned() {
                Some(x) => x,
                None => return err(format!("\"{}\" is not a sum-type variant - expression-form match cases dispatch only by variant", vname)),
            };
            let tagv = self.temp(Ty("int".into()));
            self.emit(Stmt::Assign { lhs: Place::local(tagv), rhs: Rvalue::VariantTag(v.clone()) });
            let cmp = self.bin(BinOp::Eq, Value::Local(tagv), Value::Int(tag as i64));
            let body_id = self.nb();
            let next_id = self.nb();
            self.set_exit(Exit::Branch { cond: cmp, targets: vec![Succ::new(body_id), Succ::new(next_id)] });
            self.switch_to(body_id);
            
            if matches!(pat.get(1), Some(Token::Sym("("))) {
                let mut d = 0i32;
                let mut end = None;
                for (k, tk) in pat.iter().enumerate().skip(1) {
                    match tk {
                        Token::Sym("(") => d += 1,
                        Token::Sym(")") => {
                            d -= 1;
                            if d == 0 {
                                end = Some(k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(end) = end {
                    for (idx, a) in split_args(&pat[2..end]).into_iter().enumerate() {
                        let bname = match a.first() {
                            Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
                            _ => continue,
                        };
                        let bty = ftypes.get(idx).cloned().unwrap_or(Ty("int".into()));
                        let bid = self.new_local(Some(bname.to_string()), bty.clone(), false, true);
                        self.names.insert(bname.to_string(), bid);
                        self.emit(Stmt::Assign {
                            lhs: Place::local(bid),
                            rhs: Rvalue::VariantGet { value: v.clone(), idx, ty: bty },
                        });
                    }
                }
            }
            let av = self.expr(ex)?;
            if first_ty.is_none() {
                first_ty = Some(self.ty_of_value(av.clone()));
            }
            self.emit(Stmt::Assign { lhs: Place::local(tmp), rhs: Rvalue::Const(av) });
            self.goto(exit_id);
            self.switch_to(next_id);
        }
        
        self.emit(Stmt::Assert {
            cond: Value::Bool(false),
            msg: "match hit no branch - exhaustiveness was bypassed".into(),
            fix: "add the missing case, or write `case _:` as a fallback".into(),
        });
        self.goto(exit_id);
        self.switch_to(exit_id);
        if let Some(ty) = first_ty {
            if let Some(l) = self.f.locals.get_mut(tmp) {
                l.ty = ty;
            }
        }
        Ok(Value::Local(tmp))
    }

    
    fn stmt_fn(&mut self, head: &[Token], body: &[AstStmt]) -> Result<Function, BuildError> {
        let h: Vec<Token> = head.iter().cloned().filter(|t| *t != Token::Sym(":")).collect();
        let is_rebind = matches!(h.first(), Some(Token::Kw("fn")));
        let is_hide_fn = matches!(h.first(), Some(Token::Kw("add")))
            && matches!(h.get(1), Some(Token::Kw("hide")))
            && matches!(h.get(2), Some(Token::Kw("fn")));
        let name_at = if is_rebind { 1 } else if is_hide_fn { 3 } else { 2 };
        let name = match h.get(name_at) {
            Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
            _ => return err("expected a name after statement-body fn"),
        };
        let (params, after) = parse_params(&h)?;
        let (ret, _限, _) = parse_type_ref(&h, after).map_err(|_| BuildError {
            message: "statement-body fn is missing a return type (boundary types on functions are mandatory)".into(),
            code: prion_error::Code::E301,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        self.stmt_fn_body(&name, params, ret, body)
    }

    
    fn 建语句体实例(
        &mut self,
        实例名: &str,
        类型参数: &[String],
        实参类型: &[String],
        形参: &[(String, Ty)],
        返回: &Ty,
        体: &[AstStmt],
    ) -> Result<(), BuildError> {
        let 具体形参: Vec<(String, Ty)> = 形参
            .iter()
            .map(|(n, t)| (n.clone(), Ty(替换参数(&t.0, 类型参数, 实参类型))))
            .collect();
        let 具体返回 = Ty(替换参数(&返回.0, 类型参数, 实参类型));
        let key = format!("{}|{}|{}", self.module, self.block, 实例名);
        self.fns.insert(
            key,
            FnSig {
                params: 具体形参.iter().map(|(_, t)| t.clone()).collect(),
                ret: 具体返回.clone(),
                is_outs: false,
                hidden: Vec::new(),
            },
        );
        let prev = 类型代换表.with(|c| {
            c.replace(Some((类型参数.to_vec(), 实参类型.to_vec())))
        });
        let r = self.stmt_fn_body(实例名, 具体形参, 具体返回, 体);
        类型代换表.with(|c| {
            c.replace(prev);
        });
        let f = r?;
        self.extra.push(f);
        Ok(())
    }

    
    fn stmt_fn_body(
        &mut self,
        name: &str,
        params: Vec<(String, Ty)>,
        ret: Ty,
        body: &[AstStmt],
    ) -> Result<Function, BuildError> {
        
        let (last, init) = body.split_last().ok_or_else(|| BuildError {
            message: "the statement-body fn is empty - end it with `give expression`".into(),
            code: prion_error::Code::E301,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        let give_toks: Vec<Token> = match last {
            AstStmt::Line { kind, .. } if matches!(kind.tokens().first(), Some(Token::Kw("give"))) => {
                kind.tokens()[1..].to_vec()
            }
            _ => {
                return Err(BuildError {
                    message: "the last statement of a statement-body fn must be `give expression`".into(),
                    code: prion_error::Code::E301,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })
            }
        };
        let key = format!("{}|{}|{}", self.module, self.block, name);
        let mut fb = Builder::new(
            format!("{}|{}|{}", self.module, self.block, name),
            self.f.file.clone(),
            false,
            false,
            false,
            false,
            false,
            Vec::new(),
            self.block.clone(),
            self.module.clone(),
            std::rc::Rc::clone(&self.types),
            std::rc::Rc::clone(&self.type_idx),
            std::rc::Rc::clone(&self.mod_by_tail),
            std::rc::Rc::clone(&self.global_fns),
            std::rc::Rc::clone(&self.cross_vals),
            std::rc::Rc::clone(&self.sum_variants),
            std::rc::Rc::clone(&self.pub_blocks),
            std::rc::Rc::clone(&self.hidden_items),
            std::rc::Rc::clone(&self.block_types),
            std::rc::Rc::clone(&self.常量值),
        );
        fb.cur_col = self.cur_col;
        fb.cur_len = self.cur_len;
        
        
        for (pname, pty) in &params {
            let id = fb.new_local(Some(pname.clone()), pty.clone(), true, true);
            fb.names.insert(pname.clone(), id);
        }
        
        let hidden = 注入槽参数_prep(&mut fb, &key);
        self.fns.insert(
            key.clone(),
            FnSig {
                params: params.iter().map(|(_, t)| t.clone()).collect(),
                ret: ret.clone(),
                is_outs: false,
                hidden,
            },
        );
        fb.语句体返回 = true;
        fb.body(init)?;
        fb.期望类型 = Some(ret.0.clone());
        let v = fb.expr(&give_toks)?;
        fb.set_exit(Exit::Return(Some(v)));
        let mut f = fb.finish()?;
        f.ret = Some(ret);
        Ok(f)
    }

    
    fn 任务体(
        &mut self,
        name: &str,
        ret: Ty,
        body: &[AstStmt],
        捕获: &[(String, Ty)],
    ) -> Result<(Function, bool), BuildError> {
        let mut init: &[AstStmt] = body;
        let mut give_toks: Option<Vec<Token>> = None;
        let mut 自结束 = false;
        if let Some((last, rest)) = body.split_last() {
            if let AstStmt::Line { kind, .. } = last {
                let toks = kind.tokens();
                if matches!(toks.first(), Some(Token::Kw("give"))) {
                    give_toks = Some(toks[1..].to_vec());
                    init = rest;
                } else if toks.len() == 1 && 是裸join(toks.first()) {
                    自结束 = true;
                    init = rest;
                }
            }
        }
        if !自结束 && give_toks.is_none() {
            return Err(BuildError {
                message: "the spawn task body must end with `give value` (to return a value) or a lone `join` (self-terminate, do not wait)".into(),
                code: prion_error::Code::E301,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            });
        }
        let mut fb = Builder::new(
            format!("{}|{}|{}", self.module, self.block, name),
            self.f.file.clone(),
            false,
            false,
            false,
            false,
            false,
            Vec::new(),
            self.block.clone(),
            self.module.clone(),
            std::rc::Rc::clone(&self.types),
            std::rc::Rc::clone(&self.type_idx),
            std::rc::Rc::clone(&self.mod_by_tail),
            std::rc::Rc::clone(&self.global_fns),
            std::rc::Rc::clone(&self.cross_vals),
            std::rc::Rc::clone(&self.sum_variants),
            std::rc::Rc::clone(&self.pub_blocks),
            std::rc::Rc::clone(&self.hidden_items),
            std::rc::Rc::clone(&self.block_types),
            std::rc::Rc::clone(&self.常量值),
        );
        fb.cur_col = self.cur_col;
        fb.cur_len = self.cur_len;
        let ctx_id = fb.new_local(Some("__ctx".into()), Ty("ctx".into()), true, false);
        fb.names.insert("__ctx".into(), ctx_id);
        fb.任务上下文 = Some((ctx_id, 捕获.to_vec()));
        for (i, (n, ty)) in 捕获.iter().enumerate() {
            let lid = fb.new_local(Some(n.clone()), ty.clone(), false, false);
            fb.emit(Stmt::Assign {
                lhs: Place::local(lid),
                rhs: Rvalue::CtxGet { ctx: ctx_id, idx: i, ty: ty.clone() },
            });
            fb.names.insert(n.clone(), lid);
        }
        fb.语句体返回 = true;
        fb.body(init)?;
        match give_toks {
            Some(g) => {
                let v = fb.expr(&g)?;
                fb.set_exit(Exit::Return(Some(v)));
            }
            None => fb.set_exit(Exit::Return(Some(Value::Int(0)))),
        }
        let mut f = fb.finish()?;
        f.ret = Some(if 自结束 { Ty("int".into()) } else { ret });
        Ok((f, 自结束))
    }

    
    fn body(&mut self, stmts: &[AstStmt]) -> Result<(), BuildError> {
        let mut j = 0usize;
        while j < stmts.len() {
            match &stmts[j] {
                AstStmt::Line { line, col, len, kind, exprs, .. } => {
                    self.cur_line = *line;
                    self.cur_col = *col;
                    self.cur_len = *len;
                    CUR_LINE.set(*line);
                    let t = kind.tokens().clone();
                    let ex = exprs.clone();
                    self.line(kind, &t, &ex)?;
                    j += 1;
                }
                AstStmt::Fn { head, body, .. } => {
                    if let Some((name, 类型参数, 形参, 返回)) = 解析泛型语句头(head) {
                        泛型语句体fn表.with(|g| {
                            g.borrow_mut().insert(
                                name,
                                (类型参数, 形参, 返回, body.clone(), self.module.clone(), self.block.clone()),
                            );
                        });
                    } else {
                        let s = self.stmt_fn(head, body)?;
                        self.extra.push(s);
                    }
                    j += 1;
                }
                AstStmt::Struct { kind: pk, head, body, .. } => {
                    let kind = match pk {
                        prion_parser::StructKind::If => Kind::If,
                        prion_parser::StructKind::Else => Kind::Else,
                        prion_parser::StructKind::While => Kind::While,
                        prion_parser::StructKind::Match => Kind::Match,
                        prion_parser::StructKind::For => Kind::For,
                        prion_parser::StructKind::Case => return err("case cannot appear on its own"),
                        prion_parser::StructKind::Spawn => {
                            if let Some(Token::Ident(n) | Token::Str(n)) = head.get(2) {
                                let payload = 解析token载荷(head);
                                if payload != "int" && payload != "bool" && payload != "void" {
                                    return err(format!(
                                        "token payload \"{}\" is not supported yet - only int / bool / void (self-terminating) for now",
                                        payload
                                    ));
                                }
                                let task_name = format!("{}|{}|__task_{}", self.module, self.block, n);
                                let 捕获: Vec<(String, Ty)> =
                                    self.共享序.iter().map(|(nm, _, t)| (nm.clone(), t.clone())).collect();
                                let 实参: Vec<Value> =
                                    self.共享序.iter().map(|(_, lid, _)| Value::Local(*lid)).collect();
                                let (tf, 自结束) =
                                    self.任务体(&format!("__task_{}", n), Ty(payload), body, &捕获)?;
                                self.extra.push(tf);
                                let id = self.new_local(Some(n.to_string()), Ty("token".into()), false, false);
                                let fnref = self.temp(Ty("fn".into()));
                                self.emit(Stmt::Assign {
                                    lhs: Place::local(fnref),
                                    rhs: Rvalue::FnRef { name: task_name },
                                });
                                let ctx = self.temp(Ty("ctx".into()));
                                self.emit(Stmt::Assign {
                                    lhs: Place::local(ctx),
                                    rhs: Rvalue::CtxArray { args: 实参 },
                                });
                                let n捕获 = self.共享序.len() as i64;
                                self.emit(Stmt::Assign {
                                    lhs: Place::local(id),
                                    rhs: Rvalue::Call {
                                        name: if 自结束 { "prion_spawn_detached" } else { "prion_spawn" }.into(),
                                        args: vec![Value::Local(fnref), Value::Local(ctx), Value::Int(n捕获)],
                                    },
                                });
                                self.names.insert(n.to_string(), id);
                            }
                            j += 1;
                            continue;
                        },
                    };
                    match kind {
                        Kind::For => {
                            
                            let is_range = head.contains(&Token::Sym("~"));
                            let h = head.clone();
                            let ib = body.clone();
                            if is_range {
                                self.for_range_stmt(&h, &ib)?;
                            } else {
                                self.for_list_stmt(&h, &ib)?;
                            }
                            j += 1;
                        }
                        Kind::Else => return err("else cannot appear on its own (it must come right after the ; that closes its if body)"),
                        Kind::If => {
                            let eb: Option<Vec<AstStmt>> = match stmts.get(j + 1) {
                                Some(AstStmt::Struct { kind: prion_parser::StructKind::Else, body: b, .. }) => Some(b.clone()),
                                _ => None,
                            };
                            let h = head.clone();
                            let ib = body.clone();
                            
                            match self.条件常量(&h) {
                                Some(true) => self.body(&ib)?,
                                Some(false) => {
                                    if let Some(eb2) = eb.as_ref() {
                                        self.body(eb2)?;
                                    }
                                }
                                None => self.if_stmt(&h, &ib, eb.as_ref())?,
                            }
                            j += if eb.is_some() { 2 } else { 1 };
                        }
                        Kind::While => {
                            let h = head.clone();
                            let ib = body.clone();
                            self.while_stmt(&h, &ib)?;
                            j += 1;
                        }
                        Kind::Match => {
                            let h = head.clone();
                            let ib = body.clone();
                            self.match_stmt(&h, &ib)?;
                            j += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}




fn 代换类型(s: &str) -> String {
    let base = if s.ends_with(')') {
        s.find('(')
            .filter(|&i| matches!(s.as_bytes().get(i + 1), Some(b'>') | Some(b'<')))
            .map(|i| &s[..i])
            .unwrap_or(s)
    } else { s };
    let m = 类型代换表.with(|c| c.borrow().clone());
    match m {
        Some((参数, 实参)) => 替换参数(base, &参数, &实参),
        None => base.to_string(),
    }
}


fn parse_type_ref(
    段: &[Token],
    at: usize,
) -> Result<(Ty, Option<(String, String, String)>, usize), BuildError> {
    let (t, a, b) = parse_type_ref_raw(段, at)?;
    Ok((Ty(代换类型(&t.0)), a, b))
}

fn parse_type_ref_raw(
    段: &[Token],
    at: usize,
) -> Result<(Ty, Option<(String, String, String)>, usize), BuildError> {
    match 段.get(at) {
        Some(Token::Kw(t)) => {
            let 基 = (*t).to_string();
            if matches!(基.as_str(), "list" | "que" | "stack" | "map")
                && matches!(段.get(at + 1), Some(Token::Sym("(")))
            {
                let mut 深 = 0i32;
                let mut 尽 = None;
                for (k, tk) in 段.iter().enumerate().skip(at + 1) {
                    match tk {
                        Token::Sym("(") => 深 += 1,
                        Token::Sym(")") => {
                            深 -= 1;
                            if 深 == 0 {
                                尽 = Some(k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let 尽 = 尽.ok_or_else(|| BuildError {
                    message: format!("parentheses of container type \"{}\" are not closed", 基),
                    code: prion_error::Code::E201,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
                let mut 实参: Vec<String> = Vec::new();
                for 段 in split_args(&段[at + 2..尽]) {
                    let (ty, _, _) = parse_type_ref(&段, 0)?;
                    实参.push(ty.0);
                }
                Ok((Ty(format!("{}({})", 基, 实参.join(", "))), None, 尽 + 1))
            } else if 基 == "fn" && matches!(段.get(at + 1), Some(Token::Sym("("))) {
                let mut 深 = 0i32;
                let mut 尽 = None;
                for (k, tk) in 段.iter().enumerate().skip(at + 1) {
                    match tk {
                        Token::Sym("(") => 深 += 1,
                        Token::Sym(")") => {
                            深 -= 1;
                            if 深 == 0 {
                                尽 = Some(k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let 尽 = 尽.ok_or_else(|| BuildError {
                    message: "parentheses of function type \"fn\" are not closed".into(),
                    code: prion_error::Code::E201,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
                let mut 实参: Vec<String> = Vec::new();
                for 段 in split_args(&段[at + 2..尽]) {
                    let (t, _, _) = parse_type_ref(&段, 0)?;
                    实参.push(t.0);
                }
                let (ret, _, 止) = parse_type_ref(段, 尽 + 1)?;
                Ok((Ty(format!("fn({}) {}", 实参.join(", "), ret.0)), None, 止))
            } else {
                Ok((Ty(基), None, at + 1))
            }
        }
        Some(Token::Ident(n)) | Some(Token::Str(n)) => {
            
            let 参数 = GENERICS.with(|g| g.borrow().get(n.as_ref()).cloned());
            if let Some(ps) = 参数 {
                if !matches!(段.get(at + 1), Some(Token::Sym("("))) {
                    return Err(BuildError {
                        message: format!("generic type \"{}\" requires type parameters (e.g. `{}(int, str)`)", n, n),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
                let mut 深 = 0i32;
                let mut 尽 = None;
                for (k, t) in 段.iter().enumerate().skip(at + 1) {
                    match t {
                        Token::Sym("(") => 深 += 1,
                        Token::Sym(")") => {
                            深 -= 1;
                            if 深 == 0 {
                                尽 = Some(k);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let 尽 = 尽.ok_or_else(|| BuildError {
                    message: format!("the argument parentheses of generic type \"{}\" are not closed", n),
                    code: prion_error::Code::E201,
                    line: CUR_LINE.with(|c| c.get()),
                    file: String::new(),
                })?;
                let mut 实参: Vec<String> = Vec::new();
                for 段 in split_args(&段[at + 2..尽]) {
                    let (t, _, _) = parse_type_ref(&段, 0)?;
                    if t.0.ends_with('*') || !t.0.contains('(') {
                        实参.push(t.0);
                    } else {
                        return Err(BuildError {
                            message: format!("generic arguments must be single-level types (`{}` is too complex)", t.0),
                            code: prion_error::Code::E201,
                            line: CUR_LINE.with(|c| c.get()),
                            file: String::new(),
                        });
                    }
                }
                if 实参.len() != ps.len() {
                    return Err(BuildError {
                        message: format!(
                            "generic type \"{}\" requires {} type parameter(s), {} given",
                            n,
                            ps.len(),
                            实参.len()
                        ),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    });
                }
                INST_TODO.with(|q| q.borrow_mut().push((n.to_string(), 实参.clone())));
                let 名 = format!("{}({})", n, 实参.join(", "));
                return Ok((Ty(名), None, 尽 + 1));
            }
            if matches!(段.get(at + 1), Some(Token::Sym("*"))) {
                Ok((Ty(format!("{}*", n)), None, at + 2))
            } else {
                Ok((Ty(n.to_string()), None, at + 1))
            }
        }
        Some(Token::HashColon) | Some(Token::HashHashColon) => {
            let cross_mod = matches!(段.get(at), Some(Token::HashHashColon));
            let (blk_mod, colon_at) = if cross_mod {
                let mut m = String::new();
                let mut j = at + 1;
                while let Some(tok) = 段.get(j) {
                    match tok {
                        Token::Ident(x) | Token::Str(x) => {
                            if !m.is_empty() {
                                m.push('.');
                            }
                            m.push_str(x);
                            j += 1;
                            if matches!(段.get(j), Some(Token::Sym("."))) {
                                j += 1;
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
                if m.is_empty() || !matches!(段.get(j), Some(Token::HashColon)) {
                    return err("`##:` must be followed by a module name then `#:block` - e.g. `##:frontend.tokenizer #:split`");
                }
                (m, j)
            } else {
                (String::new(), at)
            };
            let blk = match 段.get(colon_at + 1) {
                Some(Token::Ident(n)) => n.to_string(),
                Some(Token::Kw(k)) => (*k).to_string().into(),
                _ => return err("`#:` must be followed by a block name"),
            };
            if !matches!(段.get(colon_at + 2), Some(Token::Kw("type"))) {
                return err("to take a type across blocks write `#:block type \"name\"`");
            }
            let tname = match 段.get(colon_at + 3) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => return err("`#:block type` must be followed by a type name"),
            };
            let mut end = colon_at + 4;
            let ty = if matches!(段.get(end), Some(Token::Sym("*"))) {
                end += 1;
                Ty(format!("{}*", tname))
            } else {
                Ty(tname.to_string())
            };
            Ok((ty, Some((blk_mod, blk.to_string(), tname.to_string())), end))
        }
        _ => err("expected a type here (builtin type name / user-defined type name / `#:block type \"name\"`)"),
    }
}


fn parse_params(段: &[Token]) -> Result<(Vec<(String, Ty)>, usize), BuildError> {
    let open = 段
        .iter()
        .position(|t| *t == Token::Sym("("))
        .ok_or_else(|| BuildError { message: "add fn is missing a parameter list".into() , code: prion_error::Code::E301 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
    let mut depth = 0i32;
    let mut end = None;
    for (k, t) in 段.iter().enumerate().skip(open) {
        match t {
            Token::Sym("(") => depth += 1,
            Token::Sym(")") => {
                depth -= 1;
                if depth == 0 {
                    end = Some(k);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.ok_or_else(|| BuildError { message: "the add fn parameter list is not closed".into() , code: prion_error::Code::E301 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
    let mut out = Vec::new();
    for a in split_args(&段[open + 1..end]) {
        let name = match a.first() {
            Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
            _ => return err(format!("parameter is missing a name: {:?}", a)),
        };
        let (ty, _限定, _) = match a.get(1) {
            Some(_) => parse_type_ref(&a, 1)?,
            None => return err(format!("parameter \"{}\" is missing a type", name)),
        };
        out.push((name, ty));
    }
    Ok((out, end + 1))
}

fn between_parens(段: &[Token]) -> Result<Vec<Token>, BuildError> {
    let open = 段
        .iter()
        .position(|t| *t == Token::Sym("("))
        .ok_or_else(|| BuildError { message: "print is missing (".into() , code: prion_error::Code::E301 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })?;
    let mut depth = 0i32;
    let mut out = Vec::new();
    for t in &段[open..] {
        match t {
            Token::Sym("(") => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            Token::Sym(")") => {
                depth -= 1;
                if depth == 0 {
                    return Ok(out);
                }
            }
            _ => {}
        }
        out.push(t.clone());
    }
    err("parentheses are not closed")
}

fn split_case_guard(t: &[Token]) -> (&[Token], Option<&[Token]>) {
    let mut d = 0i32;
    for (i, tk) in t.iter().enumerate() {
        match tk {
            Token::Sym("(") | Token::Sym("[") | Token::Sym("{") => d += 1,
            Token::Sym(")") | Token::Sym("]") | Token::Sym("}") => d -= 1,
            Token::Kw("if") if d == 0 => return (&t[..i], Some(&t[i + 1..])),
            _ => {}
        }
    }
    (t, None)
}

fn split_args(inner: &[Token]) -> Vec<Vec<Token>> {
    let mut out: Vec<Vec<Token>> = Vec::new();
    let mut cur = Vec::new();
    let mut depth = 0i32;
    for t in inner {
        match t {
            Token::Sym("(") => depth += 1,
            Token::Sym(")") => depth -= 1,
            
            Token::Sym("[") | Token::Sym("{") => depth += 1,
            Token::Sym("]") | Token::Sym("}") => depth -= 1,
            Token::Sym(",") if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(t.clone());
    }
    if !cur.is_empty() || !out.is_empty() {
        out.push(cur);
    }
    out.into_iter().filter(|a| !a.is_empty()).collect()
}



fn is_add_type(段: &[Token]) -> bool {
    matches!(段.first(), Some(Token::Kw("add"))) && matches!(段.get(1), Some(Token::Kw("type")))
}


fn parse_type_def(段: &[Token]) -> Result<Option<StructType>, BuildError> {
    let name = match 段.get(2) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
        _ => return err("expected a type name after add type"),
    };
    
    let mut params: Vec<String> = Vec::new();
    let mut 名后 = 3usize;
    if matches!(段.get(3), Some(Token::Sym("("))) {
        let mut 深度 = 0i32;
        let mut 尽 = None;
        for (k, t) in 段.iter().enumerate().skip(3) {
            match t {
                Token::Sym("(") => 深度 += 1,
                Token::Sym(")") => {
                    深度 -= 1;
                    if 深度 == 0 {
                        尽 = Some(k);
                        break;
                    }
                }
                _ => {}
            }
        }
        let 尽 = 尽.ok_or_else(|| BuildError {
            message: "the generic parameter list parentheses are not closed".into(),
            code: prion_error::Code::E201,
            line: CUR_LINE.with(|c| c.get()),
            file: String::new(),
        })?;
        for 项 in split_args(&段[4..尽]) {
            match 项.as_slice() {
                [Token::Ident(n)] => params.push(n.to_string()),
                _ => {
                    return Err(BuildError {
                        message: "generic parameters may only be bare names (`add type \"pair\"(A, B) = ...`)".into(),
                        code: prion_error::Code::E201,
                        line: CUR_LINE.with(|c| c.get()),
                        file: String::new(),
                    })
                }
            }
        }
        if params.is_empty() {
            return Err(BuildError {
                message: "the generic parameter list parentheses are empty".into(),
                code: prion_error::Code::E201,
                line: CUR_LINE.with(|c| c.get()),
                file: String::new(),
            });
        }
        名后 = 尽 + 1;
    }
    let eq = match 段.iter().skip(名后).position(|t| *t == Token::Sym("=")) {
        Some(k) => 名后 + k,
        None => {
            return err(
                "add type requires `=` - product type `add type \"name\" = (fields...)`; sum type `add type \"name\" = variant(...) | ...`",
            )
        }
    };
    
    if matches!(段.get(eq + 1), Some(Token::Sym("("))) {
        let fields = parse_field_list(&段[eq + 2..])?;
        return Ok(Some(StructType { name: name.to_string(), params, fields, variants: Vec::new() }));
    }
    
    if matches!(段.get(eq + 1), Some(Token::Kw(_)) | Some(Token::Ident(_))) {
        let has_bar = 段[eq + 1..].iter().any(|t| *t == Token::Sym("|"));
        if !has_bar { return Ok(None); }
    }
    
    let mut variants = Vec::new();
    for seg in split_top(&段[eq + 1..]) {
        if seg.is_empty() {
            continue;
        }
        let vname = match seg.first() {
            Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
            _ => return err("variant is missing a name"),
        };
        let fields = if matches!(seg.get(1), Some(Token::Sym("("))) {
            parse_variant_fields(&seg[2..])?
        } else {
            Vec::new()
        };
        variants.push(Variant { name: vname.to_string(), fields });
    }
    if variants.is_empty() {
        return err("a sum type needs at least one variant");
    }
    Ok(Some(StructType { name: name.to_string(), params, fields: Vec::new(), variants }))
}


fn parse_field_list(段: &[Token]) -> Result<Vec<(String, Ty)>, BuildError> {
    let end = matching_close(段)?;
    let mut fields = Vec::new();
    for a in split_args(&段[..end]) {
        let (fname, 尾) = match a.first() {
            Some(Token::Ident(n)) | Some(Token::Str(n)) => (n.to_string(), &a[1..]),
            
            Some(Token::Kw(_)) | Some(Token::Int(_)) => ((fields.len() + 1).to_string(), &a[..]),
            _ => return err("field is missing a name"),
        };
        
        
        if 尾.is_empty() {
            return err(format!("field \"{}\" is missing a type", fname));
        }
        let mut fty = String::new();
        for t in 尾.iter() {
            match t {
                Token::Kw(x) => {
                    if !fty.is_empty()
                        && !fty.ends_with('(')
                        && !fty.ends_with(',')
                        && !fty.ends_with(' ')
                    {
                        fty.push(' ');
                    }
                    fty.push_str(x);
                }
                Token::Ident(x) => {
                    if !fty.is_empty()
                        && !fty.ends_with('(')
                        && !fty.ends_with(',')
                        && !fty.ends_with(' ')
                    {
                        fty.push(' ');
                    }
                    fty.push_str(x);
                }
                Token::Sym("(") => fty.push('('),
                Token::Sym(")") => fty.push(')'),
                Token::Sym(",") => fty.push_str(", "),
                Token::Sym("*") => fty.push('*'),
                _ => {}
            }
        }
        fields.push((fname, Ty(fty)));
    }
    Ok(fields)
}



fn parse_variant_fields(段: &[Token]) -> Result<Vec<(String, Ty)>, BuildError> {
    let end = matching_close(段)?;
    let mut fields = Vec::new();
    for (idx, a) in split_args(&段[..end]).into_iter().enumerate() {
        
        let (fname, fty_tok) = if a.len() >= 2 && !matches!(a.get(1), Some(Token::Sym("*"))) {
            match a.first() {
                Some(Token::Ident(n)) | Some(Token::Str(n)) => (n.to_string(), a.get(1)),
                _ => (format!("#{}", idx), a.first()),
            }
        } else {
            (format!("#{}", idx), a.first())
        };
        let fty = match fty_tok {
            Some(Token::Kw(t)) => (*t).to_string(),
            Some(Token::Ident(n)) => n.to_string(),
            _ => return err(format!("variant field \"{}\" is missing a type", fname)),
        };
        
        let fty = if a.iter().any(|t| matches!(t, Token::Sym("*"))) {
            format!("{}*", fty)
        } else {
            fty
        };
        fields.push((fname, Ty(fty)));
    }
    Ok(fields)
}


fn int_literal_val(段: &[Token]) -> Option<i64> {
    match 段 {
        [Token::Int(v)] => Some(*v),
        [Token::Sym("-"), Token::Int(v)] => Some(-v),
        _ => None,
    }
}


fn width_range(ty: &str) -> Option<(i64, i64)> {
    match ty {
        "int_8" => Some((-128, 127)),
        "uint_8" => Some((0, 255)),
        "int_16" => Some((-32_768, 32_767)),
        "uint_16" => Some((0, 65_535)),
        "int_32" => Some((-2_147_483_648, 2_147_483_647)),
        "uint_32" => Some((0, 4_294_967_295)),
        "int_64" | "uint_64" | _ => None,
    }
}

fn matching_close(段: &[Token]) -> Result<usize, BuildError> {
    let mut depth = 0i32;
    for (k, t) in 段.iter().enumerate() {
        match t {
            Token::Sym("(") => depth += 1,
            Token::Sym(")") => {
                if depth == 0 {
                    return Ok(k);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    Err(BuildError { message: "the field list is not closed".into() , code: prion_error::Code::E301 , line: CUR_LINE.with(|c| c.get()) , file: String::new() })
}


fn split_top(段: &[Token]) -> Vec<Vec<Token>> {
    let mut out = Vec::new();
    let mut cur: Vec<Token> = Vec::new();
    let mut depth = 0i32;
    for t in 段 {
        match t {
            Token::Sym("(") => {
                depth += 1;
                cur.push(t.clone());
            }
            Token::Sym(")") => {
                depth -= 1;
                cur.push(t.clone());
            }
            Token::Sym(s) if s.starts_with('|') && s.len() == 1 && depth == 0 => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(t.clone()),
        }
    }
    out.push(cur);
    out
}




fn take_kind(段: &[Token]) -> Option<&'static str> {
    let mut j = 2usize;
    while let Some(tok) = 段.get(j) {
        match tok {
            Token::Ident(_) | Token::Str(_) => {
                j += 1;
                if matches!(段.get(j), Some(Token::Sym("."))) {
                    j += 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    if !matches!(段.get(j), Some(Token::HashColon)) {
        return None;
    }
    match 段.get(j + 2) {
        Some(Token::Kw("val")) => Some("val"),
        Some(Token::Kw("fn")) => Some("fn"),
        _ => None,
    }
}

fn is_take_form(段: &[Token]) -> bool {
    let mut j = 2usize;
    while let Some(tok) = 段.get(j) {
        match tok {
            Token::Ident(_) | Token::Str(_) => {
                j += 1;
                if matches!(段.get(j), Some(Token::Sym("."))) {
                    j += 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    matches!(段.get(j), Some(Token::HashColon))
        && (matches!(段.get(j + 2), Some(Token::Kw("val")))
            || matches!(段.get(j + 2), Some(Token::Kw("fn"))))
}



fn 预扫常量fn(stmts: &[AstStmt]) {
    for s in stmts {
        match s {
            AstStmt::Line {
                kind: prion_parser::LineKind::AddFn(段),
                ..
            } => {
                if 段.iter().position(|t| *t == Token::Sym("=")).is_none() {
                    continue;
                }
                let name = match 段.get(2) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => continue,
                };
                let Ok((params, after)) = parse_params(段) else {
                    continue;
                };
                let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) else {
                    continue;
                };
                let 体 = 段[eq + 1..].to_vec();
                let 形参名: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
                常量fn表.with(|t| {
                    t.borrow_mut().entry(name.to_string()).or_insert((形参名, 体));
                });
            }
            AstStmt::Struct { body, .. } | AstStmt::Fn { body, .. } => 预扫常量fn(body),
            _ => {}
        }
    }
}

fn 预扫常量(stmts: &[AstStmt], 表: &std::rc::Rc<RefCell<BTreeMap<String, crate::consts::Const>>>) {
    for s in stmts {
        match s {
            AstStmt::Line {
                kind: prion_parser::LineKind::AddVal(段),
                ..
            } => {
                if !matches!(段.get(1), Some(Token::Kw("fix"))) {
                    continue;
                }
                let name = match 段.get(2) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => continue,
                };
                let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) else {
                    continue;
                };
                let 右 = &段[eq + 1..];
                let 已有 = 表.borrow();
                let c = crate::consts::折叠(右).or_else(|| {
                    crate::consts::折叠带环境(右, &mut |n: &str| 已有.get(n).cloned(), &mut |_, _| None)
                });
                drop(已有);
                if let Some(c) = c {
                    表.borrow_mut().entry(name.to_string()).or_insert(c);
                }
            }
            AstStmt::Struct { body, .. } | AstStmt::Fn { body, .. } => 预扫常量(body, 表),
            _ => {}
        }
    }
}

fn build_block(
    b: &AstBlock,
    types: &[StructType],
    type_idx: &std::rc::Rc<std::collections::HashMap<String, usize>>,
    global_fns: &BTreeMap<String, FnSig>,
    cross_vals: &BTreeMap<String, BTreeMap<String, Ty>>,
    sum_variants: &BTreeMap<String, (String, usize, Vec<Ty>)>,
    mod_by_tail: &BTreeMap<String, Vec<String>>,
    pub_blocks: &BTreeSet<String>,
    hidden_items: &BTreeSet<String>,
    block_types: &BTreeMap<String, BTreeSet<String>>,
    常量值: std::rc::Rc<RefCell<BTreeMap<String, crate::consts::Const>>>,
) -> Result<(Function, Vec<Function>, Vec<ExternFn>), BuildError> {
    let mut bx = Builder::new(
        b.name.clone(),
        b.file.clone(),
        true, 
        b.attrs.iter().any(|a| a == "pub"),
        b.attrs.iter().any(|a| a == "sle"),
        b.attrs.iter().any(|a| a == "ABI"),
        b.attrs.iter().any(|a| a == "asm"),
        b.asm_lines.clone(),
        b.name.clone(),
        b.module.clone(),
        std::rc::Rc::new(types.to_vec()),
        std::rc::Rc::clone(type_idx),
        std::rc::Rc::new(mod_by_tail.clone()),
        std::rc::Rc::new(global_fns.clone()),
        std::rc::Rc::new(cross_vals.clone()),
        std::rc::Rc::new(sum_variants.clone()),
        std::rc::Rc::new(pub_blocks.clone()),
        std::rc::Rc::new(hidden_items.clone()),
        std::rc::Rc::new(block_types.clone()),
        std::rc::Rc::clone(&常量值),
    );
    
    if let Some(vals) = cross_vals.get(&format!("{}|{}", b.module, b.name)) {
        for (vname, vty) in vals {
            let pid = bx.new_local(Some(vname.clone()), Ty(format!("{}*", vty.0)), true, false);
            bx.exposed.insert(vname.clone(), pid);
        }
    }
    bx.body(&b.stmts)?;
    let extra = std::mem::take(&mut bx.extra);
    let externs = std::mem::take(&mut bx.externs);
    Ok((bx.finish()?, extra, externs))
}


fn 预扫泛型语句体(stmts: &[AstStmt], module: &str, block: &str) {
    for s in stmts {
        match s {
            AstStmt::Fn { head, body, .. } => {
                if let Some((name, 类型参数, 形参, 返回)) = 解析泛型语句头(head) {
                    泛型语句体fn表.with(|g| {
                        g.borrow_mut().insert(
                            name,
                            (类型参数, 形参, 返回, body.clone(), module.to_string(), block.to_string()),
                        );
                    });
                }
                预扫泛型语句体(body, module, block);
            }
            AstStmt::Struct { body, .. } => 预扫泛型语句体(body, module, block),
            _ => {}
        }
    }
}


fn 注入槽参数_prep(fb: &mut Builder, key: &str) -> Vec<(String, Ty)> {
    let used = fn_used表.with(|g| g.borrow().get(key).cloned()).unwrap_or_default();
    for (vn, vty) in &used {
        let pid = fb.new_local(Some(vn.clone()), Ty(format!("{}*", vty.0)), true, false);
        fb.exposed.insert(vn.clone(), pid);
    }
    used.into_iter().collect()
}


fn 是裸join(t: Option<&Token>) -> bool {
    match t {
        Some(Token::Kw("join")) => true,
        Some(Token::Ident(n)) => n.as_ref() == "join",
        _ => false,
    }
}

fn 解析token载荷(head: &[Token]) -> String {
    let mut i = 0usize;
    while let Some(t) = head.get(i) {
        let 是token = matches!(t, Token::Ident(n) if n.as_ref() == "token") || matches!(t, Token::Kw("token"));
        if 是token && matches!(head.get(i + 1), Some(Token::Sym("("))) {
            let mut s = String::new();
            let mut j = i + 2;
            while let Some(tk) = head.get(j) {
                match tk {
                    Token::Sym(")") => return if s.is_empty() { "int".into() } else { s },
                    Token::Kw(k) => s.push_str(k),
                    Token::Ident(n) | Token::Str(n) => s.push_str(n),
                    Token::Sym("*") => s.push('*'),
                    Token::Sym(",") => s.push_str(", "),
                    _ => {}
                }
                j += 1;
            }
        }
        i += 1;
    }
    "int".into()
}


fn head_tokens_name(head: &TokStream) -> Option<String> {
    let mut i = 0usize;
    while let Some(tk) = head.get(i) {
        if let Token::Kw("fn") = tk {
            return match head.get(i + 1) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => Some(n.to_string()),
                _ => None,
            };
        }
        i += 1;
    }
    None
}


fn 扫行引用(
    段: &TokStream,
    module: &str,
    block: &str,
    fname: &str,
    fn局部: &BTreeSet<String>,
    val_tys: &BTreeMap<String, Ty>,
    fn_used: &mut BTreeMap<String, BTreeMap<String, Ty>>,
    fn_calls: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let key = format!("{}|{}|{}", module, block, fname);
    let 规范 = prion_parser::建val规范(段);
    let 段: &[Token] = &规范;
    let mut i = 0usize;
    while let Some(tk) = 段.get(i) {
        match tk {
            
            Token::Kw("val") => {
                let 前是建 = i > 0
                    && (matches!(段.get(i - 1), Some(Token::Kw("add")))
                        || (i > 1 && matches!(段.get(i - 2), Some(Token::Kw("hide")))));
                if !前是建 {
                    if let Some(Token::Str(v)) | Some(Token::Ident(v)) = 段.get(i + 1) {
                        if !fn局部.contains(v.as_ref()) {
                            if let Some(ty) = val_tys
                                .get(&format!("{}|{}|{}", module, block, v))
                                .cloned()
                            {
                                fn_used.entry(key.clone()).or_default().insert(v.to_string(), ty);
                            }
                        }
                    }
                }
                i += 2;
            }
            
            Token::Kw("call") if matches!(段.get(i + 1), Some(Token::Kw("val"))) => {
                if let Some(Token::Str(v)) | Some(Token::Ident(v)) = 段.get(i + 2) {
                    if !fn局部.contains(v.as_ref()) {
                        if let Some(ty) = val_tys
                            .get(&format!("{}|{}|{}", module, block, v))
                            .cloned()
                        {
                            fn_used.entry(key.clone()).or_default().insert(v.to_string(), ty);
                        }
                    }
                }
                i += 3;
            }
            
            Token::Kw("call") if matches!(段.get(i + 1), Some(Token::Kw("fn"))) => {
                if let Some(Token::Str(c)) | Some(Token::Ident(c)) = 段.get(i + 2) {
                    fn_calls
                        .entry(key.clone())
                        .or_default()
                        .insert(format!("{}|{}|{}", module, block, c));
                }
                i += 3;
            }
            
            Token::Kw("call") if matches!(段.get(i + 1), Some(Token::HashColon)) => {
                if matches!(段.get(i + 3), Some(Token::Kw("fn"))) {
                    let blk = match 段.get(i + 2) {
                        Some(Token::Ident(b)) => b.to_string(),
                        Some(Token::Kw(b)) => (*b).to_string().into(),
                        _ => String::new().into(),
                    };
                    if let Some(Token::Str(c)) | Some(Token::Ident(c)) = 段.get(i + 4) {
                        if !blk.is_empty() {
                            fn_calls
                                .entry(key.clone())
                                .or_default()
                                .insert(format!("{}|{}|{}", module, blk, c));
                        }
                    }
                }
                i += 3;
            }
            
            Token::Ident(v) => {
                let 前是建 = i > 0
                    && (matches!(段.get(i - 1), Some(Token::Kw("add")))
                        || (i > 1 && matches!(段.get(i - 2), Some(Token::Kw("hide")))));
                if !前是建 && !fn局部.contains(v.as_ref()) {
                    if let Some(ty) =
                        val_tys.get(&format!("{}|{}|{}", module, block, v)).cloned()
                    {
                        fn_used.entry(key.clone()).or_default().insert(v.to_string(), ty);
                    }
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
}


#[allow(clippy::too_many_arguments)]
fn 扫fn引用(
    stmts: &[AstStmt],
    head: &TokStream,
    module: &str,
    block: &str,
    fname: &str,
    参数名: &BTreeSet<String>,
    val_tys: &BTreeMap<String, Ty>,
    fn_used: &mut BTreeMap<String, BTreeMap<String, Ty>>,
    fn_calls: &mut BTreeMap<String, BTreeSet<String>>,
) {
    
    let mut fn局部: BTreeSet<String> = 参数名.clone();
    收局部名(stmts, &mut fn局部);
    
    扫行引用(head, module, block, fname, &fn局部, val_tys, fn_used, fn_calls);
    for s in stmts {
        match s {
            AstStmt::Line { kind, .. } => {
                扫行引用(kind.tokens(), module, block, fname, &fn局部, val_tys, fn_used, fn_calls);
            }
            AstStmt::Struct { head, body, .. } => {
                
                扫行引用(head, module, block, fname, &fn局部, val_tys, fn_used, fn_calls);
                扫fn引用(body, head, module, block, fname, &fn局部, val_tys, fn_used, fn_calls);
            }
            _ => {}
        }
    }
}


fn 收局部名(stmts: &[AstStmt], out: &mut BTreeSet<String>) {
    for s in stmts {
        match s {
            AstStmt::Line { kind, .. } => {
                let 段 = kind.tokens();
                let mut i = 0usize;
                while let Some(tk) = 段.get(i) {
                    if matches!(tk, Token::Kw("add")) || matches!(tk, Token::Kw("hide")) {
                        let at = if matches!(tk, Token::Kw("hide")) { i + 2 } else { i + 1 };
                        if matches!(段.get(at), Some(Token::Kw("val"))) {
                            if let Some(Token::Str(v)) | Some(Token::Ident(v)) = 段.get(at + 1) {
                                out.insert(v.to_string());
                            }
                        }
                        i = at + 1;
                        continue;
                    }
                    i += 1;
                }
            }
            AstStmt::Struct { body, .. } | AstStmt::Fn { body, .. } => {
                收局部名(body, out);
            }
            _ => {}
        }
    }
}



thread_local! {
    static TYPECK: std::cell::RefCell<std::collections::BTreeMap<u32, String>> =
        std::cell::RefCell::new(std::collections::BTreeMap::new());
}

pub fn build(ast: &AstProgram, typeck: &std::collections::BTreeMap<u32, String>) -> Result<Program, BuildError> {
    TYPECK.with(|m| *m.borrow_mut() = typeck.clone());
    
    
    
    
    let mut types: Vec<StructType> = Vec::new();
    let mut type_first: BTreeMap<String, (String, usize)> = BTreeMap::new(); 
    
    let base_of = |f: &str| -> String { if f.is_empty() { "this file".to_string() } else { f.to_string() } };
    for b in &ast.blocks {
        let mut flat_ln: Vec<(usize, &TokStream)> = Vec::new();
        collect_ast_lines_ln(&b.stmts, &mut flat_ln);
        for (line, 段) in flat_ln {
                if is_add_type(段) {
                    if let Some(Token::Str(n) | Token::Ident(n)) = 段.get(2) {
                        if let Some((f0, l0)) = type_first.get(n.as_ref()) {
                            return Err(BuildError {
                                message: format!(
                                    "type \"{}\" is defined twice - first at {} line {}; type names are unique across the whole program (including across modules), rename it",
                                    n, base_of(f0), l0
                                ),
                                code: prion_error::Code::E201,
                                line: line,
                                file: b.file.clone(),
                            });
                        }
                        type_first.insert(n.to_string(), (b.file.clone(), line));
                    }
                    let Some(st) = parse_type_def(段)? else { continue };
                    for v in &st.variants {
                        if let Some((f0, l0)) = type_first.get(&v.name) {
                            return Err(BuildError {
                                message: format!(
                                    "variant \"{}\" is defined twice - first at {} line {}; variant names are unique across the whole program, rename it",
                                    v.name, base_of(f0), l0
                                ),
                                code: prion_error::Code::E201,
                                line: line,
                                file: b.file.clone(),
                            });
                        }
                        type_first.insert(v.name.clone(), (b.file.clone(), line));
                    }
                    types.push(st);
                }
                    }
    }

    
    for b in &ast.blocks {
        预扫泛型语句体(&b.stmts, &b.module, &b.name);
    }

    
    let mut global_fns: BTreeMap<String, FnSig> = BTreeMap::new();
    for b in &ast.blocks {
        
        
        let mut flat_lines: Vec<&TokStream> = Vec::new();
        collect_ast_lines(&b.stmts, &mut flat_lines);
        for 段 in flat_lines {
            if matches!(段.first(), Some(Token::Kw("add")))
                && matches!(段.get(1), Some(Token::Kw("fn")))
            {
                let name = match 段.get(2) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => continue,
                };
                if let Ok((params, after)) = parse_params(段) {

                    
                    if let Ok((ret, _限, _)) = parse_type_ref(段, after) {
                        global_fns.insert(
                            format!("{}|{}|{}", b.module, b.name, name),
                            FnSig {
                                params: params.iter().map(|(_, t)| t.clone()).collect(),
                                ret,
                                is_outs: false,
                                hidden: Vec::new(),
                            },
                        );
                    }
                }
            }
            
            if matches!(段.first(), Some(Token::Kw("add")))
                && matches!(段.get(1), Some(Token::Kw("outs")))
                && matches!(段.get(2), Some(Token::Kw("fn")))
            {
                let name = match 段.get(3) {
                    Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                    _ => continue,
                };
                if let Ok((params, after)) = parse_params(段) {
                    if let Ok((ret, _限, _)) = parse_type_ref(段, after) {
                        global_fns.insert(
                            format!("{}|{}|{}", b.module, b.name, name),
                            FnSig {
                                params: params.iter().map(|(_, t)| t.clone()).collect(),
                                ret,
                                is_outs: true,
                                hidden: Vec::new(),
                            },
                        );
                    }
                }
            }
                }
    }

    
    
    let mut val_tys: BTreeMap<String, Ty> = BTreeMap::new();
    for b in &ast.blocks {
        let mut flat_lines: Vec<&TokStream> = Vec::new();
        collect_ast_lines(&b.stmts, &mut flat_lines);
        for 段 in flat_lines {
                let 规范 = prion_parser::建val规范(段);
                let 段 = &规范[..];
                if matches!(段.first(), Some(Token::Kw("add")))
                    && matches!(段.get(1), Some(Token::Kw("val")))
                {
                    let name = match 段.get(2) {
                        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                        _ => continue,
                    };
                    let ty = match container_ty_at(段, 3) {
                        Some(t) => Ty(t),
                        None => match 段.get(3) {
                            Some(Token::Kw(t)) => Ty((*t).to_string()),
                            Some(Token::Ident(n)) => {
                                if matches!(段.get(4), Some(Token::Sym("*"))) {
                                    Ty(format!("{}*", n))
                                } else {
                                    Ty(n.to_string())
                                }
                            }
                            _ => continue,
                        },
                    };
                    val_tys.insert(format!("{}|{}|{}", b.module, b.name, name), ty);
                }
                    }
    }
    
    let mut fn_used: BTreeMap<String, BTreeMap<String, Ty>> = BTreeMap::new();
    let mut fn_calls: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    
    for b in &ast.blocks {
        for s in &b.stmts {
            if let AstStmt::Fn { body, head, .. } = s {
                let fname = head_tokens_name(head);
                if let Some(fname) = fname {
                    let mut fn局部: BTreeSet<String> = BTreeSet::new();
                    let mut k = 0usize;
                    let mut 深度 = 0i32;
                    while let Some(tk) = head.get(k) {
                        match tk {
                            Token::Sym("(") => {
                                深度 += 1;
                                k += 1;
                                continue;
                            }
                            Token::Sym(")") => break,
                            _ => {}
                        }
                        if 深度 > 0 {
                            if let Token::Str(pn) | Token::Ident(pn) = tk {
                                fn局部.insert(pn.to_string());
                            }
                        }
                        k += 1;
                    }
                    收局部名(body, &mut fn局部);
                    扫fn引用(body, head, &b.module, &b.name, &fname, &fn局部, &val_tys, &mut fn_used, &mut fn_calls);
                }
            } else if let AstStmt::Line { kind, .. } = s {
                let 段 = kind.tokens();
                
                if matches!(段.first(), Some(Token::Kw("add")))
                    && matches!(段.get(1), Some(Token::Kw("fn")))
                    && 段.iter().any(|t| matches!(t, Token::Sym("=")))
                {
                    let fname = match 段.get(2) {
                        Some(Token::Str(n)) | Some(Token::Ident(n)) => Some(n.clone()),
                        _ => None,
                    };
                    if let Some(fname) = fname {
                        let mut 参数名: BTreeSet<String> = BTreeSet::new();
                        let mut k = 0usize;
                        let mut 深度 = 0i32;
                        while k < 段.len() {
                            match 段.get(k) {
                                Some(Token::Sym("(")) => {
                                    深度 += 1;
                                    k += 1;
                                    continue;
                                }
                                Some(Token::Sym(")")) => {
                                    if 深度 > 0 {
                                        深度 -= 1;
                                        if 深度 == 0 {
                                            k += 1;
                                            continue;
                                        }
                                    }
                                }
                                _ => {}
                            }
                            if 深度 > 0 {
                                if let Some(Token::Str(pn)) | Some(Token::Ident(pn)) = 段.get(k) {
                                    参数名.insert(pn.to_string());
                                }
                            }
                            k += 1;
                        }
                        扫行引用(段, &b.module, &b.name, &fname, &参数名, &val_tys, &mut fn_used, &mut fn_calls);
                    }
                }
            }
        }
    }
    
    let mut changed = true;
    while changed {
        changed = false;
        let calls: Vec<(String, BTreeSet<String>)> =
            fn_calls.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        for (caller, callees) in calls {
            let used = fn_used.get(&caller).cloned().unwrap_or_default();
            for c in callees {
                let cu = fn_used.get(&c).cloned().unwrap_or_default();
                for (vn, vt) in cu {
                    let e = fn_used.entry(caller.clone()).or_default();
                    if !e.contains_key(&vn) {
                        e.insert(vn, vt);
                        changed = true;
                    }
                }
            }
        }
    }
    
    for (k, sig) in global_fns.iter_mut() {
        if sig.hidden.is_empty() {
            if let Some(used) = fn_used.get(k) {
                sig.hidden = used.iter().map(|(v, t)| (v.to_string(), t.clone())).collect();
            }
        }
    }
    fn_used表.with(|g| *g.borrow_mut() = fn_used.clone());
    let mut cross_vals: BTreeMap<String, BTreeMap<String, Ty>> = BTreeMap::new();
    
    let mut 被引块: BTreeSet<String> = BTreeSet::new();
    for (k, _) in &fn_calls {
        let mut it = k.splitn(3, '|');
        let m = it.next().unwrap_or("");
        let blk = it.next().unwrap_or("");
        被引块.insert(format!("{}|{}", m, blk));
    }
    
    for (k, used) in &fn_used {
        let mut it = k.splitn(3, '|');
        let m = it.next().unwrap_or("");
        let blk = it.next().unwrap_or("");
        if !被引块.contains(&format!("{}|{}", m, blk)) {
            continue;
        }
        for (v, ty) in used {
            cross_vals
                .entry(format!("{}|{}", m, blk))
                .or_default()
                .insert(v.to_string(), ty.clone());
        }
    }
    fn_used表.with(|g| *g.borrow_mut() = fn_used.clone());

    let mut mod_by_tail: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for b in &ast.blocks {
        let m = &b.module;
        if !m.is_empty() {
            let tail = m.rsplit('.').next().unwrap_or(m).to_string();
            let v = mod_by_tail.entry(tail).or_default();
            if !v.contains(m) {
                v.push(m.clone());
            }
        }
    }
    for b in &ast.blocks {
        let mut flat_lines: Vec<&TokStream> = Vec::new();
        collect_ast_lines(&b.stmts, &mut flat_lines);
        for 段 in flat_lines {
                let mut i = 0usize;
                while i + 4 < 段.len() {
                    if matches!(段.get(i), Some(Token::Kw("call")))
                        && matches!(段.get(i + 1), Some(Token::HashColon))
                        && matches!(段.get(i + 3), Some(Token::Kw("val")))
                    {
                        let blk = match 段.get(i + 2) {
                            Some(Token::Ident(n)) => n.to_string(),
                            Some(Token::Kw(k)) => (*k).to_string().into(),
                            _ => { i += 1; continue; }
                        };
                        if let Some(Token::Str(v) | Token::Ident(v)) = 段.get(i + 4) {
                            let ty = val_tys.get(&format!("{}|{}|{}", b.module, blk, v)).cloned();
                            if let Some(ty) = ty {
                                cross_vals
                                    .entry(format!("{}|{}", b.module, blk))
                                    .or_default()
                                    .insert(v.to_string(), ty);
                            }
                        }
                    }
                    
                    if matches!(段.get(i), Some(Token::Kw("call")))
                        && matches!(段.get(i + 1), Some(Token::HashHashColon))
                    {
                        let mut m = String::new();
                        let mut j = i + 2;
                        while let Some(tok) = 段.get(j) {
                            match tok {
                                Token::Ident(n) | Token::Str(n) => {
                                    if !m.is_empty() {
                                        m.push('.');
                                    }
                                    m.push_str(n);
                                    j += 1;
                                    if matches!(段.get(j), Some(Token::Sym("."))) {
                                        j += 1;
                                    } else {
                                        break;
                                    }
                                }
                                _ => break,
                            }
                        }
                        if !m.is_empty()
                            && matches!(段.get(j), Some(Token::HashColon))
                            && matches!(段.get(j + 2), Some(Token::Kw("val")))
                        {
                            let blk = match 段.get(j + 1) {
                                Some(Token::Ident(n)) => n.to_string(),
                                Some(Token::Kw(k)) => (*k).to_string().into(),
                                _ => { i += 1; continue; }
                            };
                            if let Some(Token::Str(v) | Token::Ident(v)) = 段.get(j + 3) {
                                
                                let known = mod_by_tail.values().flatten().any(|x| x == &m);
                                let full = if known {
                                    m.clone()
                                } else {
                                    match mod_by_tail.get(m.rsplit('.').next().unwrap_or(&m)) {
                                        Some(v) if v.len() == 1 => v[0].clone(),
                                        _ => String::new(),
                                    }
                                };
                                if !full.is_empty() {
                                    let ty = val_tys.get(&format!("{}|{}|{}", full, blk, v)).cloned();
                                    if let Some(ty) = ty {
                                        cross_vals
                                            .entry(format!("{}|{}", full, blk))
                                            .or_default()
                                            .insert(v.to_string(), ty);
                                    }
                                }
                            }
                        }
                    }

                    
                    if matches!(段.get(i), Some(Token::Kw("take")))
                        && matches!(段.get(i + 1), Some(Token::HashColon))
                        && matches!(段.get(i + 3), Some(Token::Kw("val")))
                    {
                        let blk = match 段.get(i + 2) {
                            Some(Token::Ident(n)) => n.to_string(),
                            Some(Token::Kw(k)) => (*k).to_string().into(),
                            _ => { i += 1; continue; }
                        };
                        if let Some(Token::Str(v) | Token::Ident(v)) = 段.get(i + 4) {
                            let ty = val_tys.get(&format!("{}|{}|{}", b.module, blk, v)).cloned();
                            if let Some(ty) = ty {
                                cross_vals
                                    .entry(format!("{}|{}", b.module, blk))
                                    .or_default()
                                    .insert(v.to_string(), ty);
                            }
                        }
                    }
                    i += 1;
                }
                    }
    }

    
    let mut sum_variants: BTreeMap<String, (String, usize, Vec<Ty>)> = BTreeMap::new();
    for st in &types {
        for (tag, v) in st.variants.iter().enumerate() {
            sum_variants.insert(
                v.name.clone(),
                (st.name.clone(), tag, v.fields.iter().map(|(_, t)| t.clone()).collect()),
            );
        }
    }

    
    let mut block_types: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for b in &ast.blocks {
        let key = format!("{}|{}", b.module, b.name);
        let mut flat: Vec<&TokStream> = Vec::new();
        collect_ast_lines(&b.stmts, &mut flat);
        for 段 in flat {
                if is_add_type(段) {
                    if let Some(Token::Str(n) | Token::Ident(n)) = 段.get(2) {
                        block_types.entry(key.clone()).or_default().insert(n.to_string());
                    }
                }
                    }
    }


    let pub_blocks: BTreeSet<String> = ast
        .blocks
        .iter()
        .filter(|b| b.attrs.iter().any(|a| a == "pub"))
        .map(|b| format!("{}|{}", b.module, b.name))
        .collect();

    
    let mut hidden_items: BTreeSet<String> = BTreeSet::new();
    for b in &ast.blocks {
        let is_pub = pub_blocks.contains(&format!("{}|{}", b.module, b.name));
        let mut lines: Vec<(usize, &TokStream)> = Vec::new();
        collect_ast_lines_ln(&b.stmts, &mut lines);
        for (line_no, 段) in lines {
            if !matches!(段.first(), Some(Token::Kw("add")))
                || !matches!(段.get(1), Some(Token::Kw("hide")))
            {
                continue;
            }
            let (cat, what) = match 段.get(2) {
                Some(Token::Kw("fn")) => ("fn", "function"),
                Some(Token::Kw("type")) => ("type", "type"),
                Some(Token::Kw("fix")) => ("fix", "constant"),
                _ => {
                    return Err(BuildError {
                        message: format!(
                            "hide can only be applied to fn / type / fix - val is part of the public surface already and cannot be hidden on its own"
                        ),
                        code: prion_error::Code::E203,
                        line: line_no,
                        file: b.file.clone(),
                    })
                }
            };
            let name = match 段.get(3) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                _ => {
                    return Err(BuildError {
                        message: format!("`add hide {}` must be followed by a {} name", cat, what),
                        code: prion_error::Code::E203,
                        line: line_no,
                        file: b.file.clone(),
                    })
                }
            };
            if !is_pub {
                return Err(BuildError {
                    message: format!(
                        "block \"{}\" is not a pub block and cannot hide - hide is for opening a window in a pub block and then drawing the curtain",
                        b.name
                    ),
                    code: prion_error::Code::E203,
                    line: line_no,
                    file: b.file.clone(),
                });
            }
            hidden_items.insert(format!("{}|{}|{}|{}", b.module, b.name, cat, name));
        }
    }

    
    GENERICS.with(|g| {
        let mut g = g.borrow_mut();
        g.clear();
        for t in &types {
            if !t.params.is_empty() {
                g.insert(t.name.clone(), t.params.clone());
            }
        }
    });
    INST_TODO.with(|q| q.borrow_mut().clear());

    
    {
        let mut 线: Vec<(usize, &TokStream)> = Vec::new();
        for b in &ast.blocks {
            线.clear();
            collect_ast_lines_ln(&b.stmts, &mut 线);
            for (_l, 段) in &线 {
                for (k, t) in 段.iter().enumerate() {
                    if let Token::Ident(n) = t {
                        if 泛型判别(n) && matches!(段.get(k + 1), Some(Token::Sym("("))) {
                            let _ = parse_type_ref(段, k);
                        }
                    }
                }
            }
        }
    }
    let 预 = INST_TODO.with(|q| std::mem::take(&mut *q.borrow_mut()));
    展开实例(&mut types, 预);

    {
        let mut 可空名: BTreeSet<String> = BTreeSet::new();
        for b in &ast.blocks {
            let mut flat: Vec<(usize, &TokStream)> = Vec::new();
            collect_ast_lines_ln(&b.stmts, &mut flat);
            for (_ln, 段) in flat {
                for k in 0..段.len().saturating_sub(1) {
                    if matches!(段.get(k), Some(Token::Sym("?"))) {
                        let inner = match 段.get(k + 1) {
                            Some(Token::Kw(t)) => Some((*t).to_string()),
                            Some(Token::Ident(t)) => Some(t.to_string()),
                            _ => None,
                        };
                        if let Some(inner) = inner {
                            可空名.insert(format!("?{}", inner));
                        }
                    }
                }
            }
        }
        for name in 可空名 {
            if types.iter().any(|st| st.name == name) { continue; }
            let inner = name.trim_start_matches('?').to_string();
            types.push(StructType {
                name: name.clone(),
                params: Vec::new(),
                fields: Vec::new(),
                variants: vec![
                    Variant { name: "None".into(), fields: Vec::new() },
                    Variant { name: "Some".into(), fields: vec![("v".into(), Ty(inner))] },
                ],
            });
        }
    }

    let type_idx: std::rc::Rc<std::collections::HashMap<String, usize>> = std::rc::Rc::new(
        types.iter().enumerate().map(|(i, t)| (t.name.clone(), i)).collect(),
    );

    let mut functions = Vec::new();
    let mut externs: Vec<ExternFn> = Vec::new();
    let 共享常量: std::rc::Rc<RefCell<BTreeMap<String, crate::consts::Const>>> =
        std::rc::Rc::new(RefCell::new(BTreeMap::new()));
    for b in &ast.blocks {
        预扫常量fn(&b.stmts);
        预扫常量(&b.stmts, &共享常量);
    }
    for b in &ast.blocks {
        let (mut main, extra, ex) =
            build_block(b, &types, &type_idx, &global_fns, &cross_vals, &sum_variants, &mod_by_tail, &pub_blocks, &hidden_items, &block_types, std::rc::Rc::clone(&共享常量))?;
        
        
        main.name = format!("{}|{}", b.module, b.name);
        functions.push(main);
        functions.extend(extra);
        externs.extend(ex);
    }
    
    let 待实例 = INST_TODO.with(|q| std::mem::take(&mut *q.borrow_mut()));
    展开实例(&mut types, 待实例);

    
    let mut 名集: BTreeSet<String> = BTreeSet::new();
    for f in functions.iter() {
        名集.insert(f.name.clone());
        if let Some(短) = f.name.rsplit('|').next() {
            名集.insert(短.to_string());
        }
    }
    for f in functions.iter_mut() {
        临时直通(f);
    }
    let 区域 = 区型表(&types);
    for f in functions.iter_mut() {
        
        let mut 圈: Vec<(usize, usize)> = Vec::new();
        for b in f.blocks.iter() {
            if let Exit::Branch { targets, .. } = &b.exit {
                for t in targets.iter() {
                    if t.target < b.id {
                        圈.push((t.target, b.id));
                    }
                }
            }
        }
        let mut 复印表: Vec<Vec<Option<(String, i64)>>> = Vec::new();
        let mut 护标: BTreeSet<LocalId> = BTreeSet::new();
        for b in f.blocks.iter() {
            let mut 表 = vec![None; b.stmts.len()];
            
            let mut 本圈: Option<(usize, usize)> = None;
            for (h, c) in 圈.iter() {
                if *h <= b.id && b.id <= *c {
                    let 更内 = match 本圈 {
                        None => true,
                        Some((hh, cc)) => *h > hh || (*h == hh && *c < cc),
                    };
                    if 更内 {
                        本圈 = Some((*h, *c));
                    }
                }
            }
            if let Some((h, c)) = 本圈 {
                for i in 0..b.stmts.len() {
                    
                    let (tmp, 调名, 是调用) = match &b.stmts[i] {
                        Stmt::Assign { lhs, rhs: Rvalue::Call { name, .. } } => {
                            (lhs.base, name.as_str(), true)
                        }
                        Stmt::Assign { lhs, .. } => (lhs.base, "", false),
                        _ => continue,
                    };
                    let 是临时 = match f.locals.get(tmp) {
                        Some(l) => l.name.is_none(),
                        None => true,
                    };
                    let (dst, 位) = if !是调用 {
                        (tmp, i)
                    } else if 是临时 {
                        match b.stmts.get(i + 1) {
                            Some(Stmt::Assign { lhs, rhs }) => {
                                let src = match rhs {
                                    Rvalue::Const(Value::Local(u)) => *u,
                                    Rvalue::Use(p) => p.base,
                                    _ => continue,
                                };
                                if src != tmp {
                                    continue;
                                }
                                (lhs.base, i + 1)
                            }
                            _ => continue,
                        }
                    } else {
                        (tmp, i)
                    };
                    let Some(l) = f.locals.get(dst) else { continue };
                    if l.loop_local {
                        continue;
                    }
                    if 是调用 && !名集.contains(调名) {
                        continue;
                    }
                    if !跨圈(f, dst, h, c) {
                        continue;
                    }
                    match 复印方式(&l.ty.0, &区域, &types) {
                        复印::无需 => {}
                        复印::拷贝(名, 旗) => {
                            表[位] = Some((名, 旗));
                            护标.insert(dst);
                        }
                        复印::无法 => {
                            护标.insert(dst);
                        }
                    }
                }
            }
            复印表.push(表);
        }
        for (bi, b) in f.blocks.iter_mut().enumerate() {
            let mut 新: Vec<Stmt> = Vec::with_capacity(b.stmts.len());
            let mut 新宽: Vec<Span> = Vec::with_capacity(b.spans.len());
            for (i, s) in b.stmts.iter().enumerate() {
                let sp = b.spans.get(i).copied().unwrap_or_default();
                新.push(s.clone());
                新宽.push(sp);
                if let Some((名, 旗)) = &复印表[bi][i] {
                    let lhs = match s {
                        Stmt::Assign { lhs, .. } => lhs.clone(),
                        _ => unreachable!(),
                    };
                    let mut args = vec![Value::Local(lhs.base)];
                    if 名 != "#str_copy" {
                        args.push(Value::Int(*旗));
                    }
                    新.push(Stmt::Assign { lhs, rhs: Rvalue::Call { name: 名.clone(), args } });
                    新宽.push(sp);
                }
            }
            b.stmts = 新;
            b.spans = 新宽;
        }
    }
    
    禁裸传校验(&functions, &types)?;
    Ok(Program { functions, types, externs, objects: Vec::new(), owned_elem: Default::default(), 值入列: Default::default(), 平铺: Default::default(), owned_fields: Default::default() })
}


fn 禁裸传校验(functions: &[Function], _types: &[StructType]) -> Result<(), BuildError> {
    
    
    
    
    let 堆 = |ty: &Ty, f: &Function| -> bool {
        let _ = f;
        let n = ty.0.as_str();
        n.starts_with("list(") || n.starts_with("map(") || n.starts_with("que(")
            || n.starts_with("stack(")
    };
    for f in functions {
        for b in &f.blocks {
            for st in &b.stmts {
                let (did, sid) = match st {
                    Stmt::Assign { lhs, rhs } => match (lhs, rhs) {
                        (Place { base: did, proj: _ }, Rvalue::Use(Place { base: sid, proj: _ }))
                        | (Place { base: did, proj: _ }, Rvalue::Const(Value::Local(sid))) => (did, sid),
                        _ => continue,
                    },
                    _ => continue,
                };
                if did == sid {
                    continue; 
                }
                let Some(d) = f.locals.get(*did) else { continue };
                
                if d.name.is_none() || !堆(&d.ty, f) {
                    continue;
                }
                let Some(l) = f.locals.get(*sid) else { continue };
                
                if l.name.is_none() {
                    continue;
                }
                
                if l.name.as_deref().map(|n| n.contains('#')).unwrap_or(false) {
                    continue;
                }
                
                if l.is_param {
                    continue;
                }
                if !堆(&l.ty, f) {
                    continue;
                }
                return Err(BuildError {
                    message: format!(
                        "passing containers/str by bare name within a block is forbidden - \"{}\" and \"{}\" would share the same buffer. To hold, use take (val \"A\" = take val \"B\"), use call #:block across blocks, or compute a new value",
                        d.name.clone().unwrap_or_else(|| format!("v{}", did)),
                        l.name.clone().unwrap_or_else(|| format!("v{}", sid))
                    ),
                    code: prion_error::Code::E201,
                    line: 0,
                    file: String::new(),
                });
            }
        }
    }
    Ok(())
}


fn 展开实例(types: &mut Vec<StructType>, 待实例: Vec<(String, Vec<String>)>) {
    for (名, 实参) in 待实例 {
        let 实例名 = format!("{}({})", 名, 实参.join(", "));
        if types.iter().any(|t| t.name == 实例名) {
            continue;
        }
        let 模板 = match types.iter().find(|t| t.name == 名) {
            Some(t) => t.clone(),
            None => continue,
        };
        let 字段 = 模板
            .fields
            .iter()
            .map(|(n, ty)| (n.clone(), Ty(替换参数(&ty.0, &模板.params, &实参))))
            .collect();
        let 变体 = 模板
            .variants
            .iter()
            .map(|v| Variant {
                name: v.name.clone(),
                fields: v
                    .fields
                    .iter()
                    .map(|(n, ty)| (n.clone(), Ty(替换参数(&ty.0, &模板.params, &实参))))
                    .collect(),
            })
            .collect();
        types.push(StructType { name: 实例名, params: Vec::new(), fields: 字段, variants: 变体 });
    }
}