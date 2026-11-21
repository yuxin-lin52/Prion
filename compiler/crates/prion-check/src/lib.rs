










use prion_error::{render_check, CheckCode, DiagParts, Tone};
use prion_types::StorageClass;
use prion_lexer::Token;
use prion_parser::{Block as AstBlock, Expr, Program as AstProgram, Stmt as AstStmt, TokStream};
use std::collections::{BTreeMap, BTreeSet};


mod infer_ast;


#[derive(Debug, Clone, PartialEq)]
pub struct Diag {
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub file: String,
    pub code: String,
    
    pub message: String,
    
    pub parts: DiagParts,
    
    pub tone: Tone,
    pub is_error: bool,
}


#[derive(Debug, Clone, PartialEq)]
pub struct LocalInfo {
    pub block: String,
    pub name: String,
    pub ty: String,
    pub class: StorageClass,
    
    pub 键: (prion_types::Sym, prion_types::Sym),
}


const 空键: (prion_types::Sym, prion_types::Sym) = (prion_types::Sym(0), prion_types::Sym(0));


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymKind {
    
    Type,
    
    Fn,
    
    Val,
    
    Param,
    
    Block,
    
    Variant,
}


#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymKind,
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub block: String,
}


#[derive(Debug, Clone, PartialEq)]
pub struct CheckResult {
    pub diags: Vec<Diag>,
    
    pub typeck: std::collections::BTreeMap<u32, String>,
    pub locals: Vec<LocalInfo>,
    
    pub symbols: Vec<SymbolInfo>,
}

impl CheckResult {
    pub fn ok(&self) -> bool {
        !self.diags.iter().any(|d| d.is_error)
    }
    pub fn errors(&self) -> Vec<&Diag> {
        self.diags.iter().filter(|d| d.is_error).collect()
    }
}

const MAX_DIAGS: usize = 20;
const UNKNOWN: &str = "未知";


pub const BUILTINS: &[&str] = &[
    
    "asm",
    "print", "push", "len", "char_len", "nth_char", "empty", "has", "index", "pop", "insert", "remove",
    "clear", "sort", "slice", "concat", "code", "char", "put",
    "remove", "map", "que", "stack", "keys", "vals", "args", "read_file", "save_file", "exit",
    "join",

    "run",

    "sle_ms", "rnd",

    "read_8", "read_16", "read_32", "read_64", "write_8", "write_16", "write_32", "write_64",

    "str_to_c", "str_from_c",
];




type TypeTable = BTreeMap<String, (位置, String, Vec<(String, String, bool)>)>;

type 泛型表 = BTreeMap<
    String,
    (
        位置,
        Vec<String>,
        Vec<(String, String)>,
        Vec<(String, Vec<(String, String)>)>,
    ),
>;

struct Ctx {
    diags: Vec<Diag>,
    
    cur_file: String,
    
    names: prion_types::Names,
    locals: Vec<LocalInfo>,
    
    local_idx: std::collections::HashMap<(prion_types::Sym, prion_types::Sym), usize>,
    
    suppress_key: Option<(prion_types::Sym, prion_types::Sym)>,
    
    fns: BTreeMap<(prion_types::Sym, prion_types::Sym), FnSig>,
    
    types: TypeTable,
    
    泛型: 泛型表,
    
    泛型fn能力: BTreeMap<String, (Vec<String>, Vec<Vec<String>>, 位置, Vec<Token>, Vec<String>)>,
    
    类型参数作用域: BTreeMap<String, BTreeSet<String>>,
    
    常量: BTreeSet<String>,
    
    suppressed: usize,
    
    variants: BTreeSet<String>,
    
    变体所属: BTreeMap<String, String>,
    
    unsafe_blocks: BTreeSet<String>,
    
    val_decl_line: std::collections::HashMap<(prion_types::Sym, prion_types::Sym), usize>,
    
    fn_decl_line: std::collections::HashMap<(prion_types::Sym, prion_types::Sym), usize>,
    
    fns_mod: BTreeMap<String, FnSig>,
    
    fn_has_body: std::collections::HashSet<(prion_types::Sym, prion_types::Sym)>,
    
    bare_ok: BTreeSet<String>,
    
    val_has_value: std::collections::HashSet<(prion_types::Sym, prion_types::Sym)>,

    aliases: BTreeMap<String, String>,
}


const UNSAFE_ONLY: &[&str] = &[
    "read_8", "read_16", "read_32", "read_64", "write_8", "write_16", "write_32", "write_64",
    "str_to_c", "str_from_c",
    
    "asm",
];


#[derive(Debug, Clone)]
struct FnSig {
    params: Vec<String>,
    ret: String,
    
    is_outs: bool,
}


fn parse_fn_ty(s: &str) -> Option<(Vec<String>, String)> {
    let rest = s.strip_prefix("fn(")?;
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
    let 闭 = 闭?;
    let 段 = &rest[..闭];
    let 参 = if 段.trim().is_empty() {
        Vec::new()
    } else {
        split_type_list(段)
    };
    let 返 = rest[闭 + 1..].trim().to_string();
    if 返.is_empty() {
        return None;
    }
    Some((参, 返))
}

fn split_type_list(s: &str) -> Vec<String> {
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
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}


fn is_add_fn(段: &[Token]) -> bool {
    matches!(段.first(), Some(Token::Kw("add")))
        && (matches!(段.get(1), Some(Token::Kw("fn")))
            || (matches!(段.get(1), Some(Token::Kw("hide")))
                && matches!(段.get(2), Some(Token::Kw("fn")))))
}


fn is_add_outs_fn(段: &[Token]) -> bool {
    matches!(段.first(), Some(Token::Kw("add")))
        && matches!(段.get(1), Some(Token::Kw("outs")))
        && matches!(段.get(2), Some(Token::Kw("fn")))
}


#[derive(Clone, Copy)]
struct 位置 {
    行: usize,
    列: usize,
    长: usize,
}

impl std::fmt::Display for 位置 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.行)
    }
}

impl Ctx {
    
    fn 键(&self, block: &str, name: &str) -> Option<(prion_types::Sym, prion_types::Sym)> {
        Some((self.names.lookup(block)?, self.names.lookup(name)?))
    }

    
    fn add_local(&mut self, mut info: LocalInfo) -> usize {
        let b = self.names.intern(&info.block);
        let n = self.names.intern(&info.name);
        info.键 = (b, n);
        self.local_idx.insert(info.键, self.locals.len());
        self.locals.push(info);
        self.locals.len() - 1
    }

    
    fn find_local(&self, block: &str, name: &str) -> Option<&LocalInfo> {
        if self.is_suppressed(block, name) {
            return None;
        }
        let key = self.键(block, name)?;
        self.local_idx.get(&key).map(|&i| &self.locals[i])
    }

    
    fn is_suppressed(&self, block: &str, name: &str) -> bool {
        match (self.suppress_key, self.键(block, name)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }

    
    fn has_local(&self, block: &str, name: &str) -> bool {
        let Some(key) = self.键(block, name) else {
            return false;
        };
        if self.suppress_key == Some(key) {
            return false; 
        }
        if self.local_idx.contains_key(&key) {
            return true;
        }
        
        if let Some(plain) = block.split('#').next() {
            if plain != block && !plain.is_empty() {
                if let Some(pkey) = self.键(plain, name) {
                    if self.suppress_key != Some(pkey) {
                        return self.local_idx.contains_key(&pkey);
                    }
                }
            }
        }
        false
    }

    fn err(&mut self, 处: 位置, code: CheckCode, parts: &DiagParts, tone: Tone) {
    let line = 处.行;
        if self.diags.len() >= MAX_DIAGS {
            self.suppressed += 1;
            return; 
        }
        
        let mut parts = parts.clone();
        let code_s = code.number().to_string();
        let (de, dd) = prion_error::code_defaults(&code_s);
        if parts.预期.is_empty() {
            parts.预期 = de.to_string();
        }
        if parts.详解.is_empty() {
            parts.详解 = dd.to_string();
        }
        let (col, len) = (处.列, 处.长);
        self.diags.push(Diag {
            line,
            col,
            len,
            file: self.cur_file.clone(),
            code: code.number().to_string(),
            message: render_check(code, &parts, tone),
            parts,
            tone,
            is_error: true,
        });
    }
}





pub fn check(ast: &AstProgram) -> CheckResult {
    infer_ast::typeck清空();
    infer_ast::藏字段清空();
    let (类型表, 别名表) = scan_types(ast);
    let mut cx = Ctx {
        diags: Vec::new(),
        cur_file: String::new(),
        names: prion_types::Names::new(),
        locals: Vec::new(),
        local_idx: std::collections::HashMap::new(),
        suppress_key: None,
        fns: BTreeMap::new(),
        types: 类型表,
        泛型: scan_generics(ast),
        泛型fn能力: scan_generic_fns(ast),
        类型参数作用域: BTreeMap::new(),
        常量: scan_fix_names(ast),
        variants: scan_variants(ast),
        变体所属: scan_variant_owner(ast),
        unsafe_blocks: ast
            .blocks
            .iter()
            .filter(|b| b.attrs.iter().any(|a| a == "unsafe"))
            .map(|b| b.name.clone())
            .collect(),
        suppressed: 0,
        val_decl_line: std::collections::HashMap::new(),
        fn_decl_line: std::collections::HashMap::new(),
        fns_mod: BTreeMap::new(),
        fn_has_body: std::collections::HashSet::new(),
        bare_ok: BTreeSet::new(),
        val_has_value: std::collections::HashSet::new(),
        aliases: 别名表,
    };


    for b in &ast.blocks {
        for st in &b.stmts {
            let toks = match st { AstStmt::Line { kind, .. } => kind.tokens(), _ => continue };
            if !matches!(toks.first(), Some(Token::Kw("add"))) { continue; }
            let at = match toks.get(1) {
                Some(Token::Kw("val")) | Some(Token::Kw("fix")) | Some(Token::Kw("fn"))
                | Some(Token::Kw("type")) => 2,
                Some(Token::Kw("hide")) if matches!(toks.get(2), Some(Token::Kw("fn"))) => 3,
                _ => continue,
            };
            if let Some(Token::Kw(kw)) = toks.get(at) {
                let (line, col) = match st {
                    AstStmt::Line { line, col, .. } => (*line, *col),
                    _ => (0, 0),
                };
                cx.err(
                    位置 { 行: line, 列: col, 长: kw.chars().count() },
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "写声明",
                        format!("关键字「{}」不能作自定名", kw),
                        "",
                        "另起一个名字，或写带引号的 \"名\"（引号内允许用关键字作名字）",
                    ),
                    Tone::Fix,
                );
            }
        }
    }

    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        let is_unsafe_blk = b.attrs.iter().any(|a| a == "unsafe");
        
        
        let mut called_in_block: std::collections::BTreeSet<prion_lexer::Name> = std::collections::BTreeSet::new();
        for st3 in &b.stmts {
            if let AstStmt::Line { line: _, col: _, len: _, kind, .. } = st3 {
                let 段 = kind.tokens();
                let mut i = 0usize;
                while i < 段.len() {
                    if matches!(段.get(i), Some(Token::Kw("call"))) {
                        
                        let mut k = i + 2;
                        if matches!(段.get(i + 1), Some(Token::HashHashColon))
                            && matches!(段.get(i + 3), Some(Token::HashColon))
                        {
                            k = i + 4;
                        }
                        if let Some(Token::Ident(n)) = 段.get(k) {
                            called_in_block.insert((*n).clone());
                        }
                    }
                    i += 1;
                }
            }
        }
        
        
        let mut flat_b: Vec<(位置, &TokStream, bool)> = Vec::new();
        flatten_stmts(&b.stmts, false, &mut flat_b);
        for (处, 段, _nested) in flat_b.iter() {
            check_hidden_field_access(&mut cx, ast, 处, b, 段);
        }
        for (处, 段, nested) in flat_b.iter() {
            {
                let 段 = *段;
                if !matches!(段.first(), Some(Token::Kw("kill"))) {
                    continue;
                }
                let target = 段.iter().skip(1).filter_map(|t| match t {
                    Token::Ident(n) | Token::Str(n) => Some(n.clone()),
                    _ => None,
                }).next_back();
                let target = match target {
                    Some(t) => t,
                    None => continue,
                };
                
                if *nested {
                    cx.err(
                        *处,
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", 处.行),
                            "danger",
                            format!("`kill #:{}` 写在函数体或条件体里", target),
                            "",
                            "把 kill 挪到块体的顺序语句位置——fn 体与 if/while 体里不合法",
                        )
                        .预期("`kill` 只许写在块体的顺序语句位置")
                        .详解("`kill` 标注的是块链上的回收点；fn 体不在块链上，条件体让回收点依赖运行期值——都与回收点编译期定死相抵。"),
                        Tone::Fix,
                    );
                    continue;
                }
                if !is_unsafe_blk {
                    cx.err(
                        *处,
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", 处.行),
                            "danger",
                            format!("块「{}」里用了 `kill #:{}`，但它不是危险块", b.name, target),
                            "",
                            "把开块行写成 `#: 块名 b unsafe`——kill 与 asm 一样，只在危险模式里放行",
                        )
                        .预期("`kill` 语句只能出现在 unsafe 块里")
                        .详解("`kill` 是 unsafe 的第四类能力：它提前回收别的块的地皮，需开发者自己担保 ⇒ 只在危险块里合法。"),
                        Tone::Fix,
                    );
                }
                
                
                
                
                
                let mut taint: std::collections::BTreeMap<prion_lexer::Name, bool> =
                    std::collections::BTreeMap::new();
                let mut seen_kill = false;
                let mut violations: Vec<String> = Vec::new();
                for (ln, tk, nested) in flat_b.clone() {
                    let is_kill_line = matches!(tk.first(), Some(Token::Kw("kill")));

                    let eq_pos = tk.iter().position(|t| matches!(t, Token::Sym("=")));
                    let lhs_name: Option<&prion_lexer::Name> = match tk.first() {
                        Some(Token::Kw("add")) => match tk.get(2) {
                            Some(Token::Str(v)) => Some(v),
                            _ => None,
                        },
                        Some(Token::Kw("val")) => match tk.get(1) {
                            Some(Token::Str(v)) => Some(v),
                            _ => None,
                        },
                        _ => None,
                    };
                    let rhs_start = eq_pos.map(|p| p + 1).unwrap_or(0);
                    let rhs_names: Vec<&prion_lexer::Name> = tk[rhs_start..]
                        .iter()
                        .filter_map(|t| match t {
                            Token::Str(v) => Some(v),
                            _ => None,
                        })
                        .collect();

                    let mut refs_target = false;
                    let mut ci = 0usize;
                    while ci < tk.len() {
                        if matches!(tk.get(ci), Some(Token::Kw("call"))) {
                            let mut k = ci + 2;
                            if matches!(tk.get(ci + 1), Some(Token::HashHashColon))
                                && matches!(tk.get(ci + 3), Some(Token::HashColon))
                            {
                                k = ci + 4;
                            }
                            if let Some(Token::Ident(n)) = tk.get(k) {
                                if *n == target {
                                    refs_target = true;
                                }
                            }
                        }
                        ci += 1;
                    }

                    let rhs_tainted = rhs_names.iter().any(|v| taint.get(*v) == Some(&true));

                    if let Some(name) = lhs_name {
                        if refs_target || rhs_tainted {
                            taint.insert((*name).clone(), true);
                        } else if !nested {
                            
                            
                            taint.insert((*name).clone(), false);
                        }
                    }

                    if seen_kill && !is_kill_line {
                        for v in rhs_names.iter().chain(lhs_name.iter()) {
                            if taint.get(*v) == Some(&true) {
                                violations.push(format!("第 {} 行的「{}」", ln, v));
                            }
                        }
                    }
                    if is_kill_line {
                        seen_kill = true;
                    }
                }
                if !violations.is_empty() {
                    violations.dedup();
                    cx.err(
                        位置{ 行: b.line, 列: b.col, 长: b.len },
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", b.line),
                            "kill 追踪",
                            format!("`kill #:{}` 之后还有指向该块地皮的引用（{} 处）", target, violations.len()),
                            "",
                            "把这些引用在 kill 之前用完并清掉，或把 kill 移到它们之后",
                        )
                        .预期("kill 点之后不得存在指向待回收段的活引用")
                        .详解(&format!(
                            "编译器追踪到下面这些位置仍在引用「{}」的地皮：{}。\
                             kill 会连整棵子树一起回收 ⇒ 这些引用会变成悬空。\
                             （保守判定：流不敏感，宁可多报不可漏。）",
                            target,
                            violations.join("、")
                        )),
                        Tone::Fix,
                    );
                }

                
                if target != b.name && !called_in_block.contains(&target) {
                    cx.err(
                        位置{ 行: b.line, 列: b.col, 长: b.len },
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", b.line),
                            "kill 目标",
                            format!("`kill #:{}` 的目标块没有被本块调用过", target),
                            "",
                            "kill 只能回收**本链已经调用过**的块的活跃实例段；若只是想清理本块自己，用 `#: 块名 d kill`",
                        )
                        .预期("kill 的目标必须是本块调用过的块")
                        .详解(
                            "块的活跃实例段由调用产生：没调用过 ⇒ 没有可回收的段 ⇒ 这条 kill 是笔误\
                             （多半是块名写错，或该块其实没在这个链上被调用）。",
                        ),
                        Tone::Fix,
                    );
                }
                if target == b.name {
                    cx.err(
                        位置{ 行: b.line, 列: b.col, 长: b.len },
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", b.line),
                            "danger",
                            format!("`kill #:{}` 指向当前块自己", target),
                            "",
                            "kill 要回收**别的块**的活跃实例段；要本块跑完即清理，用关块行后缀 `#: 块名 d kill`",
                        )
                        .预期("kill 的目标必须是**非当前块**")
                        .详解("语句 `kill #:块名` 回收别的块的活跃实例段；`#: 块名 d kill` 回收本块自己——两个挂载点，别混。"),
                        Tone::Fix,
                    );
                }
            }
        }
    }

    
    
    
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        let has_kill = b.attrs.iter().any(|a| a == "kill");
        if !has_kill {
            continue;
        }
        let is_unsafe = b.attrs.iter().any(|a| a == "unsafe");
        if !is_unsafe {
            cx.err(
                位置{ 行: b.line, 列: b.col, 长: b.len },
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", b.line),
                    "danger",
                    format!("块「{}」用了 `d kill`，但它不是危险块", b.name),
                    "",
                    "把开块行写成 `#: 块名 b unsafe`——kill 与 asm 一样，只在危险模式里放行",
                )
                .预期("`d kill` 的块必须是 unsafe 块")
                .详解(
                    "`kill` 是 unsafe 的第四类能力（与裸地址读写、C 互操作、内联汇编并列）：\
                     它提前回收别的块的活跃地皮，属于需要开发者自己担保的事 ⇒ 只在危险块里合法。",
                ),
                Tone::Fix,
            );
        }
    }

    
    {
        let mut seen: std::collections::BTreeMap<(String, String), usize> =
            std::collections::BTreeMap::new();
        for b in &ast.blocks {
            cx.cur_file = b.file.clone();
            let key = (b.module.clone(), b.name.clone());
            if let Some(first) = seen.get(&key) {
                cx.err(
                    位置{ 行: b.line, 列: b.col, 长: b.len },
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", b.line),
                        "建块",
                        format!(
                            "块「{}」重名——同模块内块不许重名（第一次在第 {} 行）",
                            b.name, first
                        ),
                        "",
                        "换个块名；跨模块的同名块是合法的（引用时写 `##:模块 #:块` 区分）",
                    )
                    .预期("同一个模块里块名唯一")
                    .详解(
                        "块名在**同模块内**必须唯一：两个同名块会被当成两个链根各跑一遍，`call #:块` 也无从判断调哪个。\
                         不同模块可以有同名块——引用时 `##:模块 #:块` 天然区分。",
                    ),
                    Tone::Fix,
                );
            } else {
                seen.insert(key, b.line);
            }
        }
    }


    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        
        let a: Vec<&String> = b.attrs.iter().filter(|x| x.as_str() != "kill").collect();
        let line = b.line;
        if let Some(pos) = a.iter().position(|x| x.as_str() == "unsafe") {
            if pos != 0 {
                cx.err(
                    位置{ 行: b.line, 列: b.col, 长: b.len },
                    CheckCode::E203,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "块行属性",
                        format!("块「{}」属性顺序不合法：`unsafe` 未紧贴 `b`", b.name),
                        "",
                        "写成 `#: 名 b unsafe [pub|sle] [ABI]`",
                    ),
                    Tone::Fix,
                );
            }
        }
        if let Some(pos) = a.iter().position(|x| x.as_str() == "ABI") {
            if pos + 1 != a.len() {
                cx.err(
                    位置{ 行: b.line, 列: b.col, 长: b.len },
                    CheckCode::E203,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "块行属性",
                        format!("块「{}」属性顺序不合法：`ABI` 未殿后", b.name),
                        "",
                        "把 `ABI` 写到最后一个属性位",
                    ),
                    Tone::Fix,
                );
            }
            if !a.iter().any(|x| x.as_str() == "pub") {
                cx.err(
                    位置{ 行: b.line, 列: b.col, 长: b.len },
                    CheckCode::E203,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "块行属性",
                        format!("块「{}」标了 `ABI` 却没标 `pub`", b.name),
                        "",
                        "补上 `pub`（导出面必须显式开门，不做隐式蕴含）",
                    ),
                    Tone::Fix,
                );
            }
        }
    }

    
    pass1_collect(ast, &mut cx);
    
    check_caps(&mut cx);
    pass1_uses(ast, &mut cx);
    
    check_hide_decls(ast, &mut cx);
    
    pass2_types(ast, &mut cx);
    
    pass2_calls(ast, &mut cx);
    
    let t = cx.types.clone();
    check_type_cycles(&t, &mut cx);
    
    pass2_match_exhaustive(ast, &mut cx);
    
    check_conc(ast, &mut cx);
    
    pass3_memory(ast, &mut cx);
    
    pass_sle_calls(ast, &mut cx);
    
    infer_ast::对比(ast, &cx);

    
    if cx.suppressed > 0 {
        cx.diags.push(Diag {
            line: 0,
            col: 0,
            len: 0,
            file: String::new(),
            code: String::new(),
            message: format!(
                "提示：还有 {} 条未报告（单次诊断上限 {} 条）—— 先修上面这些，再编一次会露出后面的",
                cx.suppressed, MAX_DIAGS
            ),
            parts: DiagParts::default(),
            tone: Tone::Suggest,
            is_error: false,
        });
    }

    for (span, ty, field) in infer_ast::藏字段取() {
        cx.err(
            位置 { 行: span.line, 列: span.col, 长: span.len },
            CheckCode::E201,
            &DiagParts::full(
                format!("第 {} 行", span.line),
                "字段隐藏",
                format!("字段「{}」在类型「{}」上被 hide 了", field, ty),
                "",
                "跨模块只能读公开字段；要读请在本模块内提供取值函数",
            )
            .预期("hide 字段不在跨模块的可见面上"),
            Tone::Fix,
        );
    }

    CheckResult { diags: cx.diags, locals: cx.locals, symbols: collect_symbols(ast), typeck: infer_ast::typeck快照() }
}




pub fn collect_symbols(ast: &AstProgram) -> Vec<SymbolInfo> {
    let mut out = Vec::new();
    for b in &ast.blocks {
        out.push(SymbolInfo {
            name: b.name.clone(),
            kind: SymKind::Block,
            line: b.line,
            col: b.col,
            len: b.len,
            block: b.name.clone(),
        });
        collect_symbols_in(&b.stmts, &b.name, &mut out);
    }
    out
}


fn collect_symbols_in(stmts: &[AstStmt], 块: &str, out: &mut Vec<SymbolInfo>) {
    for s in stmts {
        let (line, col, len, 段): (usize, usize, usize, &TokStream) = match s {
            AstStmt::Line { line, col, len, kind, .. } => (*line, *col, *len, kind.tokens()),
            AstStmt::Struct { line, col, len, head, body, .. } | AstStmt::Fn { line, col, len, head, body, .. } => {
                登记头部(head, 位置{ 行: *line, 列: *col, 长: *len }, 块, out);
                collect_symbols_in(body, 块, out);
                continue;
            }
        };
        登记头部(段, 位置{ 行: line, 列: col, 长: len }, 块, out);
    }
}


fn 登记头部(段: &TokStream, 处: 位置, 块: &str, out: &mut Vec<SymbolInfo>) {
    let line = 处.行;
    let 规范 = prion_parser::建val规范流(段);
    let 段 = &规范;
    let kind = match (段.first(), 段.get(1)) {
        (Some(Token::Kw("add")), Some(Token::Kw("fn"))) => SymKind::Fn,
        (Some(Token::Kw("add")), Some(Token::Kw("val"))) => SymKind::Val,
        (Some(Token::Kw("add")), Some(Token::Kw("fix"))) => SymKind::Val,
        (Some(Token::Kw("add")), Some(Token::Kw("type"))) => SymKind::Type,
        _ => return,
    };
    let Some((Token::Str(名) | Token::Ident(名), (col, len))) = 段.get2(2) else {
        return;
    };
    out.push(SymbolInfo {
        name: 名.to_string(),
        kind,
        line,
        col,
        len,
        block: 块.to_string(),
    });
}



fn block_is_unsafe(block: &AstBlock, ast: &AstProgram) -> bool {
    
    

    let _ = (block, ast);
    false
}

fn pass1_collect(ast: &AstProgram, cx: &mut Ctx) {
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        collect_block(b, cx);
    }
}

fn collect_block(b: &AstBlock, cx: &mut Ctx) {
    collect_stmts(&b.stmts, b, cx);
}


fn stmt_fn_head(head: &[Token]) -> Vec<Token> {
    head.iter().cloned().filter(|t| *t != Token::Sym(":")).collect()
}


fn stmt_fn_name(head: &[Token]) -> Option<String> {
    let h = stmt_fn_head(head);
    let at = match (h.first(), h.get(1)) {
        (Some(Token::Kw("fn")), _) => 1,
        (Some(Token::Kw("add")), Some(Token::Kw("hide"))) => 3,
        _ => 2,
    };
    match h.get(at) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => Some(n.to_string()),
        _ => None,
    }
}


fn 泛型语句头(h: &[Token]) -> Option<(Vec<(String, Vec<String>)>, Vec<(String, String)>)> {
    let at = match (h.first(), h.get(1)) {
        (Some(Token::Kw("fn")), _) => 1,
        (Some(Token::Kw("add")), Some(Token::Kw("hide"))) => 3,
        _ => 2,
    };
    if !matches!(h.get(at + 1), Some(Token::Sym("("))) {
        return None;
    }
    let 止 = 配对括号(h, at + 1)?;
    let mut 参数: Vec<(String, Vec<String>)> = Vec::new();
    for 段 in h[at + 2..止].split(|t| *t == Token::Sym(",")) {
        match 段 {
            [Token::Ident(n)] => 参数.push((n.to_string(), Vec::new())),
            [Token::Ident(n), Token::Sym(op)] => 参数.push((n.to_string(), vec![(*op).to_string()])),
            _ => return None,
        }
    }
    if 参数.is_empty() {
        return None;
    }
    let (形参, _) = parse_params(&h[止 + 1..]).ok()?;
    Some((参数, 形参))
}


fn 找give(stmts: &[AstStmt], 出: &mut Vec<(usize, TokStream, Vec<Expr>)>) {
    for st in stmts {
        match st {
            AstStmt::Line { line, exprs, kind, .. } if matches!(*kind, prion_parser::LineKind::Give(_)) => {
                出.push((*line, kind.tokens().clone(), exprs.clone()));
            }
            AstStmt::Fn { body, .. } | AstStmt::Struct { body, .. } => 找give(body, 出),
            _ => {}
        }
    }
}


fn fn_scope_block(b: &AstBlock, fname: &str, body: &[AstStmt]) -> AstBlock {
    AstBlock {
        name: format!("{}#{}", b.name, fname),
        line: b.line,
        col: b.col,
        len: b.len,
        stmts: body.to_vec(),
        attrs: b.attrs.clone(),
        asm_lines: Vec::new(),
        module: b.module.clone(),
        file: b.file.clone(),
    }
}


fn 原块(block: &str) -> &str {
    block.split('#').next().unwrap_or(block)
}


fn collect_stmts(stmts: &[AstStmt], b: &AstBlock, cx: &mut Ctx) {
    for s in stmts {
        match s {
            AstStmt::Fn { line, col, len, head, body, .. } => {
                let h = stmt_fn_head(head);
                if matches!(h.first(), Some(Token::Kw("fn"))) {
                    collect_rebind_fn(b, 位置{ 行: *line, 列: *col, 长: *len }, &h, cx);
                } else {
                    collect_fn(b, 位置{ 行: *line, 列: *col, 长: *len }, &h, cx);
                }
                if let Some(n) = stmt_fn_name(head) {
                    cx.fn_has_body.insert((cx.names.intern(&b.name), cx.names.intern(&n)));
                    let pb = fn_scope_block(b, &n, body);
                    let 形参 = match 泛型语句头(&h) {
                        Some((类型参数, ps)) => {
                            cx.类型参数作用域.insert(
                                pb.name.clone(),
                                类型参数.iter().map(|(n, _)| n.clone()).collect(),
                            );
                            Some(ps)
                        }
                        None => parse_params(&h).ok().map(|(ps, _)| ps),
                    };
                    if let Some(params) = 形参 {
                        for (pn, pt) in params {
                            let key = (cx.names.intern(&pb.name), cx.names.intern(&pn));
                            if !cx.local_idx.contains_key(&key) {
                                cx.local_idx.insert(key, cx.locals.len());
                                cx.locals.push(LocalInfo {
                                    block: pb.name.clone(),
                                    name: pn.clone(),
                                    ty: pt,
                                    class: StorageClass::Tmp,
                                    键: key,
                                });
                            }
                        }
                    }
                    collect_stmts(body, &pb, cx);
                }
            }
            AstStmt::Line { line, col, len, exprs, kind, .. } => {
                let 段 = kind.tokens();
                
                collect_inline_case_bindings(b, 段, 0, cx);
                if matches!(*kind, prion_parser::LineKind::AddVal(_)) {
                    collect_val(b, 位置{ 行: *line, 列: *col, 长: *len }, 段, exprs, cx);
                }
                if matches!(*kind, prion_parser::LineKind::AddFn(_)) && matches!(段.get(1), Some(Token::Kw("fn"))) {
                    collect_fn(b, 位置{ 行: *line, 列: *col, 长: *len }, 段, cx);
                }
                if matches!(*kind, prion_parser::LineKind::AddOutsFn(_)) {
                    collect_outs_fn(b, 位置{ 行: *line, 列: *col, 长: *len }, 段, cx);
                }
                
                if matches!(*kind, prion_parser::LineKind::AddFn(_)) && matches!(段.first(), Some(Token::Kw("fn"))) {
                    collect_rebind_fn(b, 位置{ 行: *line, 列: *col, 长: *len }, 段, cx);
                }
                
                if matches!(*kind, prion_parser::LineKind::Assign(_)) {
                    collect_val_rebind(b, 位置{ 行: *line, 列: *col, 长: *len }, 段, cx);
                }
            }
            AstStmt::Struct { line, col, len, head, body, kind: sk, .. } => {
                if *sk == prion_parser::StructKind::Spawn {
                    if let Some(Token::Ident(n)) | Some(Token::Str(n)) = head.get(2) {
                        let ty = head.get(4).map(|t| match t {
                            Token::Kw(k) => (*k).to_string(),
                            Token::Ident(x) => x.to_string(),
                            _ => "token".to_string(),
                        }).unwrap_or_else(|| "token".to_string());
                        if !cx.has_local(&b.name, n) {
                            cx.add_local(LocalInfo {
                                block: b.name.clone(),
                                name: n.to_string(),
                                ty,
                                键: 空键,
                                class: prion_types::StorageClass::Tmp,
                            });
                        }
                    }
                    collect_stmts(body, b, cx);
                    continue;
                }
                if *sk == prion_parser::StructKind::Case {
                    if let Some(Token::Ident(n)) = head.get(1) {
                        if n != "_" && !matches!(head.get(2), Some(Token::Sym("("))) {
                            cx.err(
                                位置{ 行: *line, 列: *col, 长: *len },
                                CheckCode::E204,
                                &DiagParts::full(
                                    format!("第 {} 行", line),
                                    "match 判别",
                                    format!("无字段变体的判别也要写括号——`case {}()`", n),
                                    "",
                                    "判别统一带括号：有字段写 `case 变体(绑定)`，无字段写 `case 变体()`，兜底写 `case _:`",
                                ),
                                Tone::Fix,
                            );
                        }
                    }
                }
                
                collect_case_bindings(b, head, cx);
                
                if *sk == prion_parser::StructKind::For {
                    if let Some(Token::Ident(n)) = head.get(1) {
                        cx.err(
                            位置{ 行: *line, 列: *col, 长: *len },
                            CheckCode::E201,
                            &DiagParts::full(
                                format!("第 {} 行", line),
                                "循环变量",
                                format!("循环变量「{}」没加引号——自定义名一律带引号（块名除外）", n),
                                "",
                                format!("写成 `for \"{}\" in …`；体内读取写 val \"{}\"", n, n),
                            ),
                            Tone::Fix,
                        );
                    }
                    if let Some(Token::Str(n)) = head.get(1) {
                        let vname = (*n).clone();
                        if !cx.has_local(&b.name, &vname) {
                            let ty = iter_elem_ty(head, &cx.locals, &cx.local_idx, &cx.names, &b.name);
                            let _新号 = cx.add_local(LocalInfo {
                                block: b.name.clone(),
                                name: vname.to_string(),
                                ty,
                                键: 空键,
                                    class: StorageClass::Tmp,
                            });
                            
                        }
                    }
                }
                collect_stmts(body, b, cx);
            }
        }
    }
}


fn collect_val(b: &AstBlock, 处: 位置, 段: &[Token], exprs: &[Expr], cx: &mut Ctx) {
    let line = 处.行;
    let 规范 = prion_parser::建val规范(段);
    let 段 = &规范[..];
    let name = match 段.get(2) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
        _ => return,
    };
    
    let 新键 = (cx.names.intern(&b.name), cx.names.intern(&name));
    let 键 = match cx.local_idx.get(&新键) {
        Some(&i) => cx.locals[i].键,
        None => 新键,
    };
    cx.val_decl_line.entry(键).or_insert(line);
    
    if 段.iter().any(|t| matches!(t, Token::Sym("="))) {
        cx.val_has_value.insert(键);
    }
    if let Some(ty) = container_ty_at(段, 3) {

        let head = ty.split('(').next().unwrap_or("").trim().to_string();
        let mut bad: Option<String> = None;
        
        if matches!(head.as_ref(), "list" | "que" | "stack") {
            if let Some(e) = elem_ty_str(&ty) {
                if is_container_ty(&e) {
                    bad = Some(format!("元素是 {}", e));
                }
            }
        }
        
        if head == "map" {
            if let Some(inner) = ty.strip_prefix("map(").and_then(|x| x.strip_suffix(')')) {
                let k = inner.split(',').next().unwrap_or("").trim();

                if is_container_ty(k) {
                    bad = Some(format!("键是容器 {}", k));
                } else if k.ends_with('*') {
                    bad = Some(format!("键是把手 {}", k));
                }
            }
        }
        if let Some(why) = bad {
            cx.err(
                处,
                CheckCode::E203,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "建 val",
                    format!("容器里不能套容器——{}", why),
                    "",
                    "改成一维扁平 + 手工索引；要装容器就放进 map 的值或结构体字段（有名字的地方可以装）",
                ),
                Tone::Fix,
            );
        }
    }
    let ty = match container_ty_at(段, 3) {
        Some(t) => t,
        None => match 段.get(3) {
            Some(Token::Kw(t)) => {

                if matches!(段.get(4), Some(Token::Sym("*"))) {
                    cx.err(
                        处,
                        CheckCode::E203,
                        &DiagParts::full(
                            format!("第 {} 行", line),
                            "建 val",
                            format!("「{}」是内置类型，加不了星", t),
                            "",
                            "内置类型本来就是值；要引用语义请用自定义类型（如 node*）",
                        ),
                        Tone::Fix,
                    );
                    return;
                }
                (*t).to_string()
            }
            
            Some(Token::Ident(n)) => {
                if matches!(段.get(4), Some(Token::Sym("*"))) {
                    format!("{}*", n)
                } else {
                    n.to_string()
                }
            }
            
            _ => match 段.iter().position(|t| *t == Token::Sym("=")) {
                Some(_) => exprs
                    .first()
                    .and_then(|e| infer_ast::推断(e, cx, &b.name))
                    .unwrap_or_else(|| UNKNOWN.to_string()),
                None => return,
            },
        },
    };
    if cx.has_local(&b.name, &name) {
        
        let 同fn = cx.find_local(&b.name, &name).is_some();
        let (标题, 正文, 改法) = if 同fn {
            (
                format!("块「{}」里已经有 val「{}」", b.name, name),
                "",
                "换个名字（同一个块里 val 不许重名）".to_string(),
            )
        } else {
            (
                format!("fn 局部「{}」不许与块级 val 重名", name),
                "fn 的局部和形参有自己的名字空间，但不得占用块级 val 的名字",
                "给 fn 局部换个名字，与块级 val 错开".to_string(),
            )
        };
        cx.err(
            处,
            CheckCode::E201,
            &DiagParts::full(format!("第 {} 行", line), "建 val", 标题, 正文, 改法),
            Tone::Fix,
        );
        return;
    }
    
    if let Some(e) = exprs.first() {
        infer_ast::记(e.id(), &ty);
    }
    cx.add_local(LocalInfo { block: b.name.clone(), name: name.to_string(), ty, 键: 空键,
                                    class: StorageClass::Tmp });
}


type 局部索引 = std::collections::HashMap<(prion_types::Sym, prion_types::Sym), usize>;


fn 查局部<'a>(
    locals: &'a [LocalInfo],
    idx: &局部索引,
    names: &prion_types::Names,
    block: &str,
    name: &str,
) -> Option<&'a LocalInfo> {
    let b = names.lookup(block)?;
    let n = names.lookup(name)?;
    idx.get(&(b, n)).map(|&i| &locals[i])
}


fn iter_elem_ty(head: &[Token], locals: &[LocalInfo], idx: &局部索引, names: &prion_types::Names, block: &str) -> String {
    let mut src: &[Token] = if head.len() >= 4 { &head[3..head.len() - 1] } else { return "int".into() };
    
    if matches!(src.first(), Some(Token::Kw("call")))
        && matches!(src.get(1), Some(Token::Kw("val")))
    {
        src = &src[1..];
    }
    if src.len() >= 2
        && matches!(src[0], Token::Ident(ref n) if n == "keys")
        && matches!(src[1], Token::Sym("("))
    {
        return "str".into();
    }
    
    if let [Token::Kw("val"), rest @ (Token::Str(_) | Token::Ident(_))] = src {
        let n = match rest {
            Token::Str(n) | Token::Ident(n) => n,
            _ => unreachable!(),
        };
        if let Some(l) = 查局部(locals, idx, names, block, n) {
            if let Some(e) = elem_ty_str(&l.ty) {
                return e;
            }
        }
    }
    if let [Token::Ident(n)] = src {
        if let Some(l) = 查局部(locals, idx, names, block, n) {
            if let Some(e) = elem_ty_str(&l.ty) {
                return e;
            }
        }
    }
    "int".into()
}


fn elem_ty_str(ty: &str) -> Option<String> {
    let (head, inner) = ty.split_once('(')?;
    let inner = inner.strip_suffix(')')?;
    match head {
        "list" | "que" | "stack" => Some(inner.trim().to_string()),
        "map" => {
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
        _ => None,
    }
}


fn is_container_ty(t: &str) -> bool {
    matches!(t.split('(').next().unwrap_or("").trim(), "list" | "que" | "stack" | "map")
}


fn 是非复制类型(t: &str) -> bool {
    let t = t.trim();
    if t.is_empty() {
        return true;
    }
    if is_container_ty(t) || t == "str" || t == "ctx" || t.ends_with('*') {
        return true;
    }
    if t == "token" || t.starts_with("token(") {
        return true;
    }
    !matches!(
        t,
        "int" | "float" | "bool"
            | "int_8" | "int_16" | "int_32" | "int_64"
            | "uint_8" | "uint_16" | "uint_32" | "uint_64"
    )
}

fn 是裸join词(t: &Token) -> bool {
    match t {
        Token::Kw("join") => true,
        Token::Ident(n) => n.as_ref() == "join",
        _ => false,
    }
}


fn is_add_val(段: &[Token]) -> bool {
    if !matches!(段.first(), Some(Token::Kw("add"))) {
        return false;
    }
    let 规范 = prion_parser::建val规范(段);
    matches!(规范.get(1), Some(Token::Kw("val")) | Some(Token::Kw("fix")))
}


fn is_add_fix(段: &[Token]) -> bool {
    matches!(段.first(), Some(Token::Kw("add"))) && matches!(段.get(1), Some(Token::Kw("fix")))
}



fn scan_types(ast: &AstProgram) -> (TypeTable, BTreeMap<String, String>) {
    let mut out: TypeTable = BTreeMap::new();
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    for b in &ast.blocks {
        let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
        flatten_stmts(&b.stmts, false, &mut flat);
        for (处, 段, _nested) in flat {
            let _line = 处.行;
                if is_add_type(段) {
                    
                    if matches!(段.get(3), Some(Token::Sym("("))) {
                        continue;
                    }
                    
                    if let Some((name, fields)) = parse_type_def(段) {
                        out.entry(name).or_insert((处.clone(), b.file.clone(), fields));
                        continue;
                    }
                    
                    if let Some(name) = 段.get(2).and_then(|t| match t {
                        Token::Str(n) | Token::Ident(n) => Some(n.to_string()),
                        _ => None,
                    }) {
                        if 段.iter().any(|t| *t == Token::Sym("|")) {
                            out.entry(name).or_insert((处.clone(), b.file.clone(), Vec::new()));
                        } else if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {

                            if let Some(t) = 段.get(eq + 1) {
                                let target = match t {
                                    Token::Kw(k) => Some((*k).to_string()),
                                    Token::Ident(n) => Some(n.to_string()),
                                    _ => None,
                                };
                                if let Some(target) = target {
                                    out.entry(name.clone()).or_insert((处.clone(), b.file.clone(), Vec::new()));
                                    aliases.insert(name, target);
                                }
                            }
                        }
                    }
                }
                    }
    }
    (out, aliases)
}


fn 替换类型参数(s: &str, 参数: &[String], 实参: &[String]) -> String {
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


fn scan_generics(ast: &AstProgram) -> 泛型表 {
    let mut out: 泛型表 = BTreeMap::new();
    for b in &ast.blocks {
        let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
        flatten_stmts(&b.stmts, false, &mut flat);
        for (处, 段, _nested) in flat {
            let _line = 处.行;
            if !is_add_type(段) || !matches!(段.get(3), Some(Token::Sym("("))) {
                continue;
            }
            let name = match 段.get(2) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
                _ => continue,
            };
            let mut 深 = 0i32;
            let mut 尽 = None;
            for (k, t) in 段.iter().enumerate().skip(3) {
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
            let 尽 = match 尽 {
                Some(k) => k,
                None => continue,
            };
            let mut 参数: Vec<String> = Vec::new();
            for 段 in 段[4..尽].split(|t| *t == Token::Sym(",")) {
                if let [Token::Ident(n)] = 段 {
                    参数.push(n.to_string());
                }
            }
            if 参数.is_empty() {
                continue;
            }
            let 字段 = match parse_type_def(段) {
                Some((_名, f)) => f,
                None => Vec::new(),
            };
            
            let mut 变体表: Vec<(String, Vec<(String, String)>)> = Vec::new();
            if 字段.is_empty() {
                if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                    for 段 in 段[eq + 1..].split(|t| *t == Token::Sym("|")) {
                        if let Some(Token::Ident(vn)) | Some(Token::Str(vn)) = 段.first() {
                            let mut 载: Vec<(String, String)> = Vec::new();
                            if matches!(段.get(1), Some(Token::Sym("("))) {
                                let mut 深 = 0i32;
                                let mut 止 = None;
                                for (k, t) in 段.iter().enumerate().skip(1) {
                                    match t {
                                        Token::Sym("(") => 深 += 1,
                                        Token::Sym(")") => {
                                            深 -= 1;
                                            if 深 == 0 {
                                                止 = Some(k);
                                                break;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                if let Some(止) = 止 {
                                    for (i, a) in 段[2..止]
                                        .split(|t| *t == Token::Sym(","))
                                        .enumerate()
                                    {
                                        let 型: String = a
                                            .iter()
                                            .filter_map(|t| match t {
                                                Token::Ident(x) => Some(x.to_string()),
                                                Token::Kw(k) => Some((*k).to_string()),
                                                Token::Sym("*") => Some("*".to_string()),
                                                _ => None,
                                            })
                                            .collect();
                                        if !型.is_empty() {
                                            载.push((format!("t{}", i), 型));
                                        }
                                    }
                                }
                            }
                            变体表.push((vn.to_string(), 载));
                        }
                    }
                }
            }
            out.insert(name.to_string(), (处, 参数, 字段.iter().map(|(n, t, _)| (n.clone(), t.clone())).collect(), 变体表));
        }
    }
    out
}


fn scan_fix_names(ast: &AstProgram) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for b in &ast.blocks {
        let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
        flatten_stmts(&b.stmts, false, &mut flat);
        for (_l, 段, _nested) in flat {
            if is_add_fix(段) {
                if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 段.get(2) {
                    out.insert(format!("{}|{}", b.name, n));
                }
            }
        }
    }
    out
}


fn scan_generic_fns(ast: &AstProgram)
    -> BTreeMap<String, (Vec<String>, Vec<Vec<String>>, 位置, Vec<Token>, Vec<String>)> {
    let mut out: BTreeMap<String, (Vec<String>, Vec<Vec<String>>, 位置, Vec<Token>, Vec<String>)> =
        BTreeMap::new();
    for b in &ast.blocks {
        let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
        flatten_stmts(&b.stmts, false, &mut flat);
        for (_line, 段, _nested) in flat {
            let 名位 = if matches!(段.first(), Some(Token::Kw("add")))
                && matches!(段.get(1), Some(Token::Kw("fn")))
            {
                2
            } else if matches!(段.first(), Some(Token::Kw("fn"))) {
                1
            } else {
                continue;
            };
            if !matches!(段.get(名位 + 1), Some(Token::Sym("("))) {
                continue;
            }
            let mut 深 = 0i32;
            let mut 止 = None;
            for (k, t) in 段.iter().enumerate().skip(名位 + 1) {
                match t {
                    Token::Sym("(") => 深 += 1,
                    Token::Sym(")") => {
                        深 -= 1;
                        if 深 == 0 {
                            止 = Some(k);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let 止 = match 止 {
                Some(k) => k,
                None => continue,
            };
            let mut 参数: Vec<String> = Vec::new();
            let mut 能力: Vec<Vec<String>> = Vec::new();
            let mut 合规 = true;
            for 段 in 段[名位 + 2..止].split(|t| *t == Token::Sym(",")) {
                match 段 {
                    [Token::Ident(n)] => {
                        参数.push(n.to_string());
                        能力.push(Vec::new());
                    }
                    [Token::Ident(n), Token::Sym(op)] => {
                        参数.push(n.to_string());
                        能力.push(vec![(*op).to_string()]);
                    }
                    _ => 合规 = false,
                }
            }
            if 合规 && !参数.is_empty() {
                if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 段.get(名位) {
                    let 体 = match 段.iter().position(|t| *t == Token::Sym("=")) {
                        Some(eq) => 段[eq + 1..].to_vec(),
                        None => Vec::new(),
                    };
                    
                    let mut 形参名: Vec<String> = Vec::new();
                    if let Some(开2) = 配对括号(段, 名位 + 1).map(|k| k + 1) {
                        if let Some(止2) = 配对括号(段, 开2) {
                            for 段 in 段[开2 + 1..止2].split(|t| *t == Token::Sym(",")) {
                                if let Some(Token::Ident(pn)) | Some(Token::Str(pn)) = 段.first() {
                                    形参名.push(pn.to_string());
                                }
                            }
                        }
                    }
                    out.insert(n.to_string(), (参数, 能力, _line, 体, 形参名));
                }
            }
        }
    }
    for b in &ast.blocks {
        walk_generic_stmt_fns(&b.stmts, &mut out);
    }
    out
}


fn walk_generic_stmt_fns(
    stmts: &[AstStmt],
    out: &mut BTreeMap<String, (Vec<String>, Vec<Vec<String>>, 位置, Vec<Token>, Vec<String>)>,
) {
    for s in stmts {
        match s {
            AstStmt::Fn { line, col, len, head, body, .. } => {
                let h: Vec<Token> =
                    head.iter().cloned().filter(|t| *t != Token::Sym(":")).collect();
                if let Some((参数能力, 形参)) = 泛型语句头(&h) {
                    if let Some(n) = stmt_fn_name(head) {
                        let mut flat = Vec::new();
                        flatten_stmts(body, true, &mut flat);
                        let mut 体: Vec<Token> = Vec::new();
                        for (_, t, _) in flat {
                            体.extend(t.iter().cloned());
                        }
                        let 参数 = 参数能力.iter().map(|(n, _)| n.clone()).collect();
                        let 能力 = 参数能力.iter().map(|(_, c)| c.clone()).collect();
                        let 形参名 = 形参.iter().map(|(n, _)| n.clone()).collect();
                        out.insert(n, (参数, 能力, 位置{ 行: *line, 列: *col, 长: *len }, 体, 形参名));
                    }
                }
                walk_generic_stmt_fns(body, out);
            }
            AstStmt::Struct { body, .. } => walk_generic_stmt_fns(body, out),
            _ => {}
        }
    }
}


fn scan_variants(ast: &AstProgram) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for b in &ast.blocks {
        let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
        flatten_stmts(&b.stmts, false, &mut flat);
        for (_line, 段, _nested) in flat {
                if !is_add_type(段) {
                    continue;
                }
                if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                    
                    if matches!(段.get(eq + 1), Some(Token::Sym("("))) {
                        continue;
                    }
                    let mut i = eq + 1;
                    while i + 1 < 段.len() {
                        if let Token::Ident(n) = &段[i] {
                            if matches!(段.get(i + 1), Some(Token::Sym("("))) {
                                out.insert(n.to_string());
                            }
                        }
                        i += 1;
                    }
                }
                    }
    }
    out
}


fn scan_variant_owner(ast: &AstProgram) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for b in &ast.blocks {
        let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
        flatten_stmts(&b.stmts, false, &mut flat);
        for (_line, 段, _nested) in flat {
            if !is_add_type(段) {
                continue;
            }
            let tname = match 段.get(2) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
                _ => continue,
            };
            let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) else { continue };
            
            if matches!(段.get(eq + 1), Some(Token::Sym("("))) {
                continue;
            }
            let mut i = eq + 1;
            while i + 1 < 段.len() {
                if let Token::Ident(n) | Token::Str(n) = &段[i] {
                    if matches!(段.get(i + 1), Some(Token::Sym("("))) {
                        out.insert(n.to_string(), tname.to_string());
                    }
                }
                i += 1;
            }
        }
    }
    out
}


fn collect_case_bindings(b: &AstBlock, head: &[Token], cx: &mut Ctx) {
    if !matches!(head.first(), Some(Token::Kw("case")))
        || !matches!(head.get(2), Some(Token::Sym("(")))
    {
        return;
    }
    let mut d = 0i32;
    let mut end = None;
    for (k, tk) in head.iter().enumerate().skip(2) {
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
        for a in split_commas(&head[3..end]) {
            if let Some(Token::Ident(n)) = a.first() {
                cx.err(
                    位置{ 行: b.line, 列: b.col, 长: b.len },
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", b.line),
                        "case 绑定",
                        format!("解构绑定「{}」没加引号——自定义名一律带引号（块名除外）", n),
                        "",
                        format!("写成 `case 变体(\"{}\")：`；体内读取写 val \"{}\"", n, n),
                    ),
                    Tone::Fix,
                );
            }
            if let Some(Token::Str(n)) = a.first() {
                if !cx.has_local(&b.name, n) {
                    cx.add_local(LocalInfo {
                        block: b.name.clone(),
                        name: n.to_string(),
                        ty: UNKNOWN.to_string(),
                        键: 空键,
                                    class: StorageClass::Tmp,
                    });
                    
                }
            }
        }
    }
}

fn is_add_type(段: &[Token]) -> bool {
    matches!(段.first(), Some(Token::Kw("add"))) && matches!(段.get(1), Some(Token::Kw("type")))
}

fn 找val声明类型(ast: &AstProgram, name: &str) -> Option<String> {
    for b in &ast.blocks {
        for st in &b.stmts {
            let toks = match st { AstStmt::Line { kind, .. } => kind.tokens(), _ => continue };
            if !matches!(toks.first(), Some(Token::Kw("add"))) { continue; }
            if !matches!(toks.get(1), Some(Token::Kw("val")) | Some(Token::Kw("fix"))) { continue; }
            match toks.get(2) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) if n.as_ref() == name => {},
                _ => continue,
            }
            if matches!(toks.get(3), Some(Token::HashHashColon)) {
                let ti = toks.iter().position(|t| matches!(t, Token::Kw(k) if *k == "type"))?;
                if let Some(Token::Str(n) | Token::Ident(n)) = toks.get(ti + 1) { return Some(n.to_string()); }
            }
            if let Some(Token::Kw(kw)) = toks.get(3) {
                return Some((*kw).to_string());
            }
            if let Some(Token::Str(n) | Token::Ident(n)) = toks.get(3) {
                return Some(n.to_string());
            }
        }
    }
    None
}

fn check_hidden_field_access(cx: &mut Ctx, ast: &AstProgram, 处: &位置, b: &AstBlock, 段: &TokStream) {
    for k in 0..段.len().saturating_sub(2) {
        if !matches!(段.get(k), Some(Token::Sym("."))) { continue; }
        let fname = match 段.get(k + 1) {
            Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
            _ => continue,
        };
        let mut valname = None;
        for j in (0..k).rev() {
            if matches!(段.get(j), Some(Token::Kw("val"))) {
                if let Some(Token::Ident(n) | Token::Str(n)) = 段.get(j + 1) {
                    valname = Some(n.to_string());
                    break;
                }
            }
        }
        let Some(vn) = valname else { continue };
        let ty = cx.find_local(&b.name, &vn).map(|l| l.ty.clone())
            .or_else(|| 找val声明类型(ast, &vn)).unwrap_or_default();
        let plain = ty.trim_end_matches('*');
        let Some((_, def_file, fs)) = cx.types.get(plain) else { continue };
        if def_file == &cx.cur_file { continue; }
        if !fs.iter().any(|(n, _, h)| n == &fname && *h) { continue; }
        cx.err(
            处.clone(),
            CheckCode::E201,
            &DiagParts::full(
                format!("第 {} 行", 处.行),
                "字段隐藏",
                format!("字段「{}」在类型「{}」上被 hide 了", fname, plain),
                "",
                "跨模块只能读公开字段；要读请在本模块内提供取值函数",
            )
            .预期("hide 字段不在跨模块的可见面上"),
            Tone::Fix,
        );
    }
}

fn parse_type_def(段: &[Token]) -> Option<(String, Vec<(String, String, bool)>)> {
    let name = match 段.get(2) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
        _ => return None,
    };
    
    let eq = 段.iter().position(|t| *t == Token::Sym("="))?;
    if !matches!(段.get(eq + 1), Some(Token::Sym("("))) {
        return None;
    }
    let open = eq + 1;
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
    let end = end?;
    let mut fields = Vec::new();
    for a in split_commas(&段[open + 1..end]) {
        let hidden = matches!(a.first(), Some(Token::Kw("hide")));
        let rest: &[Token] = if hidden { &a[1..] } else { &a };
        let fname = match rest.first() {
            Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
            _ => return None,
        };
        
        if rest.len() < 2 {
            return None;
        }
        let mut fty = String::new();
        let mut depth = 0i32;
        for tk in &rest[1..] {
            match tk {
                Token::Sym("(") => {
                    depth += 1;
                    fty.push('(');
                }
                Token::Sym(")") => {
                    depth -= 1;
                    fty.push(')');
                }
                Token::Sym(",") if depth > 0 => fty.push(','),
                Token::Sym("*") => fty.push('*'),
                Token::Kw(t) => fty.push_str(t),
                Token::Ident(n) => fty.push_str(n),
                Token::Str(n) => fty.push_str(n),
                _ => {}
            }
        }
        fields.push((fname, fty, hidden));
    }
    Some((name.to_string(), fields))
}


fn check_type_cycles(types: &TypeTable, cx: &mut Ctx) {
    for (name, (tline, _def_file, fields)) in types {
        
        for (fname, fty, _h) in fields {
            if !fty.ends_with('*') && fty == name.as_str() {
                cx.err(
                    *tline,
                    CheckCode::E203,
                    &DiagParts::full(
                        "类型定义",
                        "写字段",
                        format!("裸环：字段「{}」的类型回到自己", fname),
                        "",
                        format!("写 {}*（把手）就不成环", name),
                    ),
                    Tone::Fix,
                );
            }
        }
        
        if reaches_self(types, name) {
            cx.err(
                *tline,
                CheckCode::E203,
                &DiagParts::full(
                    "类型定义",
                    "写字段",
                    format!("互指成环：{} 沿值字段又能回到自己", name),
                    "",
                    "把回环的那一端改成把手（加 *）",
                ),
                Tone::Fix,
            );
        }
    }
}


fn reaches_self(types: &TypeTable, start: &str) -> bool {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = Vec::new();
    if let Some((_, _df, fs)) = types.get(start) {
        for (_, t, _h) in fs {
            if !t.ends_with('*') {
                stack.push(t.clone());
            }
        }
    }
    while let Some(n) = stack.pop() {
        if n == start {
            return true;
        }
        if !seen.insert(n.clone()) {
            continue;
        }
        if let Some((_, _df, fs)) = types.get::<str>(n.as_ref()) {
            for (_, t, _h) in fs {
                if !t.ends_with('*') {
                    stack.push(t.clone());
                }
            }
        }
    }
    false
}



fn collect_fn(b: &AstBlock, 处: 位置, 段: &[Token], cx: &mut Ctx) {
    let _line = 处.行;
    
    let name_at = if matches!(段.get(1), Some(Token::Kw("hide"))) { 3 } else { 2 };
    register_fn(b, 处, 段, name_at, false, false, cx);
}


fn collect_rebind_fn(b: &AstBlock, 处: 位置, 段: &[Token], cx: &mut Ctx) {
    let _line = 处.行;
    register_fn(b, 处, 段, 1, false, true, cx);
}


fn collect_val_rebind(b: &AstBlock, 处: 位置, 段: &[Token], cx: &mut Ctx) {
    let line = 处.行;
    let name = match 段.get(1) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
        _ => return,
    };
    let key = (cx.names.intern(&b.name), cx.names.intern(&name));
    let eq = match 段.iter().position(|t| *t == Token::Sym("=")) {
        Some(e) => e,
        None => return,
    };
    let rhs = &段[eq + 1..];
    let is_delta = match (rhs.first(), rhs.get(1)) {
        (Some(Token::Kw("val")), Some(Token::Str(n) | Token::Ident(n))) => {
            n == &name && matches!(rhs.get(2), Some(Token::Sym("(")))
        }
        _ => false,
    };
    if is_delta && !cx.val_has_value.contains(&key) {
        cx.err(
            处,
            CheckCode::E201,
            &DiagParts::full(
                format!("第 {} 行", line),
                "改值",
                format!("取不到「{}」的当前值——先给它一个值", name),
                "",
                "delta 是「取当前值再改」：先 `add val \"名\" 类型 = 初值`，或先用 `val \"名\" = 值` 给过值",
            ),
            Tone::Fix,
        );
        return;
    }
    cx.val_has_value.insert(key);
}


fn check_hide_decls(ast: &AstProgram, cx: &mut Ctx) {
    for b in &ast.blocks {
        let is_pub = b.attrs.iter().any(|a| a == "pub");
        let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
        flatten_stmts(&b.stmts, false, &mut flat);
        for (处, 段, _nested) in flat {
            let line = 处.行;
            if !matches!(段.first(), Some(Token::Kw("add")))
                || !matches!(段.get(1), Some(Token::Kw("hide")))
            {
                continue;
            }
            let cat = match 段.get(2) {
                Some(Token::Kw("fn")) => "fn",
                Some(Token::Kw("type")) => "type",
                Some(Token::Kw("fix")) => "fix",
                _ => {
                    cx.err(
                        处,
                        CheckCode::E203,
                        &DiagParts::full(
                            format!("第 {} 行", line),
                            "hide 可见性",
                            "hide 只能对 fn / type / fix 写".to_string(),
                            "",
                            "val 本就在公开面里，不许单独藏",
                        ),
                        Tone::Fix,
                    );
                    continue;
                }
            };
            if !is_pub {
                cx.err(
                    处,
                    CheckCode::E203,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "hide 可见性",
                        format!("块「{}」不是 pub 块，不许 hide", b.name),
                        "",
                        "hide 是给 pub 块开窗又拉帘用的；普通块本来就不对外",
                    ),
                    Tone::Fix,
                );
                continue;
            }
            if !matches!(段.get(3), Some(Token::Str(_)) | Some(Token::Ident(_))) {
                cx.err(
                    处,
                    CheckCode::E203,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "hide 可见性",
                        format!(
                            "`add hide {}` 后应接{}名",
                            cat,
                            if cat == "fn" { "fn" } else { "类型" }
                        ),
                        "",
                        "写成 `add hide fn \"名\"` / `add hide type \"名\"`",
                    ),
                    Tone::Fix,
                );
            }
        }
    }
}


fn collect_outs_fn(b: &AstBlock, 处: 位置, 段: &[Token], cx: &mut Ctx) {
    let _line = 处.行;
    register_fn(b, 处, 段, 3, true, false, cx);
}



fn register_fn(
    b: &AstBlock,
    处: 位置,
    段: &[Token],
    name_at: usize,
    is_outs: bool,
    rebind: bool,
    cx: &mut Ctx,
) {
    let line = 处.行;
    let name = match 段.get(name_at) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
        _ => return,
    };

    if BUILTINS.contains(&name.as_ref()) {
        cx.err(
            处,
            CheckCode::E201,
            &DiagParts::full(
                format!("第 {} 行", line),
                "建 fn",
                format!("「{}」是内置函数名，不能拿来当 fn 名", name),
                "",
                "换个名字（内置名开箱即用，不允许被覆盖）",
            ),
            Tone::Suggest,
        );
        return;
    }
    
    let 定义键 = (cx.names.intern(&b.name), cx.names.intern(&name));
    cx.fn_decl_line.entry(定义键).or_insert(line);
    if cx.fns.contains_key(&定义键) && !rebind {
        cx.err(
            处,
            CheckCode::E201,
            &DiagParts::full(
                format!("第 {} 行", line),
                "建 fn",
                format!("fn「{}」已经建过", name),
                "",
                "换个名字（同一个块里 fn 不许重名；补体/重绑定写 `fn \"名\" (参) 返 = 体`）",
            ),
            Tone::Fix,
        );
        return;
    }
    let (params, after) = match parse_params(段) {
        Ok(v) => v,
        Err(_) => return,
    };

    let ret = match parse_type_ref(段, after) {
        Ok((t, _)) => t,
        Err(_) => return,
    };
    
    if is_outs {
        for (pname, pty) in &params {
            if !matches!(pty.as_ref(), "int" | "float" | "bool") {
                cx.err(
                    处,
                    CheckCode::E202,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "声明外部函数",
                        format!("参数「{}」是 {}——外部函数只收标量", pname, pty),
                        "",
                        "字符串进出写 to_c_str / from_c_str 显式转换",
                    )
                    .预期("外部函数的参数只能是 int / float / bool 标量")
                .详解("字符串进出 C 必须显式转换：进去写 to_c_str(串)，出来写 from_c_str(指针)——这是安全边界，不能让指针悄悄穿过。"),
                    Tone::Fix,
                );
                return;
            }
        }
        if !matches!(ret.as_ref(), "int" | "float" | "bool") {
            cx.err(
                处,
                CheckCode::E202,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "声明外部函数",
                    format!("返回类型 {}——外部函数只收标量", ret),
                    "",
                    "字符串进出写 to_c_str / from_c_str 显式转换",
                )
                .预期("外部函数的返回只能是 int / float / bool 标量")
                .详解("字符串进出 C 必须显式转换：进去写 to_c_str(串)，出来写 from_c_str(指针)。"),
                Tone::Fix,
            );
            return;
        }
    }
    let key = (cx.names.intern(&b.name), cx.names.intern(&name));
    cx.fns.insert(
        key,
        FnSig {
            params: params.iter().map(|(_, t)| t.clone()).collect(),
            ret,
            is_outs,
        },
    );
    
    if 段.iter().any(|t| matches!(t, Token::Sym("="))) {
        cx.fn_has_body.insert(key.clone());
    }
    
    if !b.module.is_empty() {
        if let Some(sig) = cx.fns.get(&key) {
            cx.fns_mod
                .insert(format!("{}|{}|{}", b.module, b.name, name), sig.clone());
        }
    }
    
    
    if !is_outs {
        for (pname, pty) in &params {
            let 局部块 = format!("{}#{}", b.name, name);
            let 块键 = (cx.names.intern(&b.name), cx.names.intern(pname));
            if cx.local_idx.contains_key(&块键) {
                cx.err(
                    处,
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", 处.行),
                        "fn 形参",
                        format!("形参「{}」与块级 val 重名", pname),
                        "fn 局部与形参不许和块级 val 同名——名字空间分开，歧义在编译期消灭",
                        "给形参换个名字",
                    ),
                    Tone::Fix,
                );
            }
            cx.add_local(LocalInfo {
                block: 局部块,
                name: pname.clone(),
                ty: pty.clone(),
                键: 空键,
                                    class: StorageClass::Tmp,
            });
        }
    }
}


fn parse_type_ref(段: &[Token], at: usize) -> Result<(String, usize), String> {
    match 段.get(at) {
        Some(Token::Kw(t)) => {
            let 基 = (*t).to_string();
            if matches!(基.as_ref(), "list" | "que" | "stack" | "map")
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
                let 尽 = 尽.ok_or_else(|| format!("容器类型「{}」的括号没闭合", 基))?;
                let mut 实参: Vec<String> = Vec::new();
                for 段 in split_commas(&段[at + 2..尽]) {
                    let (ty, _) = parse_type_ref(&段, 0)?;
                    实参.push(ty);
                }
                Ok((format!("{}({})", 基, 实参.join(", ")), 尽 + 1))
            } else if matches!(段.get(at + 1), Some(Token::Sym("(")))
                && matches!(段.get(at + 2), Some(Token::Sym(">")) | Some(Token::Sym("<")) | Some(Token::Sym(">=")) | Some(Token::Sym("<="))) {
                    let mut 深 = 0i32;
                let mut 尽 = None;
                for (k, tk) in 段.iter().enumerate().skip(at + 1) {
                    match tk {
                        Token::Sym("(") => 深 += 1,
                        Token::Sym(")") => {
                            深 -= 1;
                            if 深 == 0 { 尽 = Some(k); break; }
                        }
                        _ => {}
                    }
                }
                if let Some(尽) = 尽 {
                    let inner: Vec<String> = 段[at + 2..尽].iter().map(|t| match t {
                        Token::Sym(s) => s.to_string(),
                        Token::Int(n) => n.to_string(),
                        Token::Kw(k) => k.to_string(),
                        Token::Ident(n) => n.to_string(),
                        _ => String::new(),
                    }).filter(|s| !s.is_empty()).collect();
                    Ok((format!("{}({})", 基, inner.join("")), 尽 + 1))
                } else {
                    Ok((基, at + 1))
                }
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
                let 尽 = 尽.ok_or_else(|| "函数类型「fn」的括号没闭合".to_string())?;
                let mut 实参: Vec<String> = Vec::new();
                for 段 in split_commas(&段[at + 2..尽]) {
                    let (ty, _) = parse_type_ref(&段, 0)?;
                    实参.push(ty);
                }
                let (ret, 止) = parse_type_ref(段, 尽 + 1)?;
                Ok((format!("fn({}) {}", 实参.join(", "), ret), 止))
            } else {
                Ok((基, at + 1))
            }
        }
        Some(Token::Ident(n)) | Some(Token::Str(n)) => {
            if matches!(段.get(at + 1), Some(Token::Sym("*"))) {
                Ok((format!("{}*", n), at + 2))
            } else {
                Ok((n.to_string(), at + 1))
            }
        }
        Some(Token::HashColon) | Some(Token::HashHashColon) => {
            let cross_mod = matches!(段.get(at), Some(Token::HashHashColon));
            let colon_at = if cross_mod {
                let mut j = at + 1;
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
                    return Err("`##:` 后应接模块名再接 `#:块`——形如 `##:前端.拆词 #:分句`".into());
                }
                j
            } else {
                at
            };
            if !matches!(段.get(colon_at + 2), Some(Token::Kw("type"))) {
                return Err("跨块取类型要写 `#:块 type \"名\"`".into());
            }
            let tname = match 段.get(colon_at + 3) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
                _ => return Err("`#:块 type` 后应接类型名".into()),
            };
            if matches!(段.get(colon_at + 4), Some(Token::Sym("*"))) {
                Ok((format!("{}*", tname), colon_at + 5))
            } else {
                Ok((tname.to_string(), colon_at + 4))
            }
        }
        _ => Err("这里应当是类型（内置类型名 / 自定义类型名 / `#:块 type \"名\"`）".into()),
    }
}

fn parse_params(段: &[Token]) -> Result<(Vec<(String, String)>, usize), String> {
    let open = 段
        .iter()
        .position(|t| *t == Token::Sym("("))
        .ok_or("缺参数表")?;
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
    let end = end.ok_or("参数表没闭合")?;
    let mut out = Vec::new();
    for a in split_commas(&段[open + 1..end]) {
        let name = match a.first() {
            Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
            _ => return Err("参数缺名字".into()),
        };
        let (ty, _) = match a.get(1) {
            Some(_) => parse_type_ref(&a, 1)?,
            None => return Err(format!("参数「{}」缺类型", name)),
        };
        out.push((name, ty));
    }
    Ok((out, end + 1))
}

fn split_commas(inner: &[Token]) -> Vec<Vec<Token>> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    let mut depth = 0i32;
    for t in inner {
        match t {
            Token::Sym("(") => depth += 1,
            Token::Sym(")") => depth -= 1,
            Token::Sym("[") => depth += 1,
            Token::Sym("]") => depth -= 1,
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


fn call_args(段: &[Token], open: usize) -> Vec<Vec<Token>> {
    let mut depth = 0i32;
    let mut inner: Vec<Token> = Vec::new();
    let mut i = open;
    while i < 段.len() {
        match &段[i] {
            Token::Sym("(") => depth += 1,
            Token::Sym(")") => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        if depth >= 1 {
            inner.push(段[i].clone());
        }
        i += 1;
    }
    if matches!(inner.first(), Some(Token::Sym("("))) {
        inner.remove(0);
    }
    split_commas(&inner)
}


fn 配对括号(段: &[Token], 开: usize) -> Option<usize> {
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



fn check_caps(cx: &mut Ctx) {
    let 清单: Vec<(String, Vec<String>, Vec<Vec<String>>, 位置, Vec<Token>, Vec<String>)> = cx
        .泛型fn能力
        .iter()
        .map(|(k, v)| (k.clone(), v.0.clone(), v.1.clone(), v.2, v.3.clone(), v.4.clone()))
        .collect();
    for (名, 参数, 能力, 处, 体, 形参名) in 清单 {
        let 行 = 处.行;
        let 声明: Vec<String> = 能力.iter().flatten().cloned().collect();
        
        let 泛型型参数: Vec<String> = 形参名
            .iter()
            .filter(|_| true)
            .cloned()
            .collect();
        let mut 可用 = 声明.clone();
        if 声明.iter().any(|x| x == "<") {
            可用.extend([">", "<=", ">="].iter().map(|x| x.to_string()));
        }
        if 声明.iter().any(|x| x == "==") {
            可用.push("!=".to_string());
        }
        for (k, t) in 体.iter().enumerate() {
            let op = match t {
                Token::Sym(o) => o.to_string(),
                _ => continue,
            };
            if !["<", "<=", ">", ">=", "==", "!=", "+", "-", "*", "/", "%"]
                .contains(&op.as_ref())
            {
                continue;
            }
            
            let 邻域: Vec<&Token> = 体.iter().skip(k.saturating_sub(3)).take(6).collect();
            let 作用在参数 = 邻域.iter().any(|t| match t {
                
                Token::Ident(n) => 参数.iter().any(|p| p == n.as_ref()) || 形参名.iter().any(|p| p == n.as_ref()),
                _ => false,
            }) || 体
                .iter()
                .skip(k.saturating_sub(4))
                .take(8)
                .any(|t| match t {
                    Token::Str(n) => 泛型型参数.iter().any(|p| p == n.as_ref()),
                    _ => false,
                });
            if 作用在参数 && !可用.contains(&op) {
                cx.err(
                    处,
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", 行),
                        "能力约束",
                        format!(
                            "泛型 fn「{}」的体里对类型参数用了「{}」，参数表没声明这个能力",
                            名, op
                        ),
                        "能力集 = 运算符集，声明了才能用",
                        format!(
                            "在类型参数上标注（当前已声明：{}）",
                            if 可用.is_empty() {
                                "无".to_string()
                            } else {
                                可用.join(" ")
                            }
                        ),
                    ),
                    Tone::Fix,
                );
            }
        }
    }
}


fn check_call(b: &AstBlock, 处: 位置, name: &str, args: &[Vec<Token>], cx: &mut Ctx) {
    let line = 处.行;
    
    
    let 本键 = (cx.names.intern(&b.name), cx.names.intern(name));
    let mut sig_key = 本键;
    if !cx.fns.contains_key(&本键) {
        let alt = (cx.names.intern(原块(&b.name)), 本键.1);
        if cx.fns.contains_key(&alt) {
            sig_key = alt;
        }
    }
    let sig = match cx.fns.get(&sig_key) {
        Some(s) => s.clone(),
        None => {
            
            if cx.泛型fn能力.contains_key(name) {
                return;
            }
            cx.err(
                处,
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "调函数",
                    format!("fn「{}」还没建过", name),
                    "",
                    "先用 add fn \"名\" (参数) 返回类型 = 表达式 建",
                ),
                Tone::Fix,
            );
            
            cx.fns.insert(
                sig_key,
                FnSig { params: Vec::new(), ret: UNKNOWN.to_string(), is_outs: false },
            );
            return;
        }
    };

    
    if let Some(&decl) = cx.fn_decl_line.get(&sig_key) {
        if decl > line {
            cx.err(
                处,
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "调函数",
                    format!("fn「{}」用在它被建出来之前——它到第 {} 行才建", name, decl),
                    "",
                    format!("把建「{}」的那行 `add fn` 移到这一行之前", name),
                )
                .预期("函数要先建出来，再调它")
                .详解(
                    "Prion 是「先建后用」：调用要排在 `add fn` 之后。`add type`（类型）不受这条约束，可以先用后写。",
                ),
                Tone::Fix,
            );
        }
    }
    
    if !sig.is_outs && !cx.fn_has_body.contains(&sig_key) {
        cx.err(
            处,
            CheckCode::E201,
            &DiagParts::full(
                format!("第 {} 行", line),
                "调函数",
                format!("fn「{}」只有声明，还没有体", name),
                "",
                format!("用 `fn \"{}\" (参数) 返回类型 = 表达式` 给它补体", name),
            )
            .预期("声明过的 fn 要补体才能调用"),
            Tone::Fix,
        );
        return;
    }
    check_sig_args(b, 处, name, &sig, args, cx);
}


fn check_sig_args(
    b: &AstBlock,
    处: 位置,
    name: &str,
    sig: &FnSig,
    args: &[Vec<Token>],
    cx: &mut Ctx,
) {
    let line = 处.行;
    if sig.params.len() != args.len() {
        cx.err(
            处,
            CheckCode::E202,
            &DiagParts::full(
                format!("第 {} 行", line),
                "调函数",
                format!("fn「{}」要 {} 个参数，给了 {} 个", name, sig.params.len(), args.len()),
                "",
                "按定义给够参数",
            )
            .预期(format!("按定义给够 {} 个参数", sig.params.len()))
                .详解("参数个数由 add fn 的声明说了算；给多了给少了都拦——调用和定义必须对得上。"),
            Tone::Fix,
        );
        return;
    }
    for (k, a) in args.iter().enumerate() {
        
        let got = infer_named(a, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types)
            .or_else(|| infer(a, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types).map(|s| s.to_string()));
        if let Some(got) = got {
            let want = &sig.params[k];
            
            if got != UNKNOWN && !类型相容(want, &got) {
                cx.err(
                    处,
                    CheckCode::E202,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "调函数",
                        format!("fn「{}」第 {} 个参数要 {}，给了 {}", name, k + 1, want, got),
                        "",
                        format!("改成 {} 的值", want),
                    )
                    .预期(format!("第 {} 个参数应当是 {} 型", k + 1, want))
                .详解("实参类型要和形参声明一致；类型不合时用 int(值) / str(值) / float(值) / bool(值) 显式转换再传。"),
                    Tone::Fix,
                );
            }
        }
    }
}



fn 类型相容(want: &str, got: &str) -> bool {
    if want == got {
        return true;
    }
    let 基 = |s: &str| s.split('(').next().unwrap_or(s).trim().to_string();
    let wb = 基(want);
    let gb = 基(got);
    wb == gb
        && matches!(wb.as_ref(), "list" | "que" | "stack" | "map")
        && !(want.contains('(') && got.contains('('))
}




fn pass2_match_exhaustive(ast: &AstProgram, cx: &mut Ctx) {
    
    let mut ty_variants: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut var_owner: BTreeMap<String, String> = BTreeMap::new();
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
    let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
    flatten_stmts(&b.stmts, false, &mut flat);
        for (_line, 段, _nested) in flat {
                if !is_add_type(段) {
                    continue;
                }
                if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                    let tname = match 段.get(2) {
                        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
                        _ => continue,
                    };
                    let mut vs = Vec::new();
                    let mut i = eq + 1;
                    while i + 1 < 段.len() {
                        if let Token::Ident(n) = &段[i] {
                            if matches!(段.get(i + 1), Some(Token::Sym("("))) {
                                vs.push(n.to_string());
                            }
                        }
                        i += 1;
                    }
                    if !vs.is_empty() {
                        for v in &vs {
                            var_owner.insert(v.to_string(), tname.to_string());
                        }
                        ty_variants.insert(tname.to_string(), vs);
                    }
                }
                    }
    }
    
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        check_match_exhaustive_block(b, &ty_variants, &var_owner, cx);
        check_expr_matches_block(b, &ty_variants, &var_owner, cx);
    }
}



fn collect_match_structs<'a>(stmts: &'a [AstStmt], out: &mut Vec<&'a AstStmt>) {
    for s in stmts {
        if let AstStmt::Struct { head, body, .. } = s {
            if matches!(head.first(), Some(Token::Kw("match"))) {
                out.push(s);
            }
            collect_match_structs(body, out);
        }
    }
}




fn collect_bare_ok_names(stmts: &[AstStmt], out: &mut BTreeSet<String>) {
    for s in stmts {
        if let AstStmt::Struct { head, body, .. } = s {
            match head.first() {
                Some(Token::Kw("for")) => {
                    if let Some(Token::Ident(n)) | Some(Token::Str(n)) = head.get(1) {
                        out.insert((*n).to_string());
                    }
                }
                Some(Token::Kw("case")) => {
                    
                    if matches!(head.get(2), Some(Token::Sym("("))) {
                        for t in head.iter().skip(3) {
                            match t {
                                Token::Ident(n) | Token::Str(n) => {
                                    out.insert((*n).to_string());
                                }
                                Token::Sym(")") => break,
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
            collect_bare_ok_names(body, out);
        }
    }
}




fn match_object_ty(
    b: &AstBlock,
    obj: &[Token],
    bare_ok: &BTreeSet<String>,
    params: &BTreeMap<String, String>,
    var_owner: &BTreeMap<String, String>,
    处: 位置,
    cx: &mut Ctx,
) -> Option<String> {
    let line = 处.行;
    
    let obj: &[Token] = if matches!(obj.first(), Some(Token::Kw("call")))
        && matches!(obj.get(1), Some(Token::Kw("val")))
    {
        &obj[1..]
    } else {
        obj
    };
    
    if let [Token::Kw("val"), rest @ (Token::Str(_) | Token::Ident(_))] = obj {
        let n = match rest {
            Token::Str(n) | Token::Ident(n) => n,
            _ => unreachable!(),
        };
        let key = (cx.names.intern(&b.name), cx.names.intern(n.as_ref()));
        return match cx.local_idx.get(&key) {
            Some(&i) => Some(cx.locals[i].ty.trim_end_matches('*').to_string()),
            None => {
                cx.err(
                    处,
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "match 判别",
                        format!("val「{}」还没建过", n),
                        "",
                        "先用 add val \"名\" 和类型 = … 建，再 match",
                    ),
                    Tone::Fix,
                );
                None
            }
        };
    }
    
    if let [Token::Kw("call"), Token::Kw("fn"), rest @ (Token::Str(_) | Token::Ident(_)), Token::Sym("("), ..] = obj {
        let n = match rest {
            Token::Str(n) | Token::Ident(n) => n,
            _ => unreachable!(),
        };
        let key = (cx.names.intern(&b.name), cx.names.intern(n.as_ref()));
        if let Some(sig) = cx.fns.get(&key) {
            return Some(sig.ret.trim_end_matches('*').to_string());
        }
        return None; 
    }
    
    if let [Token::Ident(n), Token::Sym("("), ..] = obj {
        if let Some(owner) = var_owner.get(n.as_ref()) {
            return Some(owner.clone());
        }
        return None; 
    }
    
    if let Some(Token::Ident(n)) = obj.first() {
        if let Some(t) = params.get(n.as_ref()) {
            
            return Some(t.trim_end_matches('*').to_string());
        }
        if !bare_ok.contains(n.as_ref()) {
            let key = (cx.names.intern(&b.name), cx.names.intern(n.as_ref()));
            if cx.local_idx.contains_key(&key) {
                cx.err(
                    处,
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "match 判别",
                        format!("match 对象里的「{}」是裸名——取 val 请写 val \"{}\"", n, n),
                        "",
                        "match 对象必须显式：写 val \"名\"，或直接给一个表达式（如 call fn \"名\"(…)）",
                    )
                    .预期("match 对象不留裸名别名（全面显式：fn 与 val 可重名，裸名消不了歧义）"),
                    Tone::Fix,
                );
                return None;
            }
            cx.err(
                处,
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "match 判别",
                    format!("「{}」还没建过", n),
                    "",
                    "先建再用：add val \"名\" 和类型 = …，match 对象写 val \"名\"",
                ),
                Tone::Fix,
            );
            return None;
        }
        
        let key = (cx.names.intern(&b.name), cx.names.intern(n.as_ref()));
        return cx.local_idx.get(&key).map(|&i| cx.locals[i].ty.trim_end_matches('*').to_string());
    }
    
    cx.err(
        处,
        CheckCode::E202,
        &DiagParts::full(
            format!("第 {} 行", line),
            "match 判别",
            "match 对象的类型推不出来",
            "",
            "先存带类型 val 再 match（add val \"名\" 和类型 = …），或换成一个能看出类型的表达式",
        )
        .预期("match 对象的类型必须编译期可推：val 声明类型 / fn 声明返回类型 / 变体构造"),
        Tone::Fix,
    );
    None
}




fn check_match_exhaustive_block(
    b: &AstBlock,
    ty_variants: &BTreeMap<String, Vec<String>>,
    var_owner: &BTreeMap<String, String>,
    cx: &mut Ctx,
) {
    let mut found: Vec<&AstStmt> = Vec::new();
    collect_match_structs(&b.stmts, &mut found);
    if found.is_empty() {
        return;
    }
    let mut bare_ok: BTreeSet<String> = BTreeSet::new();
    collect_bare_ok_names(&b.stmts, &mut bare_ok);

    for s in found {
        let (head, body, line, col, len) = match s {
            AstStmt::Struct { head, body, line, col, len, .. } => (head, body, line, col, len),
            _ => continue,
        };
        
        let obj = if head.len() >= 2 { &head[1..head.len() - 1] } else { &[][..] };
        if obj.is_empty() {
            continue;
        }
        let Some(ty) = match_object_ty(b, obj, &bare_ok, &BTreeMap::new(), var_owner, 位置{ 行: *line, 列: *col, 长: *len }, cx) else {
            continue;
        };
        
        let Some(all) = ty_variants.get(&ty) else {
            continue;
        };
        let mut covered: Vec<String> = Vec::new();
        let mut has_wild = false;
        for s2 in body {
            if let AstStmt::Struct { head: h2, .. } = s2 {
                if !matches!(h2.first(), Some(Token::Kw("case"))) {
                    continue;
                }
                match h2.get(1) {
                    Some(Token::Ident(n)) if n == "_" => has_wild = true,
                    
                    Some(Token::Ident(n)) | Some(Token::Str(n)) => covered.push(n.to_string()),
                    _ => {}
                }
            }
        }
        if !has_wild {
            let missing: Vec<String> = all
                .iter()
                .filter(|v| !covered.contains(v))
                .map(|v| format!("「{}」", v))
                .collect();
            if !missing.is_empty() {
                cx.err(
                    位置{ 行: *line, 列: *col, 长: *len },
                    CheckCode::E204,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "match 判别",
                        format!("没覆盖变体 {}", missing.join("、")),
                        "",
                        "补上缺的 case，或写 `case _:` 兜底",
                    ),
                    Tone::Fix,
                );
            }
        }
    }
}



fn check_expr_matches_block(
    b: &AstBlock,
    ty_variants: &BTreeMap<String, Vec<String>>,
    var_owner: &BTreeMap<String, String>,
    cx: &mut Ctx,
) {
    let mut bare_ok: BTreeSet<String> = BTreeSet::new();
    collect_bare_ok_names(&b.stmts, &mut bare_ok);
    let mut flat: Vec<(位置, &TokStream, bool)> = Vec::new();
    flatten_stmts(&b.stmts, false, &mut flat);
    for (处, 段, _nested) in flat {
            let _line = 处.行;
        
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        let mut scan: &[Token] = 段;
        if is_add_fn(段) || matches!(段.first(), Some(Token::Kw("fn"))) {
            if let Ok((ps, _after)) = parse_params(段) {
                for (n, t) in ps {
                    params.insert(n, t);
                }
            }
            if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                scan = &段[eq + 1..];
            }
        }
        scan_expr_matches(b, 处, scan, &bare_ok, &params, ty_variants, var_owner, cx);
    }
}



fn scan_expr_matches(
    b: &AstBlock,
    处: 位置,
    段: &[Token],
    bare_ok: &BTreeSet<String>,
    params: &BTreeMap<String, String>,
    ty_variants: &BTreeMap<String, Vec<String>>,
    var_owner: &BTreeMap<String, String>,
    cx: &mut Ctx,
) {
    let line = 处.行;
    let mut k = 0usize;
    while k < 段.len() {
        if !matches!(段.get(k), Some(Token::Kw("match"))) {
            k += 1;
            continue;
        }
        
        let mut depth = 0i32;
        let mut brace = None;
        let mut j = k + 1;
        while j < 段.len() {
            match &段[j] {
                Token::Sym("(") | Token::Sym("[") => depth += 1,
                Token::Sym(")") | Token::Sym("]") => depth -= 1,
                Token::Sym("{") if depth == 0 => {
                    brace = Some(j);
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        let Some(brace) = brace else {
            k += 1;
            continue;
        };
        let obj: Vec<Token> = 段[k + 1..brace].to_vec();
        if obj.is_empty() {
            k += 1;
            continue;
        }
        
        collect_inline_case_bindings(b, 段, brace + 1, cx);
        let (covered, has_wild, bad) = collect_case_coverage(段, brace + 1);
        for n in &bad {
            cx.err(
                处,
                CheckCode::E204,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "match 判别",
                    format!("无字段变体的判别也要写括号——`case {}()`", n),
                    "",
                    "判别统一带括号：有字段写 `case 变体(绑定)`，无字段写 `case 变体()`，兜底写 `case _:`",
                ),
                Tone::Fix,
            );
        }
        if let Some(ty) = match_object_ty(b, &obj, bare_ok, params, var_owner, 处, cx) {
            if let Some(all) = ty_variants.get(&ty) {
                if !has_wild {
                    let missing: Vec<String> = all
                        .iter()
                        .filter(|v| !covered.contains(v))
                        .map(|v| format!("「{}」", v))
                        .collect();
                    if !missing.is_empty() {
                        cx.err(
                            处,
                            CheckCode::E204,
                            &DiagParts::full(
                                format!("第 {} 行", line),
                                "match 判别",
                                format!("没覆盖变体 {}", missing.join("、")),
                                "",
                                "补上缺的 case，或写 `case _:` 兜底",
                            ),
                            Tone::Fix,
                        );
                    }
                }
            }
        }
        k += 1; 
    }
}



fn collect_inline_case_bindings(b: &AstBlock, 段: &[Token], from: usize, cx: &mut Ctx) {
    let mut k = from;
    while k < 段.len() {
        if matches!(段.get(k), Some(Token::Kw("match"))) {
            
            let mut depth = 0i32;
            let mut brace = None;
            let mut j = k + 1;
            while j < 段.len() {
                match &段[j] {
                    Token::Sym("(") | Token::Sym("[") => depth += 1,
                    Token::Sym(")") | Token::Sym("]") => depth -= 1,
                    Token::Sym("{") if depth == 0 => {
                        brace = Some(j);
                        break;
                    }
                    _ => {}
                }
                j += 1;
            }
            if let Some(brace) = brace {
                collect_inline_case_arms(b, &段[brace + 1..], cx);
                k = brace;
            }
        }
        k += 1;
    }
}

fn collect_inline_case_arms(b: &AstBlock, 段: &[Token], cx: &mut Ctx) {
    let mut k = 0;
    while k < 段.len() {
        if !matches!(段.get(k), Some(Token::Kw("case"))) {
            k += 1;
            continue;
        }
        let mut end = k;
        while end < 段.len() && !matches!(段.get(end), Some(Token::Sym(":"))) {
            end += 1;
        }
        collect_case_bindings(b, &段[k..end], cx);
        k = end + 1;
    }
}

fn collect_case_coverage(段: &[Token], mut j: usize) -> (Vec<String>, bool, Vec<String>) {
    let mut covered = Vec::new();
    let mut has_wild = false;
    let mut bad = Vec::new();
    loop {
        if j >= 段.len() || matches!(&段[j], Token::Sym("}")) {
            break;
        }
        if !matches!(&段[j], Token::Kw("case")) {
            j += 1;
            continue;
        }
        j += 1;
        
        let mut d = 0i32;
        let pat_start = j;
        while j < 段.len() {
            match &段[j] {
                Token::Sym("(") => d += 1,
                Token::Sym(")") => d -= 1,
                Token::Sym(":") if d == 0 => break,
                _ => {}
            }
            j += 1;
        }
        let pat = &段[pat_start..j];
        match pat.first() {
            Some(Token::Ident(n)) if n == "_" => has_wild = true,
            
            Some(Token::Ident(n)) | Some(Token::Str(n)) => {
                covered.push(n.to_string());
                if pat.len() == 1 {
                    bad.push(n.to_string());
                }
            }
            _ => {}
        }
        if j < 段.len() {
            j += 1; 
        }
        
        let mut d3 = 0i32;
        while j < 段.len() {
            match &段[j] {
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
        if matches!(段.get(j), Some(Token::Sym(","))) {
            j += 1;
        }
    }
    (covered, has_wild, bad)
}

fn pass2_calls(ast: &AstProgram, cx: &mut Ctx) {
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        calls_block(b, cx);
    }
}

fn calls_block(b: &AstBlock, cx: &mut Ctx) {
    calls_stmts(&b.stmts, b, cx);
}


fn calls_stmts(stmts: &[AstStmt], b: &AstBlock, cx: &mut Ctx) {
    for s in stmts {
        match s {
            AstStmt::Line { line, col, len, kind, .. } => {
                let 段 = kind.tokens();
                
                
                let fn名 = if matches!(*kind, prion_parser::LineKind::AddFn(_)) {
                    
                    
                    if is_add_fn(段) && parse_params(段).is_err() {
                        None
                    } else {
                        stmt_fn_name(段)
                    }
                } else {
                    None
                };
                if let Some(n) = fn名 {
                    let pb = fn_scope_block(b, &n, &[]);
                    calls_expr(&pb, 位置{ 行: *line, 列: *col, 长: *len }, 段, cx);
                } else {
                    calls_expr(b, 位置{ 行: *line, 列: *col, 长: *len }, 段, cx);
                }
            }
            AstStmt::Struct { line, col, len, head, body, kind, .. } => {
                if *kind != prion_parser::StructKind::Spawn {
                    calls_expr(b, 位置{ 行: *line, 列: *col, 长: *len }, head, cx);
                }
                calls_stmts(body, b, cx);
            }
            AstStmt::Fn { head, body, .. } => {
                if let Some(n) = stmt_fn_name(head) {
                    let pb = fn_scope_block(b, &n, body);
                    calls_stmts(body, &pb, cx);
                }
            }
        }
    }
}


fn calls_expr(b: &AstBlock, 处: 位置, 段: &[Token], cx: &mut Ctx) {
    let line = 处.行;
    
    if is_add_type(段) {
        return;
    }
    
    if let Some(Token::Ident(n)) = 段.first() {
        let 是写位置 = matches!(段.get(1), Some(Token::Sym(".")) | Some(Token::Sym("[")))
            && 段.iter().any(|t| *t == Token::Sym("="));
        if 是写位置 {
            let 写法 = if matches!(段.get(1), Some(Token::Sym("."))) {
                format!("val \"{}\".字段 = 值", n)
            } else {
                format!("val \"{}\"[下标] = 值", n)
            };
            cx.err(
                处,
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "写位置",
                    format!("「{}」左边的裸名没加 val——写 `{}`", n, 写法),
                    "",
                    "写位置也要显式：字段写与下标写都写成 `val \"名\"…` 形态，不留裸名",
                )
                .预期("写位置与读位置同一套名字规则，只有一种写法"),
                Tone::Fix,
            );
        }
    }
    let expr: &[Token] = if is_add_val(段) || is_add_fn(段) || is_add_outs_fn(段) {
        match 段.iter().position(|t| *t == Token::Sym("=")) {
            Some(eq) => &段[eq + 1..],
            None => &[],
        }
    } else if matches!(段.first(), Some(Token::Kw("val"))) {
        match 段.iter().position(|t| *t == Token::Sym("=")) {
            Some(eq) => &段[eq + 1..],
            None => &[],
        }
    } else if matches!(段.last(), Some(Token::Sym(":"))) && 段.len() >= 2 {
        &段[1..段.len() - 1]
    } else {
        段
    };
    let mut i = 0usize;
    while i < expr.len() {
        
        if prion_lexer::asm::是asm词(&expr[i])
            && matches!(expr.get(i + 1), Some(Token::Sym("(")))
        {
            if !cx.unsafe_blocks.contains(&b.name) {
                cx.err(
                    处,
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "危险模式",
                        "「asm」是危险模式专属——当前块未进入危险模式",
                        "",
                        "把这块的开块行写成 `#: 块名 b unsafe`（危险模式），再在这里用它",
                    )
                    .预期("内联汇编只能出现在 unsafe 块里")
                    .详解(
                        "asm 是 unsafe 的第四类能力（与裸地址读写、C 互操作并列）：\
                         汇编绕开了 Prion 的全部类型与内存规则，只在 `#: 块名 b unsafe` 的块里放行。",
                    ),
                    Tone::Fix,
                );
            }
            if let Some(内区) = prion_lexer::asm::括号内(expr, i + 1) {
                if let Err(原因) = prion_lexer::asm::解析隔离区(内区) {
                    cx.err(
                        处,
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", line),
                            "内联汇编",
                            format!("asm 隔离区不合规：{}", 原因),
                            "括号内是隔离区，按 asm 自己的微语法写",
                            prion_lexer::asm::映射表说明(),
                        ),
                        Tone::Fix,
                    );
                }
                let mut depth = 0i32;
                let mut j = i + 1;
                while j < expr.len() {
                    match &expr[j] {
                        Token::Sym("(") => depth += 1,
                        Token::Sym(")") => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                i = j + 1;
                continue;
            }
        }
        match &expr[i] {
            
            Token::Kw("call") => {
                if matches!(expr.get(i + 1), Some(Token::Kw("fn"))) {
                    if let Some(Token::Str(n) | Token::Ident(n)) = expr.get(i + 2) {
                        if matches!(expr.get(i + 3), Some(Token::Sym("("))) {
                            let args = call_args(expr, i + 3);
                            check_call(b, 处, n, &args, cx);
                        }
                    }
                } else if let Some((module, blk, fname, at)) = parse_qualified_call(expr, i) {
                    
                    let args = call_args(expr, at);
                    let sig = match &module {
                        Some(m) => cx.fns_mod.get(&format!("{}|{}|{}", m, blk, fname)).cloned(),
                        None => cx.键(&blk, &fname).and_then(|k| cx.fns.get(&k)).cloned(),
                    };
                    if let Some(sig) = sig {
                        if module.is_none()
                            && !sig.is_outs
                            && !cx.键(&blk, &fname).is_some_and(|k| cx.fn_has_body.contains(&k))
                        {
                            cx.err(
                                处,
                                CheckCode::E201,
                                &DiagParts::full(
                                    format!("第 {} 行", line),
                                    "调函数",
                                    format!("fn「{}」只有声明，还没有体", fname),
                                    "",
                                    format!("用 `fn \"{}\" (参数) 返回类型 = 表达式` 给它补体", fname),
                                )
                                .预期("声明过的 fn 要补体才能调用"),
                                Tone::Fix,
                            );
                        } else {
                            check_sig_args(b, 处, &fname, &sig, &args, cx);
                        }
                    }
                } else if matches!(expr.get(i + 1), Some(Token::Kw("val"))) {
                    if let (Some(Token::Str(n) | Token::Ident(n)), Some(Token::Sym("("))) =
                        (expr.get(i + 2), expr.get(i + 3))
                    {
                        let fty = cx.find_local(&b.name, n).map(|l| l.ty.clone());
                        if let Some((params, ret)) = fty.as_deref().and_then(parse_fn_ty) {
                            let args = call_args(expr, i + 3);
                            let sig = FnSig { params, ret, is_outs: false };
                            check_sig_args(b, 处, n, &sig, &args, cx);
                        }
                    }
                }
            }
            Token::Kw("fn") => {
                
                
                if let Some(Token::Str(n) | Token::Ident(n)) = expr.get(i + 1) {
                    let fn块 = if b.name.contains('#') {
                        b.name.split('#').next().unwrap_or(&b.name).to_string()
                    } else {
                        b.name.clone()
                    };
                    if !matches!(expr.get(i + 2), Some(Token::Sym("(")))
                        && !matches!(expr.get(i.wrapping_sub(1)), Some(Token::Kw("call")))
                        && !cx.键(&fn块, &n).is_some_and(|k| cx.fns.contains_key(&k))
                    {
                        cx.err(
                            处,
                            CheckCode::E201,
                            &DiagParts::full(
                                format!("第 {} 行", line),
                                "函数值",
                                format!("fn「{}」还没建过", n),
                                "",
                                format!("先用 `add fn \"{}\" (参数) 返回类型 = …` 建出来", n),
                            )
                            .预期("函数值只能取自本块已声明的 fn"),
                            Tone::Fix,
                        );
                    }
                }
            }
            Token::Ident(n) => {


                if UNSAFE_ONLY.iter().any(|u| *u == n.as_ref()) && !cx.unsafe_blocks.contains(&b.name) {
                    cx.err(
                        处,
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", line),
                            "危险模式",
                            format!("「{}」是危险模式专属——当前块未进入危险模式", n),
                            "",
                            "把这块的开块行写成 `#: 块名 b unsafe`（危险模式），再在这里用它",
                        )
                        .预期("危险模式专属词只能出现在 unsafe 块里")
                        .详解(
                            "「未进入危险模式」和「这个名字不存在」是两回事：\
                             `read_8` / `write_8` 这类裸地址读写、以及 `str_to_c` / `str_from_c` 这类 C 互操作词，\
                             只在 `#: 块名 b unsafe` 的块里放行；在普通块里它们是**被模式挡住了**，不是没定义。",
                        ),
                        Tone::Fix,
                    );
                }
                
            }
            _ => {}
        }
        if let Token::Ident(n) = &expr[i] {
            if BUILTINS.iter().any(|u| *u == n.as_ref()) {
                if n.as_ref() == "concat" && matches!(expr.get(i + 1), Some(Token::Sym("("))) {
                    let args = call_args(expr, i + 1);
                    for (k, a) in args.iter().enumerate() {
                        let got = infer_named(a, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types)
                            .or_else(|| infer(a, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types).map(|s| s.to_string()));
                        if let Some(got) = got {
                            if got != UNKNOWN && !类型相容("str", &got) {
                                cx.err(
                                    处,
                                    CheckCode::E202,
                                    &DiagParts::full(
                                        format!("第 {} 行", 处.行),
                                        "调 concat",
                                        format!("concat 第 {} 段要 str，给了 {}", k + 1, got),
                                        "",
                                        "把这一段改成 str 的值（数字先 str(值)）",
                                    )
                                    .预期("concat 的每一段都必须是 str")
                                    .详解("concat 把若干段字符串按顺序接成一段；数值转文字用 str(值)，别把数字直接丢进去。"),
                                    Tone::Fix,
                                );
                            }
                        }
                    }
                }
                i += 1;
                continue; 
            }
            if matches!(expr.get(i + 1), Some(Token::Sym("("))) {
                
                let mut 本次实例: Option<String> = None;
                if !cx.types.contains_key(n.as_ref()) {
                    if let Some((_l, 参数, 字段, _变体)) = cx.泛型.get(n.as_ref()).cloned() {
                        let args = call_args(expr, i + 1);
                        let mut 绑定: Vec<Option<String>> = vec![None; 参数.len()];
                        for (k, a) in args.iter().enumerate() {
                            if k >= 字段.len() {
                                break;
                            }
                            let got = infer_named(a, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types)
                                .or_else(|| infer(a, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types).map(|s| s.to_string()));
                            let got = match got {
                                Some(g) => g,
                                None => continue,
                            };
                            let pat = 字段[k].1.clone();
                            if let Some(pi) = 参数.iter().position(|p| *p == pat) {
                                绑定[pi] = Some(got);
                            } else if let Some(pi) = 参数.iter().position(|p| pat == format!("{}*", p)) {
                                if let Some(内) = got.strip_suffix('*') {
                                    绑定[pi] = Some(内.to_string());
                                }
                            }
                        }
                        if 绑定.iter().all(|x| x.is_some()) {
                            let 实参: Vec<String> = 绑定.into_iter().map(|x| x.unwrap()).collect();
                            let 实例名 = format!("{}({})", n, 实参.join(", "));
                            let 新字段: Vec<(String, String, bool)> = 字段
                                .iter()
                                .map(|(f, t)| (f.clone(), 替换类型参数(t, &参数, &实参), false))
                                .collect();
                            cx.types.insert(实例名.clone(), (处, cx.cur_file.clone(), 新字段));
                            本次实例 = Some(实例名);
                        }
                    }
                }
                let 构造键: String = if cx.types.contains_key(n.as_ref()) {
                    n.to_string()
                } else if let Some(名) = 本次实例 {
                    名
                } else {
                    
                    String::new()
                };
                if !构造键.is_empty() {
                    
                    let fields: Vec<(String, String)> = cx.types.get(&构造键).unwrap().2.iter().map(|(n, t, _)| (n.clone(), t.clone())).collect();
                    let args = call_args(expr, i + 1);
                    if args.len() != fields.len() {
                        cx.err(
                            处,
                            CheckCode::E202,
                            &DiagParts::full(
                                format!("第 {} 行", line),
                                "构造",
                                format!("类型「{}」有 {} 个字段，给了 {} 个", 构造键, fields.len(), args.len()),
                                "",
                                "按定义给够字段",
                            ),
                            Tone::Fix,
                        );
                    } else {
                        for (k, a) in args.iter().enumerate() {
                            let got = infer_named(a, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types)
                                .or_else(|| infer(a, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types).map(|s| s.to_string()));
                            if let Some(got) = got {
                                
                                let want = fields[k].1.clone();
                                if !ty_compatible(cx, &want, &got) {
                                    cx.err(
                                        处,
                                        CheckCode::E202,
                                        &DiagParts::full(
                                            format!("第 {} 行", line),
                                            "构造",
                                            format!("字段「{}」要 {}，给了 {}", fields[k].0, want, got),
                                            "",
                                            format!("改成 {} 的值", want),
                                        ),
                                        Tone::Fix,
                                    );
                                }
                            }
                        }
                    }
                } else if !cx.variants.contains(n.as_ref()) {

                    let is_outs = cx
                        .fns
                        .get(&(cx.names.intern(&b.name), cx.names.intern(n)))
                        .map(|s| s.is_outs)
                        .unwrap_or(false);
                    if is_outs && !cx.unsafe_blocks.contains(&b.name) {
                        cx.err(
                            处,
                            CheckCode::E201,
                            &DiagParts::full(
                                format!("第 {} 行", line),
                                "调外部函数",
                                format!("外部函数「{}」只能在 unsafe 块里调", n),
                                "",
                                "把这块的开块行写成 `#: 块名 b unsafe`",
                            ),
                            Tone::Fix,
                        );
                    } else {
                        
                        let key = (cx.names.intern(&b.name), cx.names.intern(n.as_ref()));
                        if cx.fns.contains_key(&key) {
                            cx.err(
                                处,
                                CheckCode::E201,
                                &DiagParts::full(
                                    format!("第 {} 行", line),
                                    "调函数",
                                    format!("调用要写 `call`——形如 `call fn \"{}\"(…)`", n),
                                    "",
                                    "把 `名(…)` 改成 `call fn \"名\"(…)`（同块调用也要写）",
                                )
                                .预期("调用户 fn 必须写 `call fn \"名\"(…)`（全面显式）"),
                                Tone::Fix,
                            );
                        } else {
                            check_call(b, 处, n, &call_args(expr, i + 1), cx);
                        }
                    }
                }
            }
        }
        i += 1;
    }
    
    scan_bare_reads(b, 处, expr, 段, cx);
}


fn top_assign_pos(段: &[Token]) -> Option<usize> {
    let mut depth = 0i32;
    for (k, t) in 段.iter().enumerate() {
        match t {
            Token::Sym("(") | Token::Sym("[") | Token::Sym("{") => depth += 1,
            Token::Sym(")") | Token::Sym("]") | Token::Sym("}") => depth -= 1,
            Token::Sym("=") if depth == 0 => return Some(k),
            _ => {}
        }
    }
    None
}



fn parse_qualified_call(expr: &[Token], i: usize) -> Option<(Option<String>, String, String, usize)> {
    let mut j = i + 1;
    let mut module: Option<String> = None;
    match expr.get(j) {
        Some(Token::HashColon) => j += 1,
        Some(Token::HashHashColon) => {
            j += 1;
            let mut path = match expr.get(j) {
                Some(Token::Ident(n)) => {
                    j += 1;
                    n.to_string()
                }
                _ => return None,
            };
            while matches!(expr.get(j), Some(Token::Sym("."))) {
                j += 1;
                if let Some(Token::Ident(n)) = expr.get(j) {
                    path.push('.');
                    path.push_str(n.as_ref());
                    j += 1;
                } else {
                    return None;
                }
            }
            if !matches!(expr.get(j), Some(Token::HashColon)) {
                return None;
            }
            j += 1;
            module = Some(path.to_string());
        }
        _ => return None,
    }
    let blk = match expr.get(j) {
        Some(Token::Ident(n)) => n.clone(),
        Some(Token::Kw(k)) => (*k).to_string().into(),
        _ => return None,
    };
    j += 1;
    
    if !matches!(expr.get(j), Some(Token::Kw("fn"))) {
        return None;
    }
    j += 1;
    let fname = match expr.get(j) {
        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
        _ => return None,
    };
    j += 1;
    if !matches!(expr.get(j), Some(Token::Sym("("))) {
        return None;
    }
    Some((module, blk.to_string(), fname.to_string(), j))
}




fn 是名字位前驱(expr: &[Token], i: usize) -> bool {
    matches!(
        expr.get(i),
        Some(Token::Kw("val")) | Some(Token::Kw("fn")) | Some(Token::Kw("type"))
    )
}

fn scan_bare_reads(b: &AstBlock, 处: 位置, expr: &[Token], full: &[Token], cx: &mut Ctx) {
    let line = 处.行;
    if matches!(full.first(), Some(Token::Kw("case"))) {
        return; 
    }
    
    for k in 0..expr.len() {
        if !matches!(expr[k], Token::Kw("val")) || k == 0 {
            continue;
        }
        if matches!(expr[k - 1], Token::Kw("call") | Token::Kw("take")) {
            continue;
        }
        
        let mut j = k;
        while j > 0 && matches!(expr[j - 1], Token::Ident(_) | Token::Str(_)) {
            j -= 1;
        }
        if j > 0 && matches!(expr[j - 1], Token::HashColon | Token::HashHashColon) {
            continue;
        }
        let n = match expr.get(k + 1) {
            Some(Token::Str(x)) | Some(Token::Ident(x)) => x.clone(),
            _ => continue,
        };
        cx.err(
            处,
            CheckCode::E201,
            &DiagParts::full(
                format!("第 {} 行", line),
                "读值",
                format!("「{}」前面缺 call —— 读值写 `call val \"{}\"`", n, n),
                "",
                format!("改成 `call val \"{}\"`；改值仍是 `val \"{}\" = 值`（= 已承担动词）", n, n),
            )
            .预期("读 val 是动作，要带 call；改 val 有 = 当动词，不再带 call"),
            Tone::Fix,
        );
    }
    
    let mut k = 0usize;
    while k < expr.len() {
        if prion_lexer::asm::是asm词(&expr[k]) && matches!(expr.get(k + 1), Some(Token::Sym("("))) {
            return;
        }
        k += 1;
    }
    
    if let (Some(Token::Ident(n)), Some(Token::Sym("="))) = (expr.first(), expr.get(1)) {
        let 裸键 = (cx.names.intern(&b.name), cx.names.intern(n.as_ref()));
        if cx.local_idx.contains_key(&裸键) {
            cx.err(
                处,
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "改值",
                    format!("改值要写 `val \"{}\" = 值`——裸名「{}」消不了歧义", n, n),
                    "",
                    "改值的行首写 `val \"名\" = 新值`；声明才用 `add val`",
                )
                .预期("改值必须显式写 `val \"名\" = 值`"),
                Tone::Fix,
            );
            return;
        }
    }
    
    let mut vi = 0usize;
    while vi + 1 < expr.len() {
        if matches!(expr[vi], Token::Kw("val")) {
            if let Some(Token::Str(n)) = expr.get(vi + 1) {
                let 名号 = cx.names.intern(n);
                let cur = (cx.names.intern(&b.name), 名号);
                let is_write = matches!(expr.get(vi + 2), Some(Token::Sym("=")));
                let qualified = vi > 0 && matches!(expr[vi - 1], Token::HashColon);
                if !is_write && !qualified && !cx.local_idx.contains_key(&cur) {
                    let 本块 = b.name.clone();
                    if cx.local_idx.keys().any(|k| k.1 == 名号
                        && cx.names.text(k.0).strip_prefix(&本块).is_some_and(|r| r.starts_with('#'))) {
                        cx.err(
                            处,
                            CheckCode::E201,
                            &DiagParts::full(
                                format!("第 {} 行", line),
                                "取值",
                                format!("「{}」是语句体 fn 里的局部——函数外不能用", n),
                                "fn 的局部只活在该函数体里",
                                format!("在块里另建一个 val，或用 `call fn \"…\"` 把值取出来"),
                            )
                            .预期("fn 局部不能在函数体外读"),
                            Tone::Fix,
                        );
                        return;
                    }
                    
                    let 别处有 = cx.local_idx.keys().any(|k| k.1 == 名号);
                    if !别处有 {
                        cx.err(
                            处,
                            CheckCode::E201,
                            &DiagParts::full(
                                format!("第 {} 行", line),
                                "取值",
                                format!("「{}」没建过——本块和引用链里都没有这个名字", n),
                                "",
                                "先 `add val \"名\" 类型 = 值` 建它；跨块读要写 `call #:块名 val \"名\"`",
                            )
                            .预期("读的 val 必须先在当前块或引用链里建过"),
                            Tone::Fix,
                        );
                        return;
                    }
                }
            }
            vi += 2;
            continue;
        }
        vi += 1;
    }
    let mut params: BTreeSet<String> = BTreeSet::new();
    if is_add_fn(full) || matches!(full.first(), Some(Token::Kw("fn"))) {
        if let Ok((ps, _)) = parse_params(full) {
            for (n, _) in ps {
                params.insert(n);
            }
        }
    }
    let for_head = matches!(full.first(), Some(Token::Kw("for")));
    let eq = top_assign_pos(expr);
    let mut i = 0usize;
    while i < expr.len() {
        let n = match &expr[i] {
            Token::Ident(n) => n,
            _ => {
                i += 1;
                continue;
            }
        };
        
        if i > 0 {
            match expr.get(i - 1) {
                Some(Token::Sym(".")) | Some(Token::HashColon) | Some(Token::HashHashColon)
                | Some(Token::HashHashHashColon) => {
                    i += 1;
                    continue;
                }
                _ => {}
            }
        }

        if i > 0 && 是名字位前驱(expr, i - 1) {
            i += 1;
            continue;
        }
        
        if matches!(expr.get(i + 1), Some(Token::Sym("("))) {
            i += 1;
            continue;
        }
        
        if BUILTINS.iter().any(|u| *u == n.as_ref()) || UNSAFE_ONLY.iter().any(|u| *u == n.as_ref()) {
            i += 1;
            continue;
        }
        if cx.types.contains_key(n.as_ref()) || cx.variants.contains(n.as_ref()) {
            i += 1;
            continue;
        }
        if cx.键(&b.name, n).is_some_and(|k| cx.fns.contains_key(&k))
            || cx.键(原块(&b.name), n).is_some_and(|k| cx.fns.contains_key(&k))
        {
            i += 1;
            continue;
        }
        if params.contains(n.as_ref()) {
            i += 1;
            continue;
        }
        
        
        if !cx.bare_ok.is_empty() && cx.bare_ok.contains(&format!("{}|{}", b.name, n)) {
            i += 1;
            continue;
        }
        
        if n == "in" || n == "max" {
            i += 1;
            continue;
        }
        if for_head && i == 0 {
            i += 1;
            continue;
        }
        
        if let Some(e) = eq {
            if i < e {
                i += 1;
                continue;
            }
        }
        
        
        let 是fn内 = b.name.contains('#');
        let 原块有 = 是fn内 && {
            let plain = b.name.split('#').next().unwrap_or("");
            let 原键 = (cx.names.intern(plain), cx.names.intern(n.as_ref()));
            cx.local_idx.contains_key(&原键)
        };
        if 原块有 {
            cx.err(
                处,
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "取值",
                    format!("裸名「{}」——取 val 请写 val \"{}\"", n, n),
                    "",
                    "全面显式：fn 与 val 可重名，裸名消不了歧义。把读取处写成 val \"名\"",
                )
                .预期("读 val 必须写 val \"名\""),
                Tone::Fix,
            );
            return;
        }
        let 本键 = (cx.names.intern(&b.name), cx.names.intern(n.as_ref()));
        if cx.local_idx.contains_key(&本键) {
            cx.err(
                处,
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", line),
                    "取值",
                    format!("裸名「{}」——取 val 请写 val \"{}\"", n, n),
                    "",
                    "全面显式：fn 与 val 可重名，裸名消不了歧义。把读取处写成 val \"名\"",
                )
                .预期("读 val 必须写 val \"名\""),
                Tone::Fix,
            );
            return;
        }
        i += 1;
    }
}
fn pass1_uses(ast: &AstProgram, cx: &mut Ctx) {
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        uses_block(b, cx);
    }
}

fn uses_block(b: &AstBlock, cx: &mut Ctx) {
    uses_stmts(&b.stmts, b, cx);
}


fn uses_stmts(stmts: &[AstStmt], b: &AstBlock, cx: &mut Ctx) {
    for s in stmts {
        match s {
            AstStmt::Line { line, col, len, kind, .. } => {
                let 段 = kind.tokens();
                
                
                if matches!(段.first(), Some(Token::Kw("fn"))) {
                    if let Some(n) = stmt_fn_name(段) {
                        let pb = fn_scope_block(b, &n, &[]);
                        uses_line(&pb, 位置{ 行: *line, 列: *col, 长: *len }, 段, cx);
                        continue;
                    }
                }
                uses_line(b, 位置{ 行: *line, 列: *col, 长: *len }, 段, cx)
            }
            AstStmt::Struct { line, col, len, head, body, kind, .. } => {
                if *kind != prion_parser::StructKind::Spawn {
                    uses_expr_tokens(b, 位置{ 行: *line, 列: *col, 长: *len }, head, cx);
                }
                uses_stmts(body, b, cx);
            }
            AstStmt::Fn { head, body, .. } => {
                if let Some(n) = stmt_fn_name(head) {
                    let pb = fn_scope_block(b, &n, body);
                    uses_stmts(body, &pb, cx);
                }
            }
        }
    }
}



fn flatten_stmts<'a>(
    stmts: &'a [AstStmt],
    nested: bool,
    out: &mut Vec<(位置, &'a TokStream, bool)>,
) {
    for s in stmts {
        match s {
            AstStmt::Line { line, col, len, kind, .. } => {
                out.push((位置{ 行: *line, 列: *col, 长: *len }, kind.tokens(), nested))
            }
            AstStmt::Struct { line, col, len, head, body, .. } | AstStmt::Fn { line, col, len, head, body, .. } => {
                out.push((位置{ 行: *line, 列: *col, 长: *len }, head, nested));
                flatten_stmts(body, true, out);
            }
        }
    }
}

fn uses_line(b: &AstBlock, 处: 位置, 段: &[Token], cx: &mut Ctx) {
    let line = 处.行;
    
    if 段.len() == 1 && 段.first().map(是裸join词).unwrap_or(false) {
        return;
    }
    if is_add_type(段) || is_add_fn(段) || is_add_outs_fn(段) {
        return;
    }
    if is_add_val(段) {
        
        
        
        let 规范 = prion_parser::建val规范(段);
        let 段 = &规范[..];
        if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
            let decl_name = match 段.get(2) {
                Some(Token::Str(n)) | Some(Token::Ident(n)) => Some(n.to_string()),
                _ => None,
            };
            
            
            let prev_suppress = cx.suppress_key.take();
            if let Some(n) = decl_name.as_ref() {
                cx.suppress_key = Some((cx.names.intern(&b.name), cx.names.intern(n)));
            }
            uses_expr_tokens(b, 处, &段[eq + 1..], cx);
            cx.suppress_key = prev_suppress;
        }
        return;
    }
    
    if matches!(段.first(), Some(Token::Kw("val"))) {
        if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 段.get(1) {
            if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                let 本块有值 = cx.has_local(&b.name, &n);
                let 别块有常量 = !本块有值
                    && cx.常量.iter().any(|k| k.ends_with(&format!("|{}", n)));
                if cx.常量.contains(&format!("{}|{}", b.name, n)) || 别块有常量 {
                    cx.err(
                        处,
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", line),
                            "改常量",
                            format!("常量「{}」不许改赋", n),
                            "它是 add fix 声明的编译期常量，值在编译期定死",
                            "要能改的值就改用 add val 声明",
                        ),
                        Tone::Fix,
                    );
                    return;
                }
                require_known(b, 处, n, cx);
                uses_expr_tokens(b, 处, &段[eq + 1..], cx);
            } else {
                require_known(b, 处, n, cx);
            }
        }
        return;
    }
    uses_expr_tokens(b, 处, 段, cx);
}

fn require_known(b: &AstBlock, 处: 位置, name: &str, cx: &mut Ctx) {
    let line = 处.行;
    
    if name == "_" {
        return;
    }
    let known = cx.has_local(&b.name, &name);
    if known {

        
        let 声明键 = (cx.names.intern(&b.name), cx.names.intern(&name));
        if let Some(&decl) = cx.val_decl_line.get(&声明键) {
            if decl > line {
                cx.err(
                    处,
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "用名字",
                        format!("val「{}」用在它被建出来之前——它到第 {} 行才建", name, decl),
                        "",
                        format!("把建「{}」的那行 `add val` 移到这一行之前", name),
                    )
                    .预期("名字要先建出来，再用它")
                    .详解(
                        "Prion 是「先建后用」：val 的读和写都要排在它的 `add val` 之后——排在前面会读到还没放值的内存。`add fn`（函数）与 `add type`（类型）是声明，不受这条约束，可以先用后写。",
                    ),
                    Tone::Fix,
                );
            }
        }
        return;
    }
    cx.err(
        处,
        CheckCode::E201,
        &DiagParts::full(
            format!("第 {} 行", line),
            "用名字",
            format!("val「{}」还没建过", name),
            "",
            format!("在用它之前先建：add val \"{}\" <类型> = <初值>", name),
        )
        .预期("这里的名字应当是【本块已建过】的 val / 函数")
        .详解(
 "Prion 的规则是「先建后用」：名字要先由 `add val`（值）/ `add fn`（函数）/ `add type`（类型）建出来，才能在表达式里读；如果名字在别的块里，跨块取 val 要写 `call #:块 val \"名\"`；跨块调函数要写 `call #:块 fn \"名\"(…)`；同块直接用。",
        ),
        Tone::Fix,
    );
    
    cx.add_local(LocalInfo {
        block: b.name.clone(),
        name: name.to_string(),
        ty: UNKNOWN.to_string(),
        键: 空键,
                                    class: StorageClass::Tmp,
    });
}


fn 摘掉asm区(段: &[Token]) -> Vec<Token> {
    let mut 出 = Vec::with_capacity(段.len());
    let mut i = 0usize;
    while i < 段.len() {
        if prion_lexer::asm::是asm词(&段[i]) && matches!(段.get(i + 1), Some(Token::Sym("("))) {
            let mut depth = 0i32;
            let mut j = i + 1;
            while j < 段.len() {
                match &段[j] {
                    Token::Sym("(") => depth += 1,
                    Token::Sym(")") => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            i = j + 1;
            continue;
        }
        出.push(段[i].clone());
        i += 1;
    }
    出
}


fn uses_expr_tokens(b: &AstBlock, 处: 位置, 段: &[Token], cx: &mut Ctx) {
    let _line = 处.行;
    
    let 净: Vec<Token> = 摘掉asm区(段);
    let 段: &[Token] = &净;
    
    for i in 0..段.len() {
        if !matches!(段.get(i), Some(Token::Kw("call")))
            || !matches!(段.get(i + 1), Some(Token::Kw("val")))
        {
            continue;
        }
        let n = match 段.get(i + 2) {
            Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
            _ => continue,
        };
        let 本块是常量 = cx.常量.contains(&format!("{}|{}", b.name, n));
        let 别块有常量 = cx.常量.iter().any(|k| k.ends_with(&format!("|{}", n)));
        let 本块有val = !本块是常量 && cx.has_local(&b.name, &n);
        if 别块有常量 && !本块有val {
            cx.err(
                处,
                CheckCode::E201,
                &DiagParts::full(
                    format!("第 {} 行", 处.行),
                    "读常量",
                    format!("常量「{}」要用 `call fix \"{}\"` 读", n, n),
                    "",
                    format!("把 `call val \"{}\"` 改成 `call fix \"{}\"`", n, n),
                )
                .预期("常量有独立的引用动词 call fix")
                .详解(
                    "`val` 与 `fix` 是两个家族：val 是可改的运行期值，fix 是编译期定死的常量。读 val 用 `call val \"名\"`，读常量用 `call fix \"名\"`；两者不混用。",
                ),
                Tone::Fix,
            );
            return;
        }
    }
    let mut skip_paren = 0i32;
    let mut field_skip: Vec<usize> = Vec::new();
    for (i, t) in 段.iter().enumerate() {
        match t {
            Token::Sym("(") => skip_paren += 1,
            Token::Sym(")") => skip_paren -= 1,
            
            Token::Sym(".") => {
                if matches!(段.get(i + 1), Some(Token::Ident(_)) | Some(Token::Str(_))) {
                    field_skip.push(i + 1);
                }
            }
            
            Token::HashColon => {
                if matches!(段.get(i + 1), Some(Token::Ident(_))) {
                    field_skip.push(i + 1);
                }
            }
            
            Token::HashHashColon => {
                if matches!(段.get(i + 1), Some(Token::Ident(_)) | Some(Token::Str(_))) {
                    field_skip.push(i + 1);
                }
            }
            Token::Ident(n) => {
                
                if n == "in" || n == "max" {
                    continue;
                }
                
                if matches!(段.get(i + 1), Some(Token::Sym("("))) {
                    continue;
                }
                if field_skip.contains(&i) {
                    continue; 
                }
                
                if i > 0 && matches!(段.get(i - 1), Some(Token::Kw("case"))) {
                    continue;
                }
                require_known(b, 处, n, cx);
            }
            _ => {}
        }
        let _ = skip_paren;
    }
}




fn container_ty_at(段: &[Token], i: usize) -> Option<String> {
    let name = match 段.get(i)? {
        Token::Kw(k) => (*k).to_string(),
        _ => return None,
    };
    if name == "fn" {
        return parse_type_ref(段, i).ok().map(|(t, _)| t);
    }
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
            return parse_type_ref(ts, 0).ok().map(|(t, _)| t);
        }
        let base = match ts.first()? {
            Token::Kw(t) => (*t).to_string(),
            Token::Ident(n) => n.to_string(),
            _ => return None,
        };
        if matches!(ts.get(1), Some(Token::Sym("*"))) {
            Some(format!("{}*", base))
        } else {
            Some(base)
        }
    };
    if name == "map" {
        let args = split_commas(inner);
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


fn base_ty(s: &str) -> &str {
    s.split('(').next().unwrap_or(s)
}



const WIDTHS: &[&str] = &[
    "int_8", "int_16", "int_32", "int_64", "uint_8", "uint_16", "uint_32", "uint_64",
];

fn 解析精化条件(ty: &str) -> Option<(String, i64)> {
    if !ty.ends_with(')') { return None; }
    let open = ty.find('(')?;
    let cond = &ty[open + 1..ty.len() - 1];
    for op in [">=", "<=", ">", "<"] {
        if let Some(rest) = cond.strip_prefix(op) {
            if let Ok(n) = rest.trim().parse::<i64>() {
                return Some((op.to_string(), n));
            }
        }
    }
    None
}

fn 提取整数字面量(toks: &[Token]) -> Option<i64> {
    let mut i = 0;
    while i < toks.len() && !matches!(toks[i], Token::Int(_)) { i += 1; }
    let n = match toks.get(i) { Some(Token::Int(n)) => *n, _ => return None };
    let neg = toks[..i].iter().filter(|t| matches!(t, Token::Sym("-"))).count() % 2 == 1;
    Some(if neg { -n } else { n })
}


fn 展开别名(cx: &Ctx, ty: &str) -> String {
    let mut cur = ty.to_string();
    for _ in 0..8 {
        match cx.aliases.get(&cur) {
            Some(next) if *next != cur => cur = next.clone(),
            _ => break,
        }
    }
    cur
}

fn ty_compatible(cx: &Ctx, decl: &str, got: &str) -> bool {
    let decl_own = 展开别名(cx, decl);
    let got_own = 展开别名(cx, got);
    let 剥 = |s: &str| -> String {
        if s.ends_with(')') {
            if let Some(i) = s.find('(') {
                if matches!(s.as_bytes().get(i + 1), Some(b'>') | Some(b'<')) {
                    return s[..i].to_string();
                }
            }
        }
        s.to_string()
    };
    let (decl_own, got_own) = (剥(&decl_own), 剥(&got_own));
    let (decl, got) = (decl_own.as_str(), got_own.as_str());
    if decl == got {
        return true;
    }

    if got == "null" {
        return decl.ends_with('*');
    }
    if decl == "null" {
        return got.ends_with('*');
    }
    
    if WIDTHS.contains(&decl) || WIDTHS.contains(&got) {
        return (WIDTHS.contains(&decl) && WIDTHS.contains(&got))
            || decl == "int"
            || got == "int";
    }
    let (bd, bg) = (base_ty(decl), base_ty(got));
    if bd != bg || !matches!(bd, "list" | "map" | "que" | "stack") {
        return false;
    }
    decl == bd || got == bg
}


fn infer_named(
    段: &[Token],
    locals: &[LocalInfo],
    idx: &局部索引,
    names: &prion_types::Names,
    block: &str,
    types: &TypeTable,
) -> Option<String> {
    
    if matches!(段.first(), Some(Token::Kw("call")))
        && matches!(段.get(1), Some(Token::Kw("val")))
        && !matches!(段.get(3), Some(Token::Sym("(")))
    {
        return infer_named(&段[1..], locals, idx, names, block, types);
    }

    if matches!(top_op(段).0, 1 | 2 | 3) {
        return Some("bool".into());
    }

    if matches!(段.first(), Some(Token::Sym("["))) {
        let 内 = if 段.len() >= 2 && matches!(段.last(), Some(Token::Sym("]"))) {
            &段[1..段.len() - 1]
        } else {
            &段[1..]
        };
        if let Some(首) = split_commas(内).into_iter().next() {
            if let Some(t) = infer_named(&首, locals, idx, names, block, types)
                .or_else(|| infer(&首, locals, idx, names, block, types))
            {
                return Some(format!("list({})", t));
            }
        }
        return Some("list".into());
    }

    if let [Token::Kw("val"), rest @ (Token::Str(_) | Token::Ident(_)), ..] = 段 {
        
        if matches!(
            段.get(2),
            Some(Token::Sym("==")) | Some(Token::Sym("!=")) | Some(Token::Sym("<"))
                | Some(Token::Sym("<=")) | Some(Token::Sym(">")) | Some(Token::Sym(">="))
        ) {
            return Some("bool".to_string());
        }
        let n = match rest {
            Token::Str(n) | Token::Ident(n) => n.clone(),
            _ => unreachable!(),
        };
        let base = 查局部(locals, idx, names, block, &n).map(|l| l.ty.clone());
        
        
        if let Some(t) = &base {
            if 段.iter().any(|x| *x == Token::Sym("~")) {
                return Some(t.clone());
            }
            
            let mut cur = Some(t.clone());
            let mut k = 2usize;
            loop {
                if matches!(段.get(k), Some(Token::Sym("."))) {
                    let Some(Token::Ident(f) | Token::Str(f)) = 段.get(k + 1) else { break };
                    cur = match cur {
                        Some(b) => 字段型(types, &b, f),
                        None => None,
                    };
                    k += 2;
                    continue;
                }
                if matches!(段.get(k), Some(Token::Sym("["))) {
                    cur = match cur {
                        Some(b) if b == "str" => Some("str".to_string()),
                        Some(b) => elem_ty_str(&b),
                        None => None,
                    };
                    let mut 深 = 0i32;
                    while k < 段.len() {
                        match &段[k] {
                            Token::Sym("[") => 深 += 1,
                            Token::Sym("]") => {
                                深 -= 1;
                                if 深 == 0 {
                                    k += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        k += 1;
                    }
                    continue;
                }
                break;
            }
            return cur;
        }
        return base;
    }
    if let [Token::Ident(n)] = 段 {
        return 查局部(locals, idx, names, block, &n).map(|l| l.ty.clone());
    }
    

    if let Some(Token::Ident(n)) = 段.first() {
        if let Some(base) = 查局部(locals, idx, names, block, &n).map(|l| l.ty.clone()) {
            if 段.iter().any(|t| *t == Token::Sym("~")) {
                return Some(base);
            }
            if matches!(段.get(1), Some(Token::Sym("["))) {
                return Some(elem_ty_str(&base).unwrap_or_else(|| "int".into()));
            }
        }
    }
    None
}


fn paren_spans_all(段: &[Token]) -> bool {
    let mut depth = 0i32;
    for (i, t) in 段.iter().enumerate() {
        match t {
            Token::Sym("(") => depth += 1,
            Token::Sym(")") => {
                depth -= 1;
                if depth == 0 {
                    return i == 段.len() - 1;
                }
            }
            _ => {}
        }
    }
    false
}


fn 字段型(types: &TypeTable, 基: &str, 字段: &str) -> Option<String> {
    
    let plain = 基.strip_suffix('*').unwrap_or(基);
    let r = types
        .get(plain)
        .and_then(|(_, _, fs)| fs.iter().find(|(n, _, _)| n == 字段).map(|(_, t, _)| t.clone()));
    r
}

fn infer(
    段: &[Token],
    locals: &[LocalInfo],
    idx: &局部索引,
    names: &prion_types::Names,
    block: &str,
    types: &TypeTable,
) -> Option<String> {
    
    if let Some(Token::Str(n)) = 段.first() {
        if matches!(段.get(1), Some(Token::Sym("("))) && types.contains_key(n.as_ref()) {
            return Some(format!("{}*", n));
        }
    }
    
    if matches!(段.first(), Some(Token::Kw("call")))
        && matches!(段.get(1), Some(Token::Kw("val")))
        && !matches!(段.get(3), Some(Token::Sym("(")))
    {
        return infer(&段[1..], locals, idx, names, block, types);
    }
    
    if matches!(top_op(段).0, 1 | 2 | 3) {
        return Some("bool".to_string());
    }
    
    if let [Token::Kw("val"), rest @ (Token::Str(_) | Token::Ident(_)), ..] = 段 {
        
        if matches!(
            段.get(2),
            Some(Token::Sym("==")) | Some(Token::Sym("!=")) | Some(Token::Sym("<"))
                | Some(Token::Sym("<=")) | Some(Token::Sym(">")) | Some(Token::Sym(">="))
        ) {
            return Some("bool".to_string());
        }
        let n = match rest {
            Token::Str(n) | Token::Ident(n) => n.clone(),
            _ => unreachable!(),
        };
        if let Some(l) = 查局部(locals, idx, names, block, &n) {
            
            let mut cur: Option<String> = Some(l.ty.clone());
            let mut k = 2usize;
            let mut 走过 = false;
            loop {
                if matches!(段.get(k), Some(Token::Sym("."))) {
                    let Some(Token::Ident(f) | Token::Str(f)) = 段.get(k + 1) else { break };
                    走过 = true;
                    cur = match cur {
                        Some(t) => 字段型(types, &t, f),
                        None => None,
                    };
                    k += 2;
                    continue;
                }
                if matches!(段.get(k), Some(Token::Sym("["))) {
                    走过 = true;
                    cur = match cur {
                        Some(t) if t == "str" => Some("str".to_string()),
                        Some(t) => elem_ty_str(&t),
                        None => None,
                    };
                    let mut 深 = 0i32;
                    while k < 段.len() {
                        match &段[k] {
                            Token::Sym("[") => 深 += 1,
                            Token::Sym("]") => {
                                深 -= 1;
                                if 深 == 0 {
                                    k += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        k += 1;
                    }
                    continue;
                }
                break;
            }
            if 走过 && cur.is_some() {
                return cur;
            }
            let t = l.ty.as_ref();
            
            
            if is_container_ty(t) {
                return Some("list".to_string());
            }
            return Some(match t {
                "float" => "float",
                "bool" => "bool",
                "str" => "str",
                _ => "int",
            }
            .to_string());
        }
    }
    
    
    if let [Token::Kw("call"), Token::Kw("val"), Token::Str(n)] = 段 {
        if let Some(l) = 查局部(locals, idx, names, block, n) {
            return Some(
                match l.ty.as_ref() {
                    "float" => "float",
                    "bool" => "bool",
                    "str" => "str",
                    _ => "int",
                }
                .to_string(),
            );
        }
    }

    let head_name = match 段.first() {
        Some(Token::Ident(n)) => Some(n.to_string()),
        Some(Token::Kw(k)) => Some((*k).to_string()),
        _ => None,
    };
    if matches!(段.get(1), Some(Token::Sym("("))) && paren_spans_all(段) {
        if let Some(name) = head_name {
            match name.as_ref() {
                "int" | "code" | "len" | "char_len" | "index" | "rnd" | "str_to_c" => {
                    return Some("int".to_string())
                }
                "empty" | "has" => return Some("bool".to_string()),
                "map" => return Some("map".to_string()),
                
                "que" => return Some("que".to_string()),
                "stack" => return Some("stack".to_string()),
                "args" => return Some("list(str)".to_string()),
                "read_file" => return Some("str".to_string()),
                "keys" => {
                    
                    
                    let inner = &段[2..段.len() - 1];
                    let mut got = None;
                    if let [Token::Kw("call"), Token::Kw("val"), Token::Str(n)] = inner {
                        if let Some(l) = 查局部(locals, idx, names, block, n) {
                            got = Some(l.ty.to_string());
                        }
                    }
                    let mt = got.or_else(|| infer(inner, locals, idx, names, block, types));
                    if let Some(mt) = mt {
                        if let Some(rest) = mt.strip_prefix("map(") {
                            let k = rest.split(',').next().unwrap_or("").trim();
                            return Some(format!("list({})", k));
                        }
                    }
                    return Some("list(str)".to_string());
                },
                
                "vals" => return Some("list".to_string()),
                "str" | "concat" | "char" | "str_from_c" => {
                    return Some("str".to_string())
                }
                "slice" => return Some("list".to_string()),
                "float" => return Some("float".to_string()),
                "bool" => return Some("bool".to_string()),
                "list" => return Some("list".to_string()),
                _ => {}
            }
        }
    }
    
    
    if matches!(段.first(), Some(Token::Sym("["))) {
        return Some("list".to_string());
    }
    if matches!(段.first(), Some(Token::Sym("{"))) {
        return Some("map".to_string());
    }
    
    
    match top_op(段).0 {
        1 | 2 | 3 => return Some("bool".to_string()), 
        4..=7 => return Some("int".to_string()),      
        8 => {
            
            if top_has_str(段, locals, block) {
                return Some("str".to_string());
            }
            if top_has_float(段, locals, block) {
                return Some("float".to_string());
            }
            return Some("int".to_string());
        }
        9 => {
            return Some(if top_has_float(段, locals, block) { "float" } else { "int" }.to_string())
        }
        _ => {}
    }
    
    operand_ty(段, locals, idx, names, block).map(|s| s.to_string())
}


fn top_op(段: &[Token]) -> (u8, usize) {
    let mut depth = 0i32;
    let mut best = 0u8;
    let mut best_i = 0usize;
    let mut prev_end = false; 
    for (i, t) in 段.iter().enumerate() {
        match t {
            Token::Sym("(") | Token::Sym("[") => {
                depth += 1;
                prev_end = false;
                continue;
            }
            Token::Sym(")") | Token::Sym("]") => {
                depth -= 1;
                if depth == 0 {
                    prev_end = true;
                }
                continue;
            }
            _ => {}
        }
        if depth != 0 {
            continue;
        }
        let c = match t {
            Token::Kw("or") => 1,
            Token::Kw("and") => 2,
            Token::Sym("==") | Token::Sym("!=") | Token::Sym("<") | Token::Sym("<=")
            | Token::Sym(">") | Token::Sym(">=") => 3,
            Token::Sym("|") => 4,
            Token::Sym("^") => 5,
            Token::Sym("&") => 6,
            Token::Sym("<<") | Token::Sym(">>") => 7,
            Token::Sym("+") => 8,
            Token::Sym("-") if prev_end => 8,
            Token::Sym("*") | Token::Sym("/") | Token::Sym("%") => 9,
            _ => 0,
        };
        if c > 0 && (best == 0 || c < best) {
            best = c;
            best_i = i;
        }
        prev_end = matches!(
            t,
            Token::Ident(_)
                | Token::Int(_)
                | Token::Float(_)
                | Token::Str(_)
                | Token::Kw("re")
                | Token::Kw("fa")
                | Token::Kw("null")
        );
    }
    (best, best_i)
}


fn top_has_str(段: &[Token], locals: &[LocalInfo], block: &str) -> bool {
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < 段.len() {
        match &段[i] {
            Token::Sym("(") | Token::Sym("[") => depth += 1,
            Token::Sym(")") | Token::Sym("]") => depth -= 1,
            
            
            Token::Kw("call")
                if depth == 0
                    && matches!(
                        段.get(i + 1),
                        Some(Token::Kw("val")) | Some(Token::Kw("fn")) | Some(Token::Kw("type"))
                    )
                    && matches!(段.get(i + 2), Some(Token::Str(_))) =>
            {
                i += 3;
                continue;
            }
            Token::Str(_) if depth == 0 => return true,
            Token::Ident(n) if depth == 0 => {
                if locals
                    .iter()
                    .any(|l| l.block == block && *l.name == **n && l.ty == "str")
                {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}


fn top_has_float(段: &[Token], locals: &[LocalInfo], block: &str) -> bool {
    let mut depth = 0i32;
    for t in 段 {
        match t {
            Token::Sym("(") | Token::Sym("[") => depth += 1,
            Token::Sym(")") | Token::Sym("]") => depth -= 1,
            Token::Float(_) if depth == 0 => return true,
            Token::Ident(n) if depth == 0 => {
                if locals
                    .iter()
                    .any(|l| l.block == block && *l.name == **n && l.ty == "float")
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}


fn operand_ty(段: &[Token], locals: &[LocalInfo], idx: &局部索引, names: &prion_types::Names, block: &str) -> Option<&'static str> {
    match 段.first() {
        Some(Token::Int(_)) => Some("int"),
        Some(Token::Float(_)) => Some("float"),
        Some(Token::Str(_)) => Some("str"),
        Some(Token::Kw("re")) | Some(Token::Kw("fa")) => Some("bool"),
        Some(Token::Sym("-")) => match 段.get(1) {
            
            Some(Token::Float(_)) => Some("float"),
            _ => Some("int"),
        }, 
        Some(Token::Ident(n)) => {
            if matches!(段.get(1), Some(Token::Sym("("))) {
                None 
            } else {
                查局部(locals, idx, names, block, n)
                    .filter(|l| l.ty != UNKNOWN)
                    .map(|l| match l.ty.as_ref() {
                        "float" => "float",
                        "bool" => "bool",
                        "str" => "str",
                        _ => "int",
                    })
            }
        }
        _ => None,
    }
}


fn is_known_type(cx: &Ctx, block: &str, t: &str) -> bool {
    let base = t.trim_end_matches('*');
    let base = base.split('(').next().unwrap_or(base);
    if let Some(inner) = base.strip_prefix('?') {
        return is_known_type(cx, block, inner);
    }
    const BUILTIN: &[&str] = &["int", "float", "bool", "str", "list", "map", "que", "stack", "token"];
    BUILTIN.contains(&base)
        || WIDTHS.contains(&base)
        || cx.types.contains_key(base)
        || cx.泛型.contains_key(base)
        || cx
            .类型参数作用域
            .get(block)
            .map(|s| s.contains(base))
            .unwrap_or(false)
}

fn pass2_types(ast: &AstProgram, cx: &mut Ctx) {
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        types_block(b, cx);
    }
}

fn types_block(b: &AstBlock, cx: &mut Ctx) {
    for s in &b.stmts {
        match s {
            AstStmt::Line { line, col, len, exprs, kind, .. } => {
                let 规范 = prion_parser::建val规范(kind.tokens());
                let 段: &[Token] = &规范;
                
                let 行式 = exprs.first();
                if matches!(*kind, prion_parser::LineKind::AddVal(_)) {
                    let name = match 段.get(2) {
                        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.clone(),
                        _ => continue,
                    };
                    
                    
                    let decl = match container_ty_at(段, 3) {
                        Some(t) => t,
                        None => match 段.get(3) {
                            
                            Some(Token::Kw(t)) if *t == "re" || *t == "fa" => {
                                cx.err(
                                    位置{ 行: *line, 列: *col, 长: *len },
                                    CheckCode::E201,
                                    &DiagParts::full(
                                        format!("第 {} 行", line),
                                        "类型名",
                                        format!("没有类型「{}」", t),
                                        "re / fa 是真假值的字面量，不是类型名",
                                        "类型名写 bool",
                                    ),
                                    Tone::Fix,
                                );
                                return;
                            }
                            Some(Token::Kw(t)) => {
                                if matches!(段.get(4), Some(Token::Sym("("))) {
                                    parse_type_ref(段, 3).map(|(ty, _)| ty).unwrap_or_else(|_| (*t).to_string())
                                } else {
                                    (*t).to_string()
                                }
                            }
                            
                            Some(Token::Ident(t)) => {
                                
                                let t = if matches!(段.get(4), Some(Token::Sym("*"))) {
                                    format!("{}*", t)
                                } else {
                                    t.to_string()
                                };
                                if !is_known_type(cx, &b.name, &t) {
                                    cx.err(
                                        位置{ 行: *line, 列: *col, 长: *len },
                                        CheckCode::E201,
                                        &DiagParts::full(
                                            format!("第 {} 行", line),
                                            "类型名",
                                            format!("没有类型「{}」", t),
                                            "",
                                            "内置类型：int / float / bool / str / list(T) / map(K,V) / que(T) / stack(T) \
                                             / int_8…int_64 / uint_8…uint_64；自定义类型要先 `add type \"名\" (…)`。\
                                             旧名 int8 / uint8 / size 已废止（2026-10-08 词名重构）。",
                                        )
                                        .预期("类型名必须是内置类型或已 add type 的类型")
                                        .详解(
                                            "类型名做**存在性校验**：不在内置表、又没有 `add type` 声明过 ⇒ 报错。\
                                             这样废止名与被拼错的类型名都会当场暴露，不需要维护废止名单。",
                                        ),
                                        Tone::Fix,
                                    );
                                    continue;
                                }
                                t.clone()
                            }
                            _ => continue,
                        },
                    };
                    
                    if let Some(rest) = decl.strip_prefix("map(") {
                        let k = rest.split(',').next().unwrap_or("").trim();
                        if k != "str" && k != "int" {
                            cx.err(
                                位置{ 行: *line, 列: *col, 长: *len },
                                CheckCode::E201,
                                &DiagParts::full(
                                    format!("第 {} 行", line),
                                    "map 键型",
                                    format!("map 的键型「{}」不支持", k),
                                    "map 的键只支持 str 或 int",
                                    "把键型改成 str 或 int",
                                ),
                                Tone::Fix,
                            );
                            continue;
                        }
                    }
                    if let Some(eq) = 段.iter().position(|t| *t == Token::Sym("=")) {
                        let rhs = &段[eq + 1..];
                        let got: Option<String> = if rhs.len() == 2 && matches!(rhs[0], Token::Kw("fn")) {
                            let f = match &rhs[1] {
                                Token::Str(f) | Token::Ident(f) => f.clone(),
                                _ => String::new().into(),
                            };
                            cx.键(&b.name, &f).and_then(|k| cx.fns.get(&k)).map(|s| {
                                let ps: Vec<String> = s.params.clone();
                                format!("fn({}) {}", ps.join(", "), s.ret)
                            })
                        } else {
                            行式.and_then(|e| infer_ast::推断带期望(e, cx, &b.name, Some(decl.as_ref())))
                        };
                        if let Some(got) = got {
                            if let Some((op, bound)) = 解析精化条件(&decl) {
                                let val = 段.iter().position(|t| *t == Token::Sym("="))
                                    .and_then(|eq| 提取整数字面量(&段[eq + 1..]));
                                if let Some(v) = val {
                                    let 满足 = match op.as_str() {
                                        ">" => v > bound,
                                        ">=" => v >= bound,
                                        "<" => v < bound,
                                        "<=" => v <= bound,
                                        _ => true,
                                    };
                                    if !满足 {
                                        cx.err(
                                            位置{ 行: *line, 列: *col, 长: *len },
                                            CheckCode::E201,
                                            &DiagParts::full(
                                                format!("第 {} 行", line),
                                                "精化条件",
                                                format!("「{}」的值 {} 不满足类型 {} 的条件", name, v, decl),
                                                "",
                                                format!("把值改成满足 {} 的数", decl),
                                            ),
                                            Tone::Fix,
                                        );
                                    }
                                }
                            }
                            if !ty_compatible(cx, &decl, &got) {
                                cx.err(
                                    位置{ 行: *line, 列: *col, 长: *len },
                                    CheckCode::E202,
                                    &DiagParts::full(
                                        format!("第 {} 行", line),
                                        "赋值",
                                        format!("「{}」声明是 {}，右边是 {}", name, decl, got),
                                        "",
                                        format!("改成 {} 的值，或把声明改成 {}", decl, got),
                                    )
                                    .预期(format!("右边的值应当是 {} 型（左边声明说了算）", decl))
                .详解("Prion 不悄悄转换类型：int 不会自动变 str。要换类型请显式写 int(值) / str(值) / float(值) / bool(值) 转换；若是声明写错了，就把左边类型改成右边的型。"),
                                    Tone::Fix,
                                );
                            }
                        }
                    }
                }
                
                if matches!(段.first(), Some(Token::Kw("val")))
                    && !matches!(段.get(2), Some(Token::Sym("[")) | Some(Token::Sym(".")))
                {
                    if let (Some(Token::Str(n)) | Some(Token::Ident(n)), Some(_eq)) = (
                        段.get(1),
                        段.iter().position(|t| *t == Token::Sym("=")),
                    ) {
                        let decl = cx
                            .find_local(&b.name, n)
                            .map(|l| l.ty.clone())
                            .unwrap_or_else(|| UNKNOWN.to_string());
                        if decl != UNKNOWN {
                            
                            let got: Option<String> = 行式.and_then(|e| infer_ast::推断带期望(e, cx, &b.name, Some(decl.as_ref())));
                            if let Some(got) = got {
                                if !ty_compatible(cx, &decl, &got) {
                                    cx.err(
                                        位置{ 行: *line, 列: *col, 长: *len },
                                        CheckCode::E202,
                                        &DiagParts::full(
                                        format!("第 {} 行", line),
                                        "改值",
                                        format!("「{}」是 {}，右边是 {}", n, decl, got),
                                        "",
                                        format!("改成 {} 的值", decl),
                                        )
                                        .预期(format!("新值应当是 {} 型", decl))
                .详解("改值必须和它当初声明的类型一致——同一条 val 不改类型，要另一种型请用转换函数。"),
                                        Tone::Fix,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            AstStmt::Fn { line, col, len, head, body, .. } => {
                
                let fname = stmt_fn_name(head).unwrap_or_default();
                if !fname.is_empty() {
                    let ret = cx
                        .fns
                        .get(&(cx.names.intern(&b.name), cx.names.intern(&fname)))
                        .or_else(|| {
                            let k = (cx.names.intern(原块(&b.name)), cx.names.intern(&fname));
                            cx.fns.get(&k)
                        })
                        .map(|s| s.ret.clone())
                        .unwrap_or_default();
                    let pb = fn_scope_block(b, &fname, body);
                    match body.last() {
                        Some(AstStmt::Line { line: gl, col, len, exprs: gexprs, kind, .. }) if matches!(*kind, prion_parser::LineKind::Give(_)) => {
                            let _段 = kind.tokens();
                            
                            let mut 早: Vec<(usize, TokStream, Vec<Expr>)> = Vec::new();
                            找give(&body[..body.len() - 1], &mut 早);
                            for (el, 段, gexprs) in 早 {
                                let _expr = &段[1..];
                                let got = gexprs.first().and_then(|e| infer_ast::推断(e, cx, &pb.name));
                                if let Some(got) = got {
                                    if !ret.is_empty() && !ty_compatible(cx, &ret, &got) {
                                        cx.err(
                                            位置{ 行: el, 列: *col, 长: *len },
                                            CheckCode::E202,
                                            &DiagParts::full(
                                                format!("第 {} 行", el),
                                                "写 give",
                                                format!("fn「{}」声明返回 {}，中途 give 的是 {}", fname, ret, got),
                                                "",
                                                format!("give 一个 {} 的值", ret),
                                            )
                                            .预期("中途 give 与末尾 give 同型")
                                            .详解("中途 give 即立即返回；所有 give 都要和声明的返回类型一致。"),
                                            Tone::Fix,
                                        );
                                    }
                                }
                            }
                            let got = gexprs.first().and_then(|e| infer_ast::推断(e, cx, &pb.name));
                            if let Some(got) = got {
                                if !ret.is_empty() && !ty_compatible(cx, &ret, &got) {
                                    cx.err(
                                        位置{ 行: *gl, 列: *col, 长: *len },
                                        CheckCode::E202,
                                        &DiagParts::full(
                                            format!("第 {} 行", gl),
                                            "写 give",
                                            format!("fn「{}」声明返回 {}，give 的是 {}", fname, ret, got),
                                            "",
                                            format!("give 一个 {} 的值", ret),
                                        )
                                        .预期(format!("give 的表达式应当是 {} 型", ret)),
                                        Tone::Fix,
                                    );
                                }
                            }
                        }
                        _ => {
                            cx.err(
                                位置{ 行: *line, 列: *col, 长: *len },
                                CheckCode::E202,
                                &DiagParts::full(
                                    format!("第 {} 行", line),
                                    "写语句体 fn",
                                    format!("fn「{}」的最后一条不是 give", fname),
                                    "语句体必须有返回值",
                                    "在函数体末尾写 `give 表达式`",
                                )
                                .预期("语句体 fn 末尾要写 give 表达式")
                                .详解("fn 是算值的：语句体允许中间步骤，但最后要用 give 把值交回调用者。"),
                                Tone::Fix,
                            );
                        }
                    }
                    types_block(&pb, cx);
                }
            }
            AstStmt::Struct { line, col, len, head, body, .. } => {
                
                let kind = head.first().and_then(|t| match t {
                    Token::Kw(k) => Some(*k),
                    _ => None,
                });
                match kind {
                    Some("if") | Some("while") => {
                        let cond = if head.len() >= 2 { &head[1..head.len() - 1] } else { &[] };
                        if let Some(got) = infer(cond, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types) {
                            if got != "bool" {
                                cx.err(
                                    位置{ 行: *line, 列: *col, 长: *len },
                                    CheckCode::E202,
                                    &DiagParts::full(
                                        format!("第 {} 行", line),
                                        "写条件",
                                        format!("{} 的条件不是真假值（是 {}）", kind.unwrap(), got),
                                        "",
                                        "写成比较式（如 x == 1），或直接用 re / fa",
                                    ),
                                    Tone::Fix,
                                );
                            }
                        }
                    }
                    Some("case") => {
                        let inner = if head.len() >= 2 { &head[1..head.len() - 1] } else { &[][..] };
                        let mut d = 0i32;
                        let mut gs: Option<usize> = None;
                        for (i, tk) in inner.iter().enumerate() {
                            match tk {
                                Token::Sym("(") | Token::Sym("[") | Token::Sym("{") => d += 1,
                                Token::Sym(")") | Token::Sym("]") | Token::Sym("}") => d -= 1,
                                Token::Kw("if") if d == 0 => {
                                    gs = Some(i + 1);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        if let Some(gs) = gs {
                            let cond = &inner[gs..];
                            if let Some(got) =
                                infer(cond, &cx.locals, &cx.local_idx, &cx.names, &b.name, &cx.types)
                            {
                                if got != "bool" && got != UNKNOWN {
                                    cx.err(
                                        位置{ 行: *line, 列: *col, 长: *len },
                                        CheckCode::E202,
                                        &DiagParts::full(
                                            format!("第 {} 行", line),
                                            "写守卫",
                                            format!("case 守卫的条件不是真假值（是 {}）", got),
                                            "",
                                            "写成比较式（如 x > 10），或直接用 re / fa",
                                        ),
                                        Tone::Fix,
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                }
                for s2 in body {
                    if let AstStmt::Line { line, col: _, len: _, kind, .. } = s2 {
                        let 段 = kind.tokens();
                        
                        let pseudo = AstBlock {
            name: b.name.clone(),
            line: *line,
            col: 0,
            len: 0,
            stmts: vec![AstStmt::Line { line: *line, col: 0, len: 0, kind: prion_parser::line_kind(段), exprs: Vec::new() }],
            attrs: Vec::new(),
            asm_lines: Vec::new(),
            module: b.module.clone(),
            file: b.file.clone(),
        };
                        types_block(&pseudo, cx);
                    } else if let AstStmt::Struct { line, col, len, kind, head, body } = s2 {
                        let pseudo = AstBlock {
                            name: b.name.clone(),
                            line: *line,
                            col: *col,
                            len: *len,
                            stmts: vec![AstStmt::Struct { line: *line, col: 0, len: 0, kind: *kind, head: head.clone(), body: body.clone() }],
                            attrs: Vec::new(),
                            asm_lines: Vec::new(),
                            module: b.module.clone(),
                            file: b.file.clone(),
                        };
                        types_block(&pseudo, cx);
                    }
                }
            }
        }
    }
}







fn pass_sle_calls(ast: &AstProgram, cx: &mut Ctx) {
    
    let sle: std::collections::BTreeSet<String> = ast
        .blocks
        .iter()
        .filter(|b| b.attrs.iter().any(|a| a == "sle"))
        .map(|b| b.name.clone())
        .collect();
    if sle.is_empty() {
        return;
    }
    
    let mut ran: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    
    let mut 用处: Vec<(String, String, String, usize, usize, usize)> = Vec::new();
    for b in &ast.blocks {
        for s in &b.stmts {
            if let AstStmt::Line { line, col, len, kind, .. } = s {
                let 段 = kind.tokens();
                let mut i = 0usize;
                while i < 段.len() {
                    if matches!(段.get(i), Some(Token::Kw("call")))
                        && matches!(段.get(i + 1), Some(Token::HashColon))
                    {
                        let blk = match 段.get(i + 2) {
                            Some(Token::Ident(n)) => n.clone(),
                            Some(Token::Kw(k)) => (*k).to_string().into(),
                            _ => {
                                i += 1;
                                continue;
                            }
                        };
                        match 段.get(i + 3) {
                            
                            None => {
                                ran.insert(blk.to_string());
                            }
                            
                            Some(Token::Kw("val")) if sle.contains(blk.as_ref()) => {
                                用处.push((blk.to_string(), b.name.clone(), b.file.clone(), *line, *col, *len));
                            }
                            _ => {}
                        }
                        i += 3;
                        continue;
                    }
                    i += 1;
                }
            }
        }
    }
    
    let mut 已报: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (blk, _from, file, line, col, len) in 用处 {
        if ran.contains(&blk) || !已报.insert(blk.clone()) {
            continue;
        }
        cx.cur_file = file;
        cx.err(
            位置 { 行: line, 列: col, 长: len },
            CheckCode::E203,
            &DiagParts::full(
                format!("第 {} 行", line),
                "sle 块时序",
                format!("sle 块「{}」没人显式跑过，它的 val 还不存在", blk),
                "sle 块不自动跑；取用它的 val 前，程序里要有一句 `call #:块名` 把它跑一遍",
                "先写一句 `call #:块名`，再取用它的 val",
            ),
            Tone::Fix,
        );
    }
}

#[derive(Default)]
struct 并发表 {
    声明: std::collections::BTreeMap<String, (bool, bool, usize, usize, usize)>,
    值表: std::collections::BTreeMap<String, (usize, usize, usize)>,
    已移交: std::collections::BTreeMap<String, usize>,
    token表: std::collections::BTreeMap<String, (usize, usize, usize)>,
    已消费: std::collections::BTreeSet<String>,
    自结束: std::collections::BTreeSet<String>,
    容器表: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    已捕获: std::collections::BTreeMap<String, usize>,
}


fn 扫沿用名(段: &[Token], out: &mut Vec<String>) {
    for i in 0..段.len() {
        if matches!(段[i], Token::Kw("call")) && matches!(段.get(i + 1), Some(Token::Kw("val"))) {
            if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 段.get(i + 2) {
                out.push(n.to_string());
            }
        }
    }
}


fn 调用实参(段: &[Token], 名位: usize) -> Option<Vec<Vec<Token>>> {
    if !matches!(段.get(名位 + 1), Some(Token::Sym("("))) {
        return None;
    }
    let mut 深 = 1i32;
    let mut 组: Vec<Vec<Token>> = vec![Vec::new()];
    let mut j = 名位 + 2;
    while j < 段.len() {
        match &段[j] {
            Token::Sym("(") | Token::Sym("[") | Token::Sym("{") => 深 += 1,
            Token::Sym(")") | Token::Sym("]") | Token::Sym("}") => {
                深 -= 1;
                if 深 == 0 {
                    break;
                }
            }
            Token::Sym(",") if 深 == 1 => {
                组.push(Vec::new());
                j += 1;
                continue;
            }
            _ => {}
        }
        组.last_mut()?.push(段[j].clone());
        j += 1;
    }
    if 深 != 0 {
        return None;
    }
    Some(组)
}


fn 单名实参(组: &[Token]) -> Option<String> {
    if let [Token::Kw("call"), Token::Kw("val"), Token::Str(n)] = 组 {
        return Some(n.to_string());
    }
    None
}


fn 收体内建名(stmts: &[AstStmt], out: &mut std::collections::BTreeSet<String>) {
    for s in stmts {
        match s {
            AstStmt::Line { kind, .. } => {
                let 段 = kind.tokens();
                if is_add_val(段) {
                    let 规范 = prion_parser::建val规范(段);
                    if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 规范.get(2) {
                        out.insert(n.to_string());
                    }
                }
            }
            AstStmt::Struct { body, .. } | AstStmt::Fn { body, .. } => 收体内建名(body, out),
        }
    }
}


fn 扫体引用(stmts: &[AstStmt], out: &mut Vec<String>) {
    for s in stmts {
        match s {
            AstStmt::Line { kind, .. } => 扫沿用名(kind.tokens(), out),
            AstStmt::Struct { head, body, .. } => {
                扫沿用名(head, out);
                扫体引用(body, out);
            }
            AstStmt::Fn { body, .. } => 扫体引用(body, out),
        }
    }
}


fn 是自结束体(body: &[AstStmt]) -> bool {
    match body.last() {
        Some(AstStmt::Line { kind, .. }) => {
            let 段 = kind.tokens();
            段.len() == 1 && 段.first().map(是裸join词).unwrap_or(false)
        }
        _ => false,
    }
}


fn check_conc(ast: &AstProgram, cx: &mut Ctx) {
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        let mut st = 并发表::default();
        conc_stmts(&b.stmts, b, cx, &mut st);
        let token表 = std::mem::take(&mut st.token表);
        for (n, (line, col, len)) in token表 {
            if !st.已消费.contains(&n) && !st.自结束.contains(&n) {
                if let Some(c) = st
                    .容器表
                    .iter()
                    .find(|(_, ts)| ts.contains(&n))
                    .map(|(c, _)| c.clone())
                {
                    cx.err(
                        位置 { 行: line, 列: col, 长: len },
                        CheckCode::E201,
                        &DiagParts::full(
                            format!("第 {} 行", line),
                            "任务 token",
                            format!("token「{}」装进了容器「{}」，但这个容器没在块内整体 join", n, c),
                            "装进容器只是暂存，任务还等着收",
                            format!("在本块收尾写 `join(call val \"{}\")`，一次把容器里的任务全等完", c),
                        )
                        .预期("装过 token 的容器必须在块内整体 join 一次"),
                        Tone::Fix,
                    );
                    continue;
                }
                cx.err(
                    位置 { 行: line, 列: col, 长: len },
                    CheckCode::E201,
                    &DiagParts::full(
                        format!("第 {} 行", line),
                        "任务 token",
                        format!("token「{}」没被 join——任务不许逃出块", n),
                        "",
                        format!("写 `join(call val \"{}\")` 取回结果；不要返回值时，任务体末行写单独一行 `join` 表示自结束", n),
                    )
                    .预期("每个 spawn 出来的 token 都要显式 join，或任务体末行写 join 自结束"),
                    Tone::Fix,
                );
            }
        }
    }
}


fn conc_stmts(stmts: &[AstStmt], b: &AstBlock, cx: &mut Ctx, st: &mut 并发表) {
    for s in stmts {
        match s {
            AstStmt::Line { line, col, len, kind, .. } => {
                let 段 = kind.tokens();
                let 处 = 位置 { 行: *line, 列: *col, 长: *len };
                if is_add_val(段) {
                    let 规范 = prion_parser::建val规范(段);
                    let name = match 规范.get(2) {
                        Some(Token::Str(n)) | Some(Token::Ident(n)) => Some(n.to_string()),
                        _ => None,
                    };
                    let (share, send) = prion_parser::建val修饰(段);
                    if let Some(n) = name {
                        st.值表.insert(n.clone(), (*line, *col, *len));
                        if share && send {
                            cx.err(
                                处,
                                CheckCode::E201,
                                &DiagParts::full(
                                    format!("第 {} 行", line),
                                    "并发标记",
                                    format!("val「{}」不能既是 share 又标 (send)", n),
                                    "一个数据要么共享、要么移交，没有第三种",
                                    "去掉其中一个：多任务只读用 `add share val`，交给一个任务用 `add val(send)`",
                                )
                                .预期("share 与 (send) 互斥"),
                                Tone::Fix,
                            );
                        }
                        if share || send {
                            st.声明.insert(n, (share, send, *line, *col, *len));
                        }
                    }
                }
                for i in 0..段.len() {
                    let 是join = matches!(段.get(i), Some(Token::Ident(x)) if x.as_ref() == "join")
                        || matches!(段.get(i), Some(Token::Kw("join")));
                    if 是join && matches!(段.get(i + 1), Some(Token::Sym("("))) {
                        if let Some(组) = 调用实参(&段, i) {
                            if 组.len() == 1 {
                                if let Some(c) = 单名实参(&组[0]) {
                                    if let Some(ts) = st.容器表.get(&c).cloned() {
                                        for t in ts {
                                            st.已消费.insert(t);
                                        }
                                    }
                                }
                            }
                        }
                        let mut j = i + 2;
                        while j + 1 < 段.len() {
                            if matches!(段[j], Token::Kw("call")) && matches!(段[j + 1], Token::Kw("val")) {
                                if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 段.get(j + 2) {
                                    st.已消费.insert(n.to_string());
                                }
                            }
                            j += 1;
                        }
                    }
                }
                for i in 0..段.len() {
                    let 是push = matches!(段.get(i), Some(Token::Ident(x)) if x.as_ref() == "push")
                        || matches!(段.get(i), Some(Token::Kw("push")));
                    if 是push && matches!(段.get(i + 1), Some(Token::Sym("("))) {
                        if let Some(组) = 调用实参(&段, i) {
                            if 组.len() >= 2 {
                                if let (Some(c), Some(t)) = (单名实参(&组[0]), 单名实参(&组[1])) {
                                    if st.token表.contains_key(&t) || st.已消费.contains(&t) || st.自结束.contains(&t) {
                                        st.容器表.entry(c).or_default().insert(t);
                                    }
                                }
                            }
                        }
                    }
                }
                if matches!(段.first(), Some(Token::Kw("val"))) {
                    if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 段.get(1) {
                        if st.声明.get(n.as_ref()).map(|d| d.0).unwrap_or(false) {
                            cx.err(
                                处,
                                CheckCode::E201,
                                &DiagParts::full(
                                    format!("第 {} 行", line),
                                    "写共享值",
                                    format!("「{}」是 share 值——多任务只读，不许写", n),
                                    "共享写会让任务读到半截状态",
                                    "要可变共享请用库层 Mutex；share 只给读",
                                )
                                .预期("share 值只读"),
                                Tone::Fix,
                            );
                        }
                    }
                }
                for i in 0..段.len() {
                    if matches!(段[i], Token::Kw("call")) && matches!(段.get(i + 1), Some(Token::Kw("val"))) {
                        if let Some(Token::Str(n)) | Some(Token::Ident(n)) = 段.get(i + 2) {
                            if st.已移交.contains_key(n.as_ref()) {
                                cx.err(
                                    处,
                                    CheckCode::E201,
                                    &DiagParts::full(
                                        format!("第 {} 行", line),
                                        "读已移交的值",
                                        format!("「{}」已经移交给任务——原件失效，不能再读", n),
                                        "移交后所有权归任务",
                                        "要用就给任务传一个副本，或改用 add share val 只读共享",
                                    )
                                    .预期("移交后原件失效"),
                                    Tone::Fix,
                                );
                            }
                        }
                    }
                }
            }
            AstStmt::Struct { line, col, len, head, body, kind, .. } => {
                if *kind == prion_parser::StructKind::Spawn {
                    let tokname = match head.get(2) {
                        Some(Token::Str(n)) | Some(Token::Ident(n)) => n.to_string(),
                        _ => continue,
                    };
                    let 自结束 = 是自结束体(body);
                    let 移交表: Vec<String> = st
                        .声明
                        .iter()
                        .filter(|(_, d)| d.1)
                        .map(|(n, _)| n.clone())
                        .collect();
                    for n in 移交表 {
                        st.已移交.entry(n).or_insert(*line);
                    }
                    let mut 内名 = std::collections::BTreeSet::new();
                    收体内建名(body, &mut 内名);
                    let mut 引用 = Vec::new();
                    扫体引用(body, &mut 引用);
                    let mut 报过 = std::collections::BTreeSet::new();
                    for n in 引用 {
                        if 内名.contains(&n) || !报过.insert(n.clone()) {
                            continue;
                        }
                        let 标记 = st.声明.get(&n).map(|d| (d.0, d.1));
                        match 标记 {
                            Some((_share, send)) => {
                                if 自结束 {
                                    cx.err(
                                        位置 { 行: *line, 列: *col, 长: *len },
                                        CheckCode::E201,
                                        &DiagParts::full(
                                            format!("第 {} 行", line),
                                            "自结束任务",
                                            format!("自结束的任务不能引用外层 val「{}」", n),
                                            "父块可能先退出，外层值届时已释放",
                                            "去掉末行 `join`（改为 `give` + 外层 join），或让任务只用体内自己的值",
                                        )
                                        .预期("自结束任务不捕获外层值"),
                                        Tone::Fix,
                                    );
                                }
                                if send {
                                    let 类型 = cx
                                        .find_local(&b.name, &n)
                                        .map(|l| l.ty.clone())
                                        .unwrap_or_default();
                                    if 是非复制类型(&类型) {
                                        if let Some(首行) = st.已捕获.get(&n).copied() {
                                            cx.err(
                                                位置 { 行: *line, 列: *col, 长: *len },
                                                CheckCode::E201,
                                                &DiagParts::full(
                                                    format!("第 {} 行", line),
                                                    "重复移交",
                                                    format!("「{}」已经移交给第 {} 行的任务——一个值只能搬一次", n, 首行),
                                                    "移交后所有权归第一个任务，第二个任务拿到的是同一个指针",
                                                    "要多个任务读同一个值，改用 `add share val`；要各写各的，给每个任务各建一份",
                                                )
                                                .预期("非复制的值只能移交一次"),
                                                Tone::Fix,
                                            );
                                        } else {
                                            st.已捕获.insert(n.clone(), *line);
                                        }
                                    }
                                }
                            }
                            None => {
                                if st.值表.contains_key(&n) {
                                    let 提示 = if 自结束 {
                                        "自结束任务不能引用外层 val"
                                    } else {
                                        "任务体引用外层 val——先标 add share val（只读）或 add val(send)（移交）"
                                    };
                                    cx.err(
                                        位置 { 行: *line, 列: *col, 长: *len },
                                        CheckCode::E201,
                                        &DiagParts::full(
                                            format!("第 {} 行", line),
                                            "任务引用外层值",
                                            format!("任务体用了外层 val「{}」，但没标并发标记", n),
                                            "没标 share 也没标 (send)，编译器推不出跨线程是否安全",
                                            提示,
                                        ),
                                        Tone::Fix,
                                    );
                                }
                            }
                        }
                    }
                    st.token表.insert(tokname.clone(), (*line, *col, *len));
                    if 自结束 {
                        st.自结束.insert(tokname);
                    }
                } else {
                    conc_stmts(body, b, cx, st);
                }
            }
            AstStmt::Fn { .. } => {}
        }
    }
}


fn pass3_memory(ast: &AstProgram, cx: &mut Ctx) {
    for b in &ast.blocks {
        cx.cur_file = b.file.clone();
        let unsafe_block = block_is_unsafe(b, ast);
        for l in cx.locals.iter_mut() {
            if l.block == b.name {
                l.class = if unsafe_block { StorageClass::Unsafe } else { StorageClass::Tmp };
            }
        }
        
        let _ = b;
    }
}
