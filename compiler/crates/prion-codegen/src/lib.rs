






use prion_ir::{
    BinOp, Block, Exit, Function, Local, LocalId, Place, Proj, Program, Rvalue, Stmt, Ty, Value,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};


#[derive(Debug, Clone, PartialEq)]
pub struct CodegenError {
    pub message: String,

    pub code: String,
    
    pub line: usize,
}


type TypeMap = BTreeMap<String, (String, Vec<(String, Ty)>)>;

thread_local! {
    
    static DEBUG_LINES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    
    static RELEASE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    
    static SRC_FILE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    
    static SRC_LINES: std::cell::RefCell<BTreeMap<String, Vec<String>>> =
        const { std::cell::RefCell::new(BTreeMap::new()) };
    
    static OWNED: std::cell::RefCell<BTreeSet<u32>> =
        const { std::cell::RefCell::new(BTreeSet::new()) };
    
    static 值入列: std::cell::RefCell<BTreeSet<u32>> =
        const { std::cell::RefCell::new(BTreeSet::new()) };
}


fn c_ty_in(t: &Ty, types: &TypeMap) -> String {
    let n = t.0.as_str();
    if let Some(base) = n.strip_suffix('*') {
        if let Some((c, _)) = types.get(base) {
            return format!("{}*", c); 
        }
    }
    if let Some((c, _)) = types.get(n) {
        return c.clone(); 
    }
    match n {
        "int" => "long long".to_string(),
        
        "int_8" => "signed char".to_string(),
        "uint_8" => "unsigned char".to_string(),
        "int_16" => "short".to_string(),
        "uint_16" => "unsigned short".to_string(),
        "int_32" => "int".to_string(),
        "uint_32" => "unsigned int".to_string(),
        "int_64" => "long long".to_string(),
        "uint_64" => "unsigned long long".to_string(),
        "float" => "double".to_string(),
        "bool" => "long long".to_string(),
        "str" => "const char*".to_string(),
        "ctx" => "long long*".to_string(),
        
        n if n.ends_with('*') => {
            format!("{}*", c_ty_in(&Ty(n[..n.len() - 1].to_string()), types))
        }
        
        n if n.starts_with("list") || n.starts_with("que") || n.starts_with("stack") => {
            "PrionList*".to_string()
        }
        n if n.starts_with("map") => "PrionMap*".to_string(),
        _ => "long long".to_string(),
    }
}


const PRION_LIST_RT: &str = r#"
\
\

\

     typedef struct { long long len; long long cap; long long esz; long long sgn; char* d; } PrionList;
char* prion_list_slot(PrionList* l, long long i);
long long prion_slot_get(const char* p, long long esz, long long sgn);
void prion_slot_put(char* p, long long esz, long long v);
PrionList* prion_list_new(long long esz, long long sgn);
void prion_list_push_from(PrionList* l, const char* src);
long long prion_list_push(PrionList* l, long long v);
void prion_list_reserve(PrionList* l, long long n);
long long prion_list_len(PrionList* l);
long long prion_list_at(PrionList* l, long long i);
long long prion_list_push_s(PrionList* l, const PrionStr* v);
void prion_list_die(long long i, long long n);
\
     static long long prion_f2s(double d) { long long v; memcpy(&v, &d, 8); return v; }
     static double prion_s2f(long long v) { double d; memcpy(&d, &v, 8); return d; }
\
\
\
\
\
     static PrionStr prion_desc_at(const char* p) { PrionStr s; memcpy(&s, p, sizeof(PrionStr)); return s; }
\
\
\
\
\
\
     /* 聚合元素：整块字节拷 */











\
\
\
\
\
\
\
\
void prion_list_print(PrionList* l);
void prion_list_print_tag(PrionList* l, const char* tag);
void prion_list_print_f(PrionList* l);
void prion_list_print_tag_f(PrionList* l, const char* tag);
void prion_list_print_s(PrionList* l);
void prion_list_print_tag_s(PrionList* l, const char* tag);
void prion_list_sort(PrionList* l);
void prion_list_sort_s(PrionList* l);
long long prion_list_contains_s(PrionList* l, const PrionStr* v);
long long prion_list_index_s(PrionList* l, const PrionStr* v);
PrionList* prion_dc_list(PrionList* h, long long str_elem);
PrionList* prion_list_slice(PrionList* l, long long a, long long b);

"#;


const PRION_MAP_RT: &str = r#"
\
     typedef struct { long long len; long long cap; long long esz; long long sgn; long long kty; long long ksz; char* k; char* v; long long* bk; long long bcap; } PrionMap;
\
PrionMap* prion_map_new(long long esz, long long sgn, long long kty);
long long prion_map_len(PrionMap* m);
void prion_map_reserve(PrionMap* m, long long n);
void prion_map_free(PrionMap* m, long long vstr);
long long prion_map_find_n(PrionMap* m, long long k, long long kty);
long long prion_map_contains_n(PrionMap* m, long long k, long long kty);
long long prion_map_at_n(PrionMap* m, long long k, long long kty, long long esz, long long sgn);
long long prion_map_at_n_loc(PrionMap* m, long long k, long long kty, long long esz, long long sgn, const char* f, long long ln, long long cl, const char* u);
long long prion_map_at_loc(PrionMap* m, long long k, const char* f, long long ln, long long cl, const char* u);
PrionStr prion_map_at_s(PrionMap* m, long long k, long long kty);
PrionStr prion_map_at_s_loc(PrionMap* m, long long k, long long kty, const char* f, long long ln, long long cl, const char* u);
void prion_map_set_s(PrionMap* m, long long k, const PrionStr* v, long long kty);
void prion_map_put_s(PrionMap* m, long long k, const PrionStr* v, long long kty);
void prion_map_set_n(PrionMap* m, long long k, long long v, long long kty, long long esz);
void prion_map_put_n(PrionMap* m, long long k, long long v, long long kty, long long esz);
long long prion_map_find(PrionMap* m, long long k);
long long prion_map_contains(PrionMap* m, long long k);
long long prion_map_at(PrionMap* m, long long k);
void prion_map_set(PrionMap* m, long long k, long long v);
void prion_map_put(PrionMap* m, long long k, long long v);
long long prion_map_remove(PrionMap* m, long long k);
void prion_map_clear(PrionMap* m);
PrionMap* prion_map_lit(long long n, const void* k, const long long* v, long long esz, long long sgn, long long kty);
PrionList* prion_map_keys(PrionMap* m);
PrionList* prion_map_values(PrionMap* m, long long vstr);
PrionMap* prion_dc_map(PrionMap* m, long long str_val);
void prion_map_print_key(long long k, long long kty);
void prion_map_print(PrionMap* m);
void prion_map_print_f(PrionMap* m);
void prion_map_print_s(PrionMap* m);
\
"#;


fn uses_map(p: &Program) -> bool {
    p.functions.iter().any(|f| {
        f.locals.iter().any(|l| l.ty.0 == "map" || l.ty.0.starts_with("map("))
            || f.blocks.iter().any(|b| {
                b.stmts.iter().any(|s| {
                    matches!(s, Stmt::MapSet { .. })
                        || matches!(s, Stmt::Assign { rhs: Rvalue::MapLit { .. }, .. } | Stmt::AssignRelease { rhs: Rvalue::MapLit { .. }, .. })
                        || matches!(s, Stmt::Assign { rhs: Rvalue::MapGet { .. }, .. } | Stmt::AssignRelease { rhs: Rvalue::MapGet { .. }, .. })
                })
            })
    })
}



fn uses_list(p: &Program) -> bool {
    const LIST_RET_FNS: &[&str] = &["args", "keys", "vals", "slice"];
    p.functions.iter().any(|f| {
        f.locals.iter().any(|l| {
            let t = l.ty.0.as_str();
            t == "list"
                || t.starts_with("list(")
                || t.starts_with("que")
                || t.starts_with("stack")
        })
            || f.blocks.iter().any(|b| {
                b.stmts.iter().any(|s| match s {
                    Stmt::ListSet { .. } => true,
                    Stmt::Assign { rhs: Rvalue::Call { name, .. }, .. }
                    | Stmt::AssignRelease { rhs: Rvalue::Call { name, .. }, .. } => {
                        LIST_RET_FNS.contains(&name.as_str())
                    }
                    _ => false,
                })
            })
    })
}


fn uses_str(p: &Program) -> bool {
    
    
    
    
    const STR_FNS: &[&str] = &[
        "len", "len_utf8", "concat", "slice", "sub_str", "nth_char", "code", "char", "int", "str",
        "split", "join", "replace", "starts_with", "ends_with", "trim",
        
        "#str_iter_start", "#str_iter_next",
    ];
    p.functions.iter().any(|f| {
        f.blocks.iter().any(|b| {
            b.stmts.iter().any(|s| match s {
                Stmt::Assign { rhs: Rvalue::Call { name, .. }, .. }
                | Stmt::AssignRelease { rhs: Rvalue::Call { name, .. }, .. } => {
                    STR_FNS.contains(&name.as_str())
                }
                
                Stmt::Assign { rhs: Rvalue::BinOp { op: BinOp::Eq | BinOp::Ne, lhs, rhs }, .. }
                | Stmt::AssignRelease { rhs: Rvalue::BinOp { op: BinOp::Eq | BinOp::Ne, lhs, rhs }, .. } => {
                    is_str_val(f, lhs) && is_str_val(f, rhs)
                }
                Stmt::Assign { rhs: Rvalue::BinOp { op: BinOp::Add, lhs, rhs }, .. }
                | Stmt::AssignRelease { rhs: Rvalue::BinOp { op: BinOp::Add, lhs, rhs }, .. } => {
                    is_str_val(f, lhs) || is_str_val(f, rhs)
                }
                _ => false,
            })
        })
    })
}


fn uses_file(p: &Program) -> bool {
    called_builtins(p)
        .iter()
        .any(|n| matches!(*n, "read_file" | "save_file"))
}


fn uses_args(p: &Program) -> bool {
    called_builtins(p).contains("args")
}


fn called_builtins(p: &Program) -> std::collections::BTreeSet<&str> {
    let mut s = std::collections::BTreeSet::new();
    for f in &p.functions {
        for b in &f.blocks {
            for st in &b.stmts {
                match st {
                    Stmt::Call { name, .. } => {
                        s.insert(name.as_str());
                    }
                    Stmt::Assign { rhs: Rvalue::Call { name, .. }, .. }
                    | Stmt::AssignRelease { rhs: Rvalue::Call { name, .. }, .. } => {
                        s.insert(name.as_str());
                    }
                    _ => {}
                }
            }
        }
    }
    s
}


fn vname(id: LocalId) -> String {
    
    if let Some(g) = SLOT_MAP.with(|m| m.borrow().get(&id).cloned()) {
        return g;
    }
    format!("v{}", id)
}


fn field_c_name(types: &TypeMap, owner: Option<&str>, field: &str) -> String {
    if let Some((_, fields)) = owner.and_then(|o| types.get(o)) {
        if let Some(i) = fields.iter().position(|(n, _)| n == field) {
            return format!("f{}", i);
        }
    }
    format!("f_{}", field)
}

fn field_ty(types: &TypeMap, owner: Option<&str>, field: &str) -> Option<String> {
    owner
        .and_then(|o| types.get(o))
        .and_then(|(_, fs)| fs.iter().find(|(n, _)| n == field).map(|(_, t)| t.0.clone()))
}


fn mmio_ty(name: &str) -> Option<&'static str> {
    match name {
        "read_8" | "write_8" => Some("char"),
        "read_16" | "write_16" => Some("short"),
        "read_32" | "write_32" => Some("int"),
        "read_64" | "write_64" => Some("long long"),
        _ => None,
    }
}


fn place_c(f: &Function, types: &TypeMap, p: &Place) -> String {
    let mut s = vname(p.base);
    let mut cur: Option<String> = f.locals.get(p.base).map(|l| l.ty.0.clone());
    let mut is_ptr = cur.as_deref().is_some_and(|t| t.ends_with('*'));
    for pr in &p.proj {
        match pr {
            Proj::Field(n) => {
                let owner = cur.as_deref().map(|t| t.strip_suffix('*').unwrap_or(t));
                s.push_str(if is_ptr { "->" } else { "." });
                s.push_str(&field_c_name(types, owner, n));
                cur = field_ty(types, owner, n);
                is_ptr = cur.as_deref().is_some_and(|t| t.ends_with('*'));
            }
            Proj::Index(i) => s.push_str(&format!("[{}]", i)),
            Proj::Deref => {
                
                s = format!("(*{})", s);
                cur = cur.as_deref().and_then(|t| t.strip_suffix('*')).map(|t| t.to_string());
                is_ptr = false;
            }
        }
    }
    s
}



fn 边实参代码(f: &Function, s: &prion_ir::Succ) -> String {
    if s.args.is_empty() {
        return String::new();
    }
    let Some(b) = f.blocks.get(s.target) else {
        return String::new();
    };
    let mut o = String::new();
    for (k, a) in s.args.iter().enumerate() {
        if let Some((pid, _)) = b.params.get(k) {
            o.push_str(&format!("  {} = {};\n", vname(*pid), addr_c(a)));
        }
    }
    o
}



fn place_ty_of(f: &Function, types: &TypeMap, p: &Place) -> String {
    let mut t = f.locals.get(p.base).map(|l| l.ty.0.clone()).unwrap_or_default();
    for pr in &p.proj {
        match pr {
            Proj::Deref => {
                if let Some(s) = t.strip_suffix('*') {
                    t = s.to_string();
                }
            }
            Proj::Field(n) => {
                let owner = t.strip_suffix('*').unwrap_or(&t).to_string();
                if let Some(ft) = field_ty(types, Some(&owner), n) {
                    t = ft;
                }
            }
            Proj::Index(_) => {
                if let Some(e) = elem_ty_of(&t) {
                    t = e;
                }
            }
        }
    }
    t
}

fn c_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\\' => o.push_str("\\\\"),
            '"' => o.push_str("\\\""),
            _ => o.push(c),
        }
    }
    o
}


fn fn_sig_c(s: &str) -> (String, Vec<String>) {
    if let Some(rest) = s.strip_prefix("fn(") {
        let mut 深 = 1i32;
        for (k, c) in rest.char_indices() {
            match c {
                '(' => 深 += 1,
                ')' => {
                    深 -= 1;
                    if 深 == 0 {
                        let ret = rest[k + 1..].trim();
                        if ret.is_empty() {
                            break;
                        }
                        let 段 = rest[..k].trim();
                        let ps = if 段.is_empty() {
                            Vec::new()
                        } else {
                            split_ty_list(段)
                        };
                        return (ret.to_string(), ps);
                    }
                }
                _ => {}
            }
        }
    }
    ("long long".to_string(), Vec::new())
}

fn split_ty_list(s: &str) -> Vec<String> {
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

fn value_c(v: &Value) -> String {
    match v {
        
        Value::Local(id) => {
            let n = vname(*id);
            if 槽集.with(|s| s.borrow().contains(id)) {
                format!("(*{})", n)
            } else {
                n
            }
        }
        
        Value::Int(i) => format!("{}LL", i),
        Value::Float(x) => format!("{:?}", x),
        Value::Str(s) => format!("\"{}\"", c_escape(s)),
        Value::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
        Value::Null => "0".to_string(),
    }
}


fn addr_c(v: &Value) -> String {
    match v {
        Value::Local(id) => vname(*id),
        _ => value_c(v),
    }
}


fn slot_bits_c(f: &Function, v: &Value) -> String {
    let t = match v {
        Value::Local(id) => f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_default(),
        Value::Float(_) => "float".to_string(),
        Value::Str(_) => "str".to_string(),
        Value::Bool(_) => "bool".to_string(),
        _ => "int".to_string(),
    };
    let x = value_c(v);
    if t == "float" {
        format!("prion_bits_f64({})", x)
    } else if t == "str"
        || t.ends_with('*')
        || t.starts_with("list")
        || t.starts_with("que")
        || t.starts_with("stack")
        || t.starts_with("map")
    {
        format!("(long long)(size_t)({})", x)
    } else {
        x
    }
}


fn guarded(op: BinOp) -> Option<&'static str> {
    
    if RELEASE.with(|c| c.get()) {
        return None;
    }
    match op {
        BinOp::Add => Some("prion_sadd"),
        BinOp::Sub => Some("prion_ssub"),
        BinOp::Mul => Some("prion_smul"),
        BinOp::Div => Some("prion_div"),
        BinOp::Mod => Some("prion_mod"),
        _ => None,
    }
}


fn container_kind(ty: &str) -> &'static str {
    if ty.starts_with("que") { "que" } else if ty.starts_with("stack") { "stack" } else { "list" }
}


fn container_val_kind(f: &Function, v: &Value) -> Option<&'static str> {
    let ty = match v {
        Value::Local(id) => f.locals.get(*id).map(|l| l.ty.0.as_str())?,
        _ => return None,
    };
    if ty.starts_with("list(str") {
        Some("list(str)")
    } else if ty == "list" || ty.starts_with("list(") {
        if matches!(container_elem_ty(ty).as_deref(), Some("float")) {
            Some("list(float)")
        } else {
            Some("list")
        }
    } else if ty.starts_with("que") {
        if matches!(container_elem_ty(ty).as_deref(), Some("str")) {
            Some("que(str)")
        } else if matches!(container_elem_ty(ty).as_deref(), Some("float")) {
            Some("que(float)")
        } else {
            Some("que")
        }
    } else if ty.starts_with("stack") {
        if matches!(container_elem_ty(ty).as_deref(), Some("str")) {
            Some("stack(str)")
        } else if matches!(container_elem_ty(ty).as_deref(), Some("float")) {
            Some("stack(float)")
        } else {
            Some("stack")
        }
    } else if ty == "map" || ty.starts_with("map(") {
        if matches!(container_elem_ty(ty).as_deref(), Some("str")) {
            Some("map(str)")
        } else if matches!(container_elem_ty(ty).as_deref(), Some("float")) {
            Some("map(float)")
        } else {
            Some("map")
        }
    } else {
        None
    }
}


fn print_one_c(f: &Function, a: &Value) -> (String, String) {
    let is_bool = match a {
        Value::Bool(_) => true,
        Value::Local(id) => f.locals.get(*id).map(|l| l.ty.0 == "bool").unwrap_or(false),
        _ => false,
    };
    if is_bool {
        return ("%s".into(), format!("prion_bool_str({})", value_c(a)));
    }
    match a {
        Value::Str(_) => ("%s".into(), value_c(a)),
        Value::Float(_) => ("%g".into(), value_c(a)),
        Value::Local(id) => match f.locals.get(*id).map(|l| l.ty.0.as_str()) {
            Some("str") => ("%s".into(), value_c(a)),
            Some("float") => ("%g".into(), value_c(a)),
            
            Some(t)
                if matches!(
                    t,
                    "int_8" | "uint_8" | "int_16" | "uint_16" | "int_32" | "uint_32" | "int_64"
                        | "uint_64"
                ) =>
            {
                ("%lld".into(), format!("(long long)({})", value_c(a)))
            }
            _ => ("%lld".into(), value_c(a)),
        },
        _ => ("%lld".into(), value_c(a)),
    }
}


fn elem_ty_of(ty: &str) -> Option<String> {
    ty.strip_prefix("list(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|s| s.to_string())
}


fn container_elem_ty(ty: &str) -> Option<String> {
    let (head, inner) = ty.split_once('(')?;
    let tail = inner.strip_suffix(')')?;
    match head {
        "list" | "que" | "stack" => Some(tail.trim().to_string()),
        "map" => {
            
            let mut depth = 0i32;
            for (i, c) in tail.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    ',' if depth == 0 => return Some(tail[i + 1..].trim().to_string()),
                    _ => {}
                }
            }
            None
        }
        _ => None,
    }
}


fn elem_esz_sgn(t: &str, types: &TypeMap) -> (String, i64) {
    
    if let Some((c, _)) = types.get(t) {
        return (format!("sizeof({})", c), 0);
    }
    let (n, s) = match t {
        "int_8" => (1, 1),
        "uint_8" => (1, 0),
        "int_16" => (2, 1),
        "uint_16" => (2, 0),
        "int_32" => (4, 1),
        "uint_32" => (4, 0),
        "int_64" => (8, 1),
        "uint_64" => (8, 0),
        _ => (8, 1),
    };
    (n.to_string(), s)
}


fn is_agg_ty(t: &str, types: &TypeMap) -> bool {
    types.contains_key(t)
}


fn is_agg_val(f: &Function, types: &TypeMap, v: &Value) -> bool {
    match v {
        Value::Local(id) => f
            .locals
            .get(*id)
            .map(|l| is_agg_ty(l.ty.0.trim_end_matches('*'), types))
            .unwrap_or(false),
        _ => false,
    }
}


fn is_ptr_val(f: &Function, v: &Value) -> bool {
    match v {
        Value::Local(id) => f.locals.get(*id).map(|l| l.ty.0.ends_with('*')).unwrap_or(false),
        _ => false,
    }
}



fn is_handle_val(f: &Function, v: &Value) -> bool {
    match v {
        Value::Local(id) => f
            .locals
            .get(*id)
            .map(|l| l.ty.0.ends_with('*'))
            .unwrap_or(false),
        _ => false,
    }
}

fn is_str_val(f: &Function, v: &Value) -> bool {
    match v {
        Value::Str(_) => true,
        Value::Local(id) => f.locals.get(*id).map(|l| l.ty.0 == "str").unwrap_or(false),
        _ => false,
    }
}


fn map_kty(ty: &str) -> i64 {
    if let Some(rest) = ty.strip_prefix("map(") {
        let k = rest.split(',').next().unwrap_or("").trim();
        if k == "int" {
            return 1;
        }
    }
    0
}


fn key_c(f: &Function, v: &Value) -> String {
    if is_str_val(f, v) {
        format!("(long long)(size_t)({})", value_c(v))
    } else {
        value_c(v)
    }
}


fn container_elem_is_float(f: &Function, v: &Value) -> bool {
    match v {
        Value::Local(id) => f
            .locals
            .get(*id)
            .and_then(|l| container_elem_ty(&l.ty.0))
            .map(|e| e == "float")
            .unwrap_or(false),
        _ => false,
    }
}


fn container_elem_is_agg(f: &Function, v: &Value, types: &TypeMap) -> bool {
    match v {
        Value::Local(id) => f
            .locals
            .get(*id)
            .and_then(|l| container_elem_ty(&l.ty.0))
            .map(|e| is_agg_ty(&e, types))
            .unwrap_or(false),
        _ => false,
    }
}


fn 容器元素助手(f: &Function, v: &Value, types: &TypeMap) -> String {
    match v {
        Value::Local(id) => f
            .locals
            .get(*id)
            .and_then(|l| container_elem_ty(&l.ty.0))
            .and_then(|e| types.get(&e).map(|(c, _)| c.replace("prion_st_", "prion_at_st_")))
            .unwrap_or_default(),
        _ => String::new(),
    }
}


fn is_float_val(f: &Function, v: &Value) -> bool {
    match v {
        Value::Float(_) => true,
        Value::Local(id) => f.locals.get(*id).map(|l| l.ty.0 == "float").unwrap_or(false),
        _ => false,
    }
}


fn value_is_handle(f: &Function, v: &Value) -> bool {
    match v {
        Value::Local(id) => f
            .locals
            .get(*id)
            .map(|l| l.ty.0.ends_with('*'))
            .unwrap_or(false),
        _ => false,
    }
}

fn binop_c(
    f: &Function,
    op: BinOp,
    lhs: &Value,
    rhs: &Value,
    used: &mut BTreeSet<&'static str>,
) -> String {
    
    if matches!(op, BinOp::Eq | BinOp::Ne) && is_str_val(f, lhs) && is_str_val(f, rhs) {
        used.insert("prion_str_eq");
        let c = format!("prion_str_eq({}, {})", value_c(lhs), value_c(rhs));
        return if matches!(op, BinOp::Ne) { format!("(!{})", c) } else { c };
    }

    if matches!(op, BinOp::Add) && (is_str_val(f, lhs) || is_str_val(f, rhs)) {
        return format!("prion_concat({}, {})", value_c(lhs), value_c(rhs));
    }
    let a = value_c(lhs);
    let b = value_c(rhs);
    
    if is_float_val(f, lhs) || is_float_val(f, rhs) {
        if let Some(sym) = match op {
            BinOp::Add => Some("+"),
            BinOp::Sub => Some("-"),
            BinOp::Mul => Some("*"),
            BinOp::Div => Some("/"),
            _ => None,
        } {
            return format!("(({}) {} ({}))", a, sym, b);
        }
    }
    let core = match guarded(op) {
        Some(fn_name) => {
            used.insert(fn_name);
            format!("{}({}, {})", fn_name, a, b)
        }
        None => {
            let sym = match op {
                
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Eq => "==",
                BinOp::Ne => "!=",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::And => "&&",
                BinOp::Or => "||",
                BinOp::BitAnd => "&",
                BinOp::BitOr => "|",
                BinOp::BitXor => "^",
                BinOp::Shl => "<<",
                BinOp::Shr => ">>",
                _ => unreachable!(),
            };
            format!("(({}) {} ({}))", a, sym, b)
        }
    };
    
    if is_wrap_op(op) {
        if let Some(t) = fixed_cty(f, lhs).or_else(|| fixed_cty(f, rhs)) {
            return format!("(({})({}))", t, core);
        }
    }
    core
}


fn is_wrap_op(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
    )
}


fn fixed_cty(f: &Function, v: &Value) -> Option<&'static str> {
    let ty = match v {
        Value::Local(id) => f.locals.get(*id).map(|l| l.ty.0.as_str())?,
        _ => return None,
    };
    match ty {
        "int_8" => Some("signed char"),
        "uint_8" => Some("unsigned char"),
        "int_16" => Some("short"),
        "uint_16" => Some("unsigned short"),
        "int_32" => Some("int"),
        "uint_32" => Some("unsigned int"),
        "int_64" => Some("long long"),
        "uint_64" => Some("unsigned long long"),
        _ => None,
    }
}

fn stmt_c(
    f: &Function,
    s: &Stmt,
    out: &mut String,
    used: &mut BTreeSet<&'static str>,
    names: &BTreeMap<String, String>,
    types: &TypeMap,
) -> Result<(), CodegenError> {
    match s {

        Stmt::Call { name, args }
            if mmio_ty(name).is_some() && name.starts_with("write_") => {
            match (mmio_ty(name), args.first(), args.get(1)) {
                (Some(t), Some(a), Some(v)) => {
                    out.push_str(&format!(
                        "  (*(volatile {}*)({}) = ({})({}));\n",
                        t,
                        value_c(a),
                        t,
                        value_c(v)
                    ));
                    return Ok(());
                }
                _ => return Err(CodegenError { message: format!("「{}」要地址与值两个参数", name) , code: "E401".into() , line: 0 }),
            }
        }
        
        Stmt::Alloc { .. } | Stmt::Bind { .. } => {}
        
        
        Stmt::Drop { obj } => {
            let lids: Vec<usize> = f
                .locals
                .iter()
                .enumerate()
                .filter(|(_, l)| l.obj == Some(*obj))
                .map(|(i, _)| i)
                .collect();
            let owned = OWNED.with(|s| s.borrow().contains(obj));
            
            let 值入 = 值入列.with(|s| s.borrow().contains(obj));
            for lid in lids {
                let Some(l) = f.locals.get(lid) else { continue };
                let ct = c_ty_in(&l.ty, types);
                if ct.starts_with("PrionMap") || ct.starts_with("PrionList") {
                    
                    let 聚合元 = container_elem_ty(&l.ty.0)
                        .map(|e| types.contains_key(e.trim_end_matches('*')))
                        .unwrap_or(false);
                    out.push_str(&drop_container_code(&vname(lid), &l.ty.0, owned || 值入, 聚合元));
                } else if l.ty.0.strip_suffix('*').map(|b| types.contains_key(b)).unwrap_or(false) {
                    
                    let vn = vname(lid);
                    out.push_str(&format!("  if ({vn}) {{ free({vn}); {vn} = 0; }}\n"));
                } else if l.ty.0 == "str" {
                    
                    let vn = vname(lid);
                    out.push_str(&format!("  if ({vn}) {{ prion_str_free({vn}); {vn} = 0; }}\n"));
                }
            }
        }
        
        Stmt::DropVar { lid } => {
            let Some(l) = f.locals.get(*lid) else { return Ok(()) };
            let ct = c_ty_in(&l.ty, types);
            let vn = vname(*lid);
            if ct.starts_with("PrionMap") || ct.starts_with("PrionList") {
                let o = l.obj.unwrap_or(u32::MAX);
                let owned = OWNED.with(|s| s.borrow().contains(&o));
                let 值入 = 值入列.with(|s| s.borrow().contains(&o));
                
                let 聚合元 = container_elem_ty(&l.ty.0)
                    .map(|e| types.contains_key(e.trim_end_matches('*')))
                    .unwrap_or(false);
                out.push_str(&drop_container_code(&vn, &l.ty.0, owned || 值入, 聚合元));
            } else if l.ty.0.strip_suffix('*').map(|b| types.contains_key(b)).unwrap_or(false) {
                out.push_str(&format!("  if ({vn}) {{ free({vn}); {vn} = 0; }}
"));
            } else if l.ty.0 == "str" {
                out.push_str(&format!("  if ({vn}) {{ prion_str_free({vn}); {vn} = 0; }}
"));
            }
        }
        
        Stmt::Asm(form) => {
            let outs: Vec<String> = form
                .outs
                .iter()
                .map(|(约束, id, 读写)| {
                    let 名 = f
                        .locals
                        .get(*id)
                        .and_then(|l| l.name.clone())
                        .unwrap_or_else(|| format!("o{}", id));
                    format!(
                        "[{}]\"{}{}\"({})",
                        名,
                        if *读写 { "+" } else { "=" },
                        约束,
                        vname(*id)
                    )
                })
                .collect();
            let ins: Vec<String> = form
                .ins
                .iter()
                .enumerate()
                .map(|(k, (约束, v))| format!("[i{}]\"{}\"({})", k, 约束, value_c(v)))
                .collect();
            let 破坏: Vec<String> = form.clobbers.iter().map(|c| format!("\"{}\"", c)).collect();
            out.push_str(&format!(
                "  __asm__ volatile (\"{}\" : {} : {} : {});\n",
                c_escape(&form.tmpl),
                outs.join(", "),
                ins.join(", "),
                破坏.join(", ")
            ));
            return Ok(());
        }
        
        Stmt::Call { name, args } => {
            
            if name == "#str_iter_start" {
                if let Some(a) = args.first() {
                    out.push_str(&format!("  prion_str_iter_start({});\n", value_c(a)));
                }
                return Ok(());
            }
            if name == "print" {
                
                if let (Some(Value::Local(id)), None) = (args.first(), args.get(1)) {
                    let ty = f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_default();
                    if ty.starts_with("list(str") {
                        out.push_str(&format!("  prion_list_print_s({});\n  printf(\"\\n\");\n", value_c(&args[0])));
                        return Ok(());
                    }
                    
                    if matches!(ty.as_str(), "int_8" | "uint_8" | "int_16" | "uint_16"
                        | "int_32" | "uint_32" | "int_64" | "uint_64")
                    {
                        out.push_str(&format!("  printf(\"%lld\", (long long)({}))", value_c(&args[0])));
                        out.push_str(";\n");
                        return Ok(());
                    }
                    if ty.starts_with("list(float") {
                        out.push_str(&format!("  prion_list_print_f({});\n  printf(\"\\n\");\n", value_c(&args[0])));
                        return Ok(());
                    }
                    if ty == "list" || ty.starts_with("list(") {
                        out.push_str(&format!("  prion_list_print({});\n  printf(\"\\n\");\n", value_c(&args[0])));
                        return Ok(());
                    }
                    
                    if ty.starts_with("que") {
                        let fname = match container_elem_ty(&ty).as_deref() {
                            Some("str") => "prion_list_print_tag_s",
                            Some("float") => "prion_list_print_tag_f",
                            _ => "prion_list_print_tag",
                        };
                        out.push_str(&format!(
                            "  {}({}, \"que\");\n  printf(\"\\n\");\n",
                            fname,
                            value_c(&args[0])
                        ));
                        return Ok(());
                    }
                    if ty.starts_with("stack") {
                        let fname = match container_elem_ty(&ty).as_deref() {
                            Some("str") => "prion_list_print_tag_s",
                            Some("float") => "prion_list_print_tag_f",
                            _ => "prion_list_print_tag",
                        };
                        out.push_str(&format!(
                            "  {}({}, \"stack\");\n  printf(\"\\n\");\n",
                            fname,
                            value_c(&args[0])
                        ));
                        return Ok(());
                    }
                    if ty == "map" || ty.starts_with("map(") {
                        let fname = match container_elem_ty(&ty).as_deref() {
                            Some("str") => "prion_map_print_s",
                            Some("float") => "prion_map_print_f",
                            _ => "prion_map_print",
                        };
                        out.push_str(&format!("  {}({});\n  printf(\"\\n\");\n", fname, value_c(&args[0])));
                        return Ok(());
                    }
                }
                
                
                if args.iter().any(|a| container_val_kind(f, a).is_some()) {
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            out.push_str("  printf(\" \");\n");
                        }
                        match container_val_kind(f, a) {
                            Some("list(str)") => {
                                out.push_str(&format!("  prion_list_print_s({});\n", value_c(a)))
                            }
                            Some("list(float)") => {
                                out.push_str(&format!("  prion_list_print_f({});\n", value_c(a)))
                            }
                            Some("list") => {
                                out.push_str(&format!("  prion_list_print({});\n", value_c(a)))
                            }
                            Some("que(str)") => out.push_str(&format!(
                                "  prion_list_print_tag_s({}, \"que\");\n",
                                value_c(a)
                            )),
                            Some("que(float)") => out.push_str(&format!(
                                "  prion_list_print_tag_f({}, \"que\");\n",
                                value_c(a)
                            )),
                            Some("que") => out.push_str(&format!(
                                "  prion_list_print_tag({}, \"que\");\n",
                                value_c(a)
                            )),
                            Some("stack(str)") => out.push_str(&format!(
                                "  prion_list_print_tag_s({}, \"stack\");\n",
                                value_c(a)
                            )),
                            Some("stack(float)") => out.push_str(&format!(
                                "  prion_list_print_tag_f({}, \"stack\");\n",
                                value_c(a)
                            )),
                            Some("stack") => out.push_str(&format!(
                                "  prion_list_print_tag({}, \"stack\");\n",
                                value_c(a)
                            )),
                            Some("map(str)") => {
                                out.push_str(&format!("  prion_map_print_s({});\n", value_c(a)))
                            }
                            Some("map(float)") => {
                                out.push_str(&format!("  prion_map_print_f({});\n", value_c(a)))
                            }
                            Some("map") => {
                                out.push_str(&format!("  prion_map_print({});\n", value_c(a)))
                            }
                            _ => {
                                let (fmt, expr) = print_one_c(f, a);
                                out.push_str(&format!("  printf(\"{}\", {});\n", fmt, expr));
                            }
                        }
                    }

                    out.push_str("  printf(\"\\n\");\n");
                    return Ok(());
                }

                for a in args {
                    if let Value::Local(id) = a {
                        if let Some(l) = f.locals.get(*id) {
                            if types.contains_key(l.ty.0.trim_end_matches('*')) {
                                return Err(CodegenError {
                                    message: "print 不收积类型——先取字段（结构可能成环）".into(),
                                    code: "E401".into(),
                                    line: 0,
                                });
                            }
                        }
                    }
                }
                
                let mut fmts: Vec<String> = Vec::new();
                let mut as_: Vec<String> = Vec::new();
                for a in args {
                    let is_bool = match a {
                        Value::Bool(_) => true,
                        Value::Local(id) => {
                            f.locals.get(*id).map(|l| l.ty.0 == "bool").unwrap_or(false)
                        }
                        _ => false,
                    };
                    if is_bool {
                        fmts.push("%s".into());
                        as_.push(format!("prion_bool_str({})", value_c(a)));
                        continue;
                    }
                    match a {
                        Value::Str(_) => {
                            fmts.push("%s".into());
                            as_.push(value_c(a));
                        }
                        Value::Float(_) => {
                            fmts.push("%g".into());
                            as_.push(value_c(a));
                        }
                        Value::Local(id) => match f.locals.get(*id).map(|l| l.ty.0.as_str()) {
                            Some("str") => {
                                fmts.push("%s".into());
                                as_.push(value_c(a));
                            }
                            Some("float") => {
                                fmts.push("%g".into());
                                as_.push(value_c(a));
                            }
                            
                            Some(t)
                                if matches!(
                                    t,
                                    "int_8" | "uint_8" | "int_16" | "uint_16" | "int_32"
                                        | "uint_32" | "int_64" | "uint_64"
                                ) =>
                            {
                                fmts.push("%lld".into());
                                as_.push(format!("(long long)({})", value_c(a)));
                            }
                            _ => {
                                fmts.push("%lld".into());
                                as_.push(value_c(a));
                            }
                        },
                        _ => {
                            fmts.push("%lld".into());
                            as_.push(value_c(a));
                        }
                    }
                }
                out.push_str(&format!(
                    "  printf(\"{}\\n\", {});\n",
                    fmts.join(" "),
                    as_.join(", ")
                ));
            } else if name == "push" {
                
                if args.len() != 2 {
                    return Err(CodegenError { message: "push 要 2 个参数（列表, 值）".into() , code: "E401".into() , line: 0 });
                }
                if is_str_val(f, &args[1]) {
                    out.push_str(&format!(
                        "  prion_list_push({}, (long long)(size_t)prion_str_copy({}));\n",
                        value_c(&args[0]),
                        value_c(&args[1])
                    ));
                } else if is_float_val(f, &args[1]) {
                    out.push_str(&format!(
                        "  prion_list_push({}, prion_f2s({}));\n",
                        value_c(&args[0]),
                        value_c(&args[1])
                    ));
                } else if is_handle_val(f, &args[1]) {
                    
                    
                    let esc = match &args[1] {
                        Value::Local(id) => f.locals.get(*id).filter(|l| l.fn_owned).and_then(|l| {
                            let b = l.ty.0.trim_end_matches('*').to_string();
                            types.get(&b).map(|(ct, _)| (*id, vname(*id), ct.clone()))
                        }),
                        _ => None,
                    };
                    if let Some((lid, vn, ct)) = &esc {
                        let 已逃 = ESCAPED.with(|s| s.borrow().contains(lid));
                        if !已逃 {
                            out.push_str(&format!(
                                "  {vn} = ({ct}*)prion_copy_to_parent({vn}, (long long)sizeof({ct}));\n",
                                vn = vn,
                                ct = ct
                            ));
                            ESCAPED.with(|s| s.borrow_mut().insert(*lid));
                        }
                    }
                    out.push_str(&format!(
                        "  prion_list_push({}, (long long)(size_t){});\n",
                        value_c(&args[0]),
                        value_c(&args[1])
                    ));
                } else if is_agg_val(f, types, &args[1]) {
                    
                    out.push_str(&format!(
                        "  prion_list_push_from({}, (const char*)&{});\n",
                        value_c(&args[0]),
                        value_c(&args[1])
                    ));
                } else {
                    out.push_str(&format!(
                        "  prion_list_push({}, {});\n",
                        value_c(&args[0]),
                        value_c(&args[1])
                    ));
                }
            } else if name == "#预留列表" {
                out.push_str(&format!(
                    "  prion_list_reserve({}, {});\n",
                    value_c(&args[0]),
                    value_c(&args[1])
                ));
            } else if name == "#预留映射" {
                out.push_str(&format!(
                    "  prion_map_reserve({}, {});\n",
                    value_c(&args[0]),
                    value_c(&args[1])
                ));
            } else if name == "exit" {
                
                if args.len() != 1 {
                    return Err(CodegenError { message: "exit 要 1 个参数（码）".into() , code: "E401".into() , line: 0 });
                }
                out.push_str(&format!("  exit((int)({}));\n", value_c(&args[0])));
            } else if name == "save_file" {
                
                if args.len() != 2 {
                    return Err(CodegenError { message: "save_file 要 2 个参数（路径, 内容）".into() , code: "E401".into() , line: 0 });
                }
                let as_: Vec<String> = args.iter().map(value_c).collect();
                out.push_str(&format!("  prion_save_file({}, {});\n", as_[0], as_[1]));
            } else if name == "put" {
                
                if args.len() != 3 {
                    return Err(CodegenError { message: "put 要 3 个参数（map, 键, 值）".into() , code: "E401".into() , line: 0 });
                }
                let mut as_: Vec<String> = args.iter().map(value_c).collect();
                
                if args.len() > 1 {
                    as_[1] = key_c(f, &args[1]);
                }
                
                if let Some(v) = args.get(2) {
                    
                    let need_box = match v {
                        Value::Str(_) => true,
                        Value::Local(id) => f
                            .locals
                            .get(*id)
                            .map(|l| {
                                let t = l.ty.0.as_str();
                                t == "str"
                                    || t.ends_with('*')
                                    || matches!(
                                        t.split('(').next().unwrap_or("").trim(),
                                        "list" | "que" | "stack" | "map"
                                    )
                            })
                            .unwrap_or(false),
                        _ => false,
                    };
                    if is_str_val(f, v) {
                        
                        as_[2] = format!("(long long)(size_t)prion_str_copy({})", value_c(v));
                    } else if is_float_val(f, v) {
                        as_[2] = format!("prion_f2s({})", value_c(v));
                    } else if need_box {
                        as_[2] = format!("(long long)(size_t)({})", value_c(v));
                    }
                }
                out.push_str(&format!("  prion_map_put({}, {}, {});\n", as_[0], as_[1], as_[2]));
            } else if name == "remove" && matches!(args.first(), Some(Value::Local(id)) if f.locals.get(*id).map(|l| l.ty.0 == "map" || l.ty.0.starts_with("map(")).unwrap_or(false)) {
                
                if args.len() != 2 {
                    return Err(CodegenError { message: "remove 要 2 个参数（map, 键）".into() , code: "E401".into() , line: 0 });
                }
                out.push_str(&format!(
                    "  prion_map_remove({}, {});\n",
                    value_c(&args[0]),
                    key_c(f, &args[1])
                ));
            } else if name == "clear"
                && matches!(args.first(), Some(Value::Local(id))
                    if f.locals.get(*id).map(|l| l.ty.0 == "map" || l.ty.0.starts_with("map(")).unwrap_or(false))
            {
                out.push_str(&format!("  prion_map_clear({});\n", value_c(&args[0])));
            } else if name == "pop" {
                
                let first_ty = match args.first() {
                    Some(Value::Local(id)) => {
                        f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_default()
                    }
                    _ => String::new(),
                };
                if first_ty.starts_with("que") {
                    out.push_str(&format!(
                        "  prion_list_remove({}, 1);\n",
                        value_c(&args[0])
                    ));
                } else {
                    out.push_str(&format!("  prion_list_pop({});\n", value_c(&args[0])));
                }
            } else if name == "insert" || name == "remove" || name == "clear" || name == "sort" {
                
                let first_ty = match args.first() {
                    Some(Value::Local(id)) => {
                        f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_default()
                    }
                    _ => String::new(),
                };
                let kind = container_kind(&first_ty);
                if kind != "list" {
                    return Err(CodegenError {
                        message: "队列/栈只进只出——insert/remove(按位置)/sort 只对列表用".into(),
                        code: "E401".into(),
                        line: 0,
                    });
                }
                let c = match name.as_str() {
                    "run" => {
                        used.insert("prion_run");
                        "prion_run"
                    }
                    "insert" => "prion_list_insert",
                    "remove" => "prion_list_remove",
                    "clear" => "prion_list_clear",
                    _ => {
                        
                        let is_str_elem = matches!(args.first(), Some(Value::Local(id))
                            if f.locals.get(*id).map(|l| l.ty.0.starts_with("list(str")).unwrap_or(false));
                        if is_str_elem {
                            used.insert("prion_list_sort_s");
                            "prion_list_sort_s"
                        } else {
                            "prion_list_sort"
                        }
                    }
                };
                let mut as_: Vec<String> = args.iter().map(value_c).collect();
                if name == "insert" && args.len() == 3 && is_float_val(f, &args[2]) {
                    as_[2] = format!("prion_f2s({})", as_[2]);
                }
                if name == "insert" && args.len() == 3 && is_agg_val(f, types, &args[2]) {
                    
                    as_[2] = format!("(const char*)&{}", as_[2]);
                    out.push_str(&format!("  prion_list_insert_from({});\n", as_.join(", ")));
                    return Ok(());
                }
                out.push_str(&format!("  {}({});\n", c, as_.join(", ")));
            } else if name == "sle_ms" {
                
                if args.len() != 1 {
                    return Err(CodegenError { message: "sleep 要 1 个参数（毫秒）".into() , code: "E401".into() , line: 0 });
                }
                used.insert("prion_sleep");
                out.push_str(&format!("  prion_sleep({});\n", value_c(&args[0])));
            } else if let Some(callee) = names.get(name) {
                
                
                let as_: Vec<String> = args.iter().map(addr_c).collect();
                out.push_str(&format!("  {}({});\n", callee, as_.join(", ")));
            } else {
                return Err(CodegenError { message: format!("暂未支持的内置调用：{}", name) , code: "E401".into() , line: 0 });
            }
        }
        Stmt::AssignRelease { lhs, rhs } if lhs.proj.is_empty() => {
            let n = vname(lhs.base);
            let mut inner = String::new();
            stmt_c(
                f,
                &Stmt::Assign { lhs: lhs.clone(), rhs: rhs.clone() },
                &mut inner,
                used,
                names,
                types,
            )?;
            out.push_str(&format!("  {{ const char* prion_rel = {};\n", n));
            out.push_str(&inner);
            out.push_str("  if (prion_rel) prion_str_free(prion_rel); }\n");
        }
        Stmt::AssignRelease { lhs, rhs } => {
            stmt_c(
                f,
                &Stmt::Assign { lhs: lhs.clone(), rhs: rhs.clone() },
                out,
                used,
                names,
                types,
            )?;
        }
        Stmt::Assign { lhs, rhs } => {
            
            let dest_ty = place_ty_of(f, types, lhs);
            let (elem_esz, elem_sgn) = container_elem_ty(&dest_ty)
                .map(|e| elem_esz_sgn(&e, types))
                .unwrap_or(("8".to_string(), 1));
            let r = match rhs {
                Rvalue::Use(p) => place_c(f, types, p),
                Rvalue::Ref(p) => format!("(&{})", place_c(f, types, p)),
                Rvalue::CtxArray { args } => {
                    let items: Vec<String> = args.iter().map(|a| slot_bits_c(f, a)).collect();
                    if items.is_empty() {
                        "(long long[]){0}".to_string()
                    } else {
                        format!("(long long[]){{{}}}", items.join(", "))
                    }
                }
                Rvalue::CtxGet { ctx, idx, ty } => {
                    let base = format!("{}[{}]", vname(*ctx), idx);
                    let t = ty.0.as_str();
                    if t == "float" {
                        format!("prion_unbits_f64({})", base)
                    } else if t == "str" {
                        format!("((const char*)(size_t)({}))", base)
                    } else if t.ends_with('*')
                        || t.starts_with("list")
                        || t.starts_with("que")
                        || t.starts_with("stack")
                        || t.starts_with("map")
                    {
                        format!("(({})(size_t)({}))", c_ty_in(ty, types), base)
                    } else {
                        base
                    }
                }
                
                Rvalue::VariantLit { ty, tag, args, tys } => {
                    let c = types.get(ty).map(|(c, _)| c.clone()).unwrap_or_else(|| ty.clone());
                    let mut parts = vec![format!(".tag = {}LL", tag)];
                    for (i, a) in args.iter().enumerate() {
                        
                        let v = value_c(a);
                        let v = match tys.get(i).map(|t| t.0.as_str()) {
                            Some("str") => format!("(long long)(size_t)({})", v),
                            
                            
                            Some(t) if t.ends_with('*') => match a {
                                Value::Local(id)
                                    if !f.locals.get(*id).map(|l| l.ty.0.ends_with('*')).unwrap_or(false) =>
                                {
                                    format!("(long long)(size_t)(&{})", v)
                                }
                                _ => format!("(long long)(size_t)({})", v),
                            },
                            _ => v,
                        };
                        parts.push(format!(".p{} = {}", i, v));
                    }
                    format!("(({}){{ {} }})", c, parts.join(", "))
                }
                Rvalue::VariantTag(v) => {
                    
                    let base = if value_is_handle(f, v) {
                        format!("(*{})", value_c(v))
                    } else {
                        value_c(v)
                    };
                    format!("(({}).tag)", base)
                }
                Rvalue::VariantGet { value, idx, ty } => {
                    let base = if value_is_handle(f, value) {
                        format!("(*{})", value_c(value))
                    } else {
                        value_c(value)
                    };
                    let raw = format!("(({}).p{})", base, idx);
                    
                    if ty.0 == "str" {
                        format!("((const char*)(size_t)({}))", raw)
                    } else if ty.0.ends_with('*') {
                        format!("(({})(size_t)({}))", c_ty_in(ty, types), raw)
                    } else {
                        raw
                    }
                }
                Rvalue::Const(v) => value_c(v),
                Rvalue::FnRef { name } => {
                    let callee = names.get(name).cloned().unwrap_or_else(|| name.clone());
                    format!("((long long)(size_t){})", callee)
                }
                Rvalue::CallIndirect { callee, args } => {
                    
                    let as_: Vec<String> = args.iter().map(addr_c).collect();
                    let fty = match callee {
                        Value::Local(id) => f
                            .locals
                            .get(*id)
                            .map(|l| l.ty.0.clone())
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    let (ret, ps) = fn_sig_c(&fty);
                    let ps_c: Vec<String> = ps.iter().map(|t| c_ty_in(&Ty(t.clone()), types)).collect();
                    let ps_s = if ps_c.is_empty() { "void".to_string() } else { ps_c.join(", ") };
                    format!(
                        "(({} (*)({}))(size_t){})({})",
                        c_ty_in(&Ty(ret), types),
                        ps_s,
                        value_c(callee),
                        as_.join(", ")
                    )
                }
                Rvalue::BinOp { op, lhs, rhs } => binop_c(f, *op, lhs, rhs, used),

                Rvalue::Call { name, args }
                    if mmio_ty(name).is_some() && name.starts_with("read_") => {
                    match (mmio_ty(name), args.first()) {
                        (Some(t), Some(a)) => {
                            format!("((long long)(*(volatile {}*)({})))", t, value_c(a))
                        }
                        _ => return Err(CodegenError { message: format!("「{}」要一个地址参数", name) , code: "E401".into() , line: 0 }),
                    }
                }
                Rvalue::Call { name, args } => {
                    let as_: Vec<String> = args.iter().map(value_c).collect();
                    let arg = |i: usize| as_.get(i).cloned().unwrap_or_else(|| "0".to_string());
                    match name.as_str() {
                        "join" | "prion_join" => format!("prion_join({})", as_.get(0).cloned().unwrap_or_else(|| "0".into())),
                        "prion_join_all" => format!("prion_join_all({})", as_.get(0).cloned().unwrap_or_else(|| "0".into())),
                        "len" => {
                            
                            let is_list = matches!(args.first(), Some(Value::Local(id))
                                if f.locals.get(*id).map(|l| {
                                    let t = l.ty.0.as_str();
                                    t == "list" || t == "que" || t == "stack"
                                        || t.starts_with("list(") || t.starts_with("que(") || t.starts_with("stack(")
                                }).unwrap_or(false));
                            let is_map = matches!(args.first(), Some(Value::Local(id))
                                if f.locals.get(*id).map(|l| l.ty.0 == "map" || l.ty.0.starts_with("map(")).unwrap_or(false));
                            if is_list {
                                format!("prion_list_len({})", arg(0))
                            } else if is_map {
                                format!("prion_map_len({})", arg(0))
                            } else {
                                format!("prion_str_char_len({})", arg(0))
                            }
                        }
                        "empty" => {
                            let is_map = matches!(args.first(), Some(Value::Local(id))
                                if f.locals.get(*id).map(|l| l.ty.0 == "map" || l.ty.0.starts_with("map(")).unwrap_or(false));
                            if is_map {
                                format!("(prion_map_len({}) == 0)", arg(0))
                            } else {
                                format!("(prion_list_len({}) == 0)", arg(0))
                            }
                        }
                        "len_utf8" => format!("prion_str_byte_len({})", arg(0)),
                        "str_index" => {
                            
                            format!(
                                "(strstr({}, {}) ? (long long)(strstr({}, {}) - {}) + 1 : 0)",
                                arg(0), arg(1), arg(0), arg(1), arg(0)
                            )
                        }
                        "has" | "index" => {
                            let list_ty = match args.first() {
                                Some(Value::Local(id)) => {
                                    f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_default()
                                }
                                _ => String::new(),
                            };
                            if list_ty == "map" || list_ty.starts_with("map(") {
                                if name == "has" {
                                    format!(
                                        "prion_map_contains({}, {})",
                                        arg(0),
                                        key_c(f, &args[1])
                                    )
                                } else {
                                    
                                    format!(
                                        "prion_map_at({}, {})",
                                        arg(0),
                                        key_c(f, &args[1])
                                    )
                                }
                            } else if list_ty.starts_with("list(str") {
                                if name == "has" {
                                    format!("prion_list_contains_s({}, {})", arg(0), arg(1))
                                } else {
                                    format!("prion_list_index_s({}, {})", arg(0), arg(1))
                                }
                            } else if name == "has" {
                                let b = if is_ptr_val(f, &args[1]) {
                                    format!("(long long)(size_t){}", arg(1))
                                } else {
                                    arg(1)
                                };
                                format!("prion_list_contains({}, {})", arg(0), b)
                            } else {
                                let b = if is_ptr_val(f, &args[1]) {
                                    format!("(long long)(size_t){}", arg(1))
                                } else {
                                    arg(1)
                                };
                                format!("prion_list_index({}, {})", arg(0), b)
                            }
                        }
                        
                        
                        
                        "slice" => {
                            let is_str = matches!(args.first(), Some(Value::Local(id))
                                if f.locals.get(*id).map(|l| l.ty.0 == "str").unwrap_or(false));
                            if is_str {
                                format!("prion_sub_str({}, {}, {})", arg(0), arg(1), arg(2))
                            } else {
                                format!("prion_list_slice({}, {}, {})", arg(0), arg(1), arg(2))
                            }
                        }
                        
                        "#str_copy" => format!("prion_str_copy({})", arg(0)),
                        "#dc_list" => format!("prion_dc_list({}, {})", arg(0), arg(1)),
                        "#dc_map" => format!("prion_dc_map({}, {})", arg(0), arg(1)),
                        
                        n if n.starts_with("#dcdeep_") => {
                            let 型 = n.trim_start_matches("#dcdeep_");
                            let c = types.get(型).map(|t| t.0.clone()).unwrap_or_default();
                            let 助手 = c.replace("prion_st_", "prion_dcdeep_st_");
                            format!("{}(&{})", 助手, arg(0))
                        }
                        "concat" => {
                            if args.is_empty() {
                                "prion_concat_n(0, (const char* const*)0, (long long*)0, 0)".to_string()
                            } else {
                                let ps: Vec<String> = (0..args.len()).map(|i| arg(i)).collect();
                                let mut lens: Vec<String> = Vec::with_capacity(args.len());
                                let mut 字 = 0i64;
                                let mut 全知 = true;
                                for a in args {
                                    match a {
                                        Value::Str(t) if !t.contains('\0') => {
                                            lens.push(format!("{}LL", t.len()));
                                            字 += t.chars().count() as i64;
                                        }
                                        _ => {
                                            lens.push("-1LL".to_string());
                                            全知 = false;
                                        }
                                    }
                                }
                                let chars = if 全知 { 字 } else { -1 };
                                format!(
                                    "prion_concat_n({}, (const char*[]){{{}}}, (long long[]){{{}}}, {}LL)",
                                    args.len(),
                                    ps.join(", "),
                                    lens.join(", "),
                                    chars
                                )
                            }
                        }
                        "sub_str" => format!("prion_sub_str({}, {}, {})", arg(0), arg(1), arg(2)),
                        "nth_char" => format!("prion_nth_char({}, {})", arg(0), arg(1)),
                        
                        "#str_iter_next" => format!("prion_str_iter_next({}, {})", arg(0), arg(1)),
                        "code" => format!("prion_char_code({})", arg(0)),
                        "char" => format!("prion_code_char({})", arg(0)),

                        
                        "que" | "stack" => {
                            if args.is_empty() {
                                format!("prion_list_new({}, {})", elem_esz, elem_sgn)
                            } else {
                                arg(0)
                            }
                        }
                        "args" => "prion_args()".to_string(),
                        "read_file" => format!("prion_read_file({})", arg(0)),
                        
                        "pop" => {
                            let first_ty = match args.first() {
                                Some(Value::Local(id)) => {
                                    f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_default()
                                }
                                _ => String::new(),
                            };
                            let raw = if first_ty.starts_with("que") {
                                format!("prion_list_remove({}, 1)", arg(0))
                            } else {
                                format!("prion_list_pop({})", arg(0))
                            };
                            
                            
                            if matches!(container_elem_ty(&first_ty).unwrap_or_default().as_str(), "str") {
                                format!("((const char*)(size_t){})", raw)
                            } else if matches!(container_elem_ty(&first_ty).unwrap_or_default().as_str(), "float") {
                                format!("prion_s2f({})", raw)
                            } else {
                                raw
                            }
                        }
                        "keys" => format!("prion_map_keys({})", arg(0)),
                        "vals" => format!("prion_map_values({})", arg(0)),
                        "int" => {
                            
                            let is_f = matches!(args.first(), Some(Value::Float(_)))
                                || matches!(args.first(), Some(Value::Local(id))
                                    if f.locals.get(*id).map(|l| l.ty.0 == "float").unwrap_or(false))
                                || matches!(args.first(), Some(Value::Str(_)));
                            if is_f {
                                format!("((long long)({}))", arg(0))
                            } else {
                                format!("prion_int_of_str({})", arg(0))
                            }
                        }
                        "str" => {
                            
                            let is_float = matches!(args.first(), Some(Value::Float(_)))
                                || matches!(args.first(), Some(Value::Local(id))
                                    if f.locals.get(*id).map(|l| l.ty.0 == "float").unwrap_or(false));
                            if is_float {
                                format!("prion_str_of_float({})", arg(0))
                            } else {
                                format!("prion_str_of_int({})", arg(0))
                            }
                        }
                        "float" => {
                            
                            let is_s = matches!(args.first(), Some(Value::Str(_)))
                                || matches!(args.first(), Some(Value::Local(id))
                                    if f.locals.get(*id).map(|l| l.ty.0 == "str").unwrap_or(false));
                            if is_s {
                                format!("prion_float_of_str({})", arg(0))
                            } else {
                                format!("((double)({}))", arg(0))
                            }
                        }
                        "bool" => format!("(({}) != 0)", arg(0)),
                        
                        "int_8" | "uint_8" | "int_16" | "uint_16" | "int_32" | "uint_32"
                        | "int_64" | "uint_64" => {
                            format!("(({})({}))", c_ty_in(&Ty(name.clone()), types), arg(0))
                        }
                        
                        "rnd" => {
                            used.insert("prion_random");
                            format!("prion_random({})", arg(0))
                        }
                        
                        "remove" => {
                            let first_ty = match args.first() {
                                Some(Value::Local(id)) => f.locals.get(*id).map(|l| l.ty.0.clone()).unwrap_or_default(),
                                _ => String::new(),
                            };
                            
                            if first_ty == "map" || first_ty.starts_with("map(") {
                                used.insert("prion_map_remove");
                                format!("prion_map_remove({}, {})", arg(0), key_c(f, &args.get(1).unwrap_or(&Value::Int(0))))
                            } else {
                            used.insert("prion_list_remove");
                            let a1 = args.get(1).map(value_c).unwrap_or_else(|| "1".into());
                            let raw = format!("prion_list_remove({}, {})", arg(0), a1);
                            if matches!(container_elem_ty(&first_ty).unwrap_or_default().as_str(), "str") {
                                format!("((const char*)(size_t){})", raw)
                            } else if matches!(container_elem_ty(&first_ty).unwrap_or_default().as_str(), "float") {
                                format!("prion_s2f({})", raw)
                            } else {
                                raw
                            }
                            }
                        }

                        "run" => {
                            used.insert("prion_run");
                            let a1 = args.get(1).map(value_c).unwrap_or_else(|| "0".into());
                            let a2 = args.get(2).map(value_c).unwrap_or_else(|| "\"\"".into());
                            let a3 = args.get(3).map(value_c).unwrap_or_else(|| "\"\"".into());
                            format!("prion_run({}, {}, {}, {})", arg(0), a1, a2, a3)
                        }
                        "str_to_c" => {
                            used.insert("prion_to_c_str");
                            format!("((long long)(size_t)prion_to_c_str({}))", arg(0))
                        }
                        "str_from_c" => {
                            used.insert("prion_from_c_str");
                            format!("prion_from_c_str({})", arg(0))
                        }
                        
                        "save_file" => {
                            used.insert("prion_save_file");
                            format!("prion_save_file({}, {})", arg(0), arg(1))
                        }
                        _ => {
                            let callee = names.get(name).cloned().unwrap_or_else(|| name.clone());
                            
                            let 地址: Vec<String> = args.iter().map(addr_c).collect();
                            format!("{}({})", callee, 地址.join(", "))
                        }
                    }
                }
                Rvalue::ListLit { elems } => {
                    let es: Vec<String> = elems.iter().map(value_c).collect();
                    if es.is_empty() {
                        format!("prion_list_new({}, {})", elem_esz, elem_sgn)
                    } else if elems.iter().any(|e| is_str_val(f, e)) {
                        
                        format!(
                            "prion_list_lit_s({}, (const char* const[]){{{}}})",
                            es.len(),
                            es.join(", ")
                        )
                    } else if elems.iter().any(|e| is_float_val(f, e)) {
                        
                        let es2: Vec<String> = elems
                            .iter()
                            .map(|e| {
                                if is_float_val(f, e) {
                                    format!("prion_f2s({})", value_c(e))
                                } else {
                                    value_c(e)
                                }
                            })
                            .collect();
                        format!(
                            "prion_list_lit({}, (long long[]){{{}}}, {}, {})",
                            es.len(),
                            es2.join(", "),
                            elem_esz,
                            elem_sgn
                        )
                    } else if elems.iter().any(|e| is_ptr_val(f, e)) {
                        
                        let es2: Vec<String> = elems
                            .iter()
                            .map(|e| format!("(long long)(size_t){}", value_c(e)))
                            .collect();
                        format!(
                            "prion_list_lit({}, (long long[]){{{}}}, {}, {})",
                            es.len(),
                            es2.join(", "),
                            elem_esz,
                            elem_sgn
                        )
                    } else if elems.iter().any(|e| is_agg_val(f, types, e)) {
                        
                        let (c, _) = container_elem_ty(&dest_ty)
                            .and_then(|e| types.get(&e).map(|t| (t.0.clone(), ())))
                            .unwrap_or((String::new(), ()));
                        format!(
                            "prion_list_lit_from({}, (const char*)({}[]){{{}}}, sizeof({}))",
                            es.len(),
                            c,
                            es.join(", "),
                            c
                        )
                    } else {
                        format!(
                            "prion_list_lit({}, (long long[]){{{}}}, {}, {})",
                            es.len(),
                            es.join(", "),
                            elem_esz,
                            elem_sgn
                        )
                    }
                }
                Rvalue::MapLit { pairs } => {
                     
                    let kty = map_kty(&dest_ty);
                    let ks: Vec<String> = pairs
                        .iter()
                        .map(|(k, _)| match k {
                            prion_ir::MapKey::Int(n) => format!("{}LL", n),
                            prion_ir::MapKey::Str(s) => format!("(long long)(size_t)\"{}\"", s),
                        })
                        .collect();
                    let vs: Vec<String> = pairs
                        .iter()
                        .map(|(_, v)| {
                            if is_float_val(f, v) {
                                format!("prion_f2s({})", value_c(v))
                            } else if is_str_val(f, v) {
                                format!("(long long)(size_t)prion_str_copy({})", value_c(v))
                            } else {
                                value_c(v)
                            }
                        })
                        .collect();
                    format!(
                        "prion_map_lit({}, (const long long[]){{{}}}, (long long[]){{{}}}, {}, {}, {})",
                        pairs.len(),
                        ks.join(", "),
                        vs.join(", "),
                        elem_esz,
                        elem_sgn,
                        kty
                    )
                }
                Rvalue::MapGet { map, key } => {
                    
                    
                    let vt = match map {
                        Value::Local(id) => f
                            .locals
                            .get(*id)
                            .and_then(|l| l.ty.0.strip_prefix("map("))
                            .and_then(|x| x.strip_suffix(')'))
                            .and_then(|inner| {
                                let mut d = 0i32;
                                for (i, c) in inner.char_indices() {
                                    match c {
                                        '(' => d += 1,
                                        ')' => d -= 1,
                                        ',' if d == 0 => {
                                            return Some(inner[i + 1..].trim().to_string())
                                        }
                                        _ => {}
                                    }
                                }
                                None
                            })
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    let raw = format!("prion_map_at({}, {})", value_c(map), key_c(f, key));
                    
                    if vt == "str" {
                        format!("((const char*)(size_t){})", raw)
                    } else if vt.ends_with('*') {
                        format!("((void*)(size_t){})", raw)
                    } else if matches!(vt.split('(').next().unwrap_or("").trim(), "list" | "que" | "stack" | "map") {
                        format!("((PrionList*)(size_t){})", raw)
                    } else if vt == "float" {
                        format!("prion_s2f({})", raw)
                    } else {
                        raw
                    }
                }
                Rvalue::ListGet { list, index } => {
                    
                    let elem_str = matches!(list, Value::Local(id)
                        if f.locals.get(*id).map(|l| l.ty.0.starts_with("list(str")).unwrap_or(false));
                    let elem_handle = matches!(list, Value::Local(id)
                        if f.locals.get(*id).map(|l| {
                            let e = elem_ty_of(&l.ty.0).unwrap_or_default();
                            e.ends_with('*')
                        }).unwrap_or(false));
                    
                    let 直读 = format!(
                        "((long long*)({})->d)[({}) - 1]",
                        value_c(list),
                        value_c(index)
                    );
                    if elem_str {
                        format!("((const char*)(size_t){})", 直读)
                    } else if elem_handle {
                        format!("((void*)(size_t){})", 直读)
                    } else if container_elem_is_float(f, list) {
                        format!("prion_s2f({})", 直读)
                    } else if container_elem_is_agg(f, list, types) {
                        
                        let 助手 = 容器元素助手(f, list, types);
                        format!("{}({}, {})", 助手, value_c(list), value_c(index))
                    } else if !RELEASE.with(|c| c.get()) {
                        
                        used.insert("prion_list_at");
                        format!("prion_list_at({}, {})", value_c(list), value_c(index))
                    } else {
                        直读
                    }
                }            };
            
            
            let base_ty = dest_ty.trim_end_matches('*').to_string();
            let is_handle = dest_ty.ends_with('*');
            let is_struct_val = matches!(rhs, Rvalue::Use(_) | Rvalue::Const(Value::Local(_)));

            if is_handle && is_struct_val {
                let src_is_handle = match rhs {
                    Rvalue::Use(p) => f.locals.get(p.base).map(|l| l.ty.0.ends_with('*')).unwrap_or(false),
                    Rvalue::Const(Value::Local(id)) => {
                        f.locals.get(*id).map(|l| l.ty.0.ends_with('*')).unwrap_or(false)
                    }
                    _ => false,
                };
                if src_is_handle {
                    out.push_str(&format!("  {} = {};\n", place_c(f, types, lhs), r));
                    return Ok(());
                }
            }
            if types.contains_key(&base_ty) && is_struct_val {
                let (ct, _) = types.get(&base_ty).unwrap();
                let dc = ct.replace("prion_st_", "prion_dc_");
                let lhs_c = place_c(f, types, lhs);
                if is_handle {
                    let ptr_ct = c_ty_in(&Ty(dest_ty.clone()), types); 
                    out.push_str(&format!(
                        "  {lhs} = ({ptr})malloc((size_t)(sizeof({ct})));\n  *{lhs} = {dc}(&{r});\n",
                        lhs = lhs_c,
                        ptr = ptr_ct,
                        ct = ct,
                        dc = dc,
                        r = r
                    ));
                } else {
                    out.push_str(&format!("  {} = {}(&{});\n", lhs_c, dc, r));
                }
                return Ok(());
            }
            
            
            let 槽赋容器 = matches!(rhs, Rvalue::ListLit { .. })
                && matches!(lhs, Place { base, proj } if proj.is_empty()
                    && f.locals.get(*base).map(|l| l.is_param && l.ty.0.ends_with('*')).unwrap_or(false));
            let r = if 槽赋容器 {
                format!("prion_list_dc_parent({})", r)
            } else {
                r
            };
            
            
            let r = if dest_ty == "str"
                && !matches!(rhs, Rvalue::BinOp { .. })
                && !matches!(rhs, Rvalue::Call { name, .. } if prion_ir::返回自有串(name))
            {
                format!("prion_str_copy({})", r)
            } else {
                r
            };
            out.push_str(&format!("  {} = {};\n", place_c(f, types, lhs), r));
            
            if dest_ty.starts_with("list") || dest_ty.starts_with("map")
                || dest_ty.starts_with("que") || dest_ty.starts_with("stack")
            {
                
                let 源 = match rhs {
                    Rvalue::Const(Value::Local(u)) | Rvalue::Use(Place { base: u, proj: _ }) => Some(*u),
                    Rvalue::Call { name, args } if (name == "que" || name == "stack") && args.len() == 1 => {
                        match args[0] {
                            Value::Local(u) => Some(u),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some(u) = 源 {
                    let 匿名 = f.locals.get(u).map(|l| l.name.is_none()).unwrap_or(false);
                    if 匿名 {
                        out.push_str(&format!("  {} = 0;\n", vname(u)));
                    }
                }
            }
        }
        
        Stmt::AccessBegin(_) | Stmt::AccessEnd(_) => {}
        
        Stmt::MapSet { map, key, value } => {
            let slot = if is_str_val(f, value) {
                format!("(long long)(size_t)prion_str_copy({})", value_c(value))
            } else if is_float_val(f, value) {
                format!("prion_f2s({})", value_c(value))
            } else {
                value_c(value)
            };
            out.push_str(&format!(
                "  prion_map_set({}, {}, {});\n",
                value_c(map),
                key_c(f, key),
                slot
            ))
        }
        
        Stmt::ListSet { list, index, value } => {
            if is_str_val(f, value) {
                out.push_str(&format!(
                    "  prion_list_setat({}, {}, (long long)(size_t)prion_str_copy({}));\n",
                    value_c(list),
                    value_c(index),
                    value_c(value)
                ));
            } else if is_float_val(f, value) {
                out.push_str(&format!(
                    "  prion_list_setat({}, {}, prion_f2s({}));\n",
                    value_c(list),
                    value_c(index),
                    value_c(value)
                ));
            } else if is_agg_val(f, types, value) {
                
                out.push_str(&format!(
                    "  prion_list_setat_from({}, {}, (const char*)&{});\n",
                    value_c(list),
                    value_c(index),
                    value_c(value)
                ));
            } else {
                out.push_str(&format!(
                    "  prion_list_setat({}, {}, {});\n",
                    value_c(list),
                    value_c(index),
                    value_c(value)
                ));
            }
        }
        
        Stmt::Assert { .. } if RELEASE.with(|c| c.get()) => {}
        Stmt::Assert { cond, msg, fix } => out.push_str(&format!(
            "  if (!({})) {{ prion_die({}, prion_line, \"{}\", \"{}\"); }}\n",
            value_c(cond),
            if msg.contains("空引用") { "1" } else { "7" },
            c_escape(msg),
            c_escape(fix)
        )),
    }
    Ok(())
}


thread_local! {
    
    static SLOT_MAP: std::cell::RefCell<std::collections::BTreeMap<usize, String>> =
        std::cell::RefCell::new(std::collections::BTreeMap::new());
    
    static ESCAPED: std::cell::RefCell<std::collections::BTreeSet<usize>> =
        std::cell::RefCell::new(std::collections::BTreeSet::new());
    
    static 槽集: std::cell::RefCell<std::collections::BTreeSet<usize>> =
        std::cell::RefCell::new(std::collections::BTreeSet::new());
}


fn slot_global_name(key: &str, idx: usize) -> String {
    let _ = key;
    format!("prion_slot_{}", idx)
}


fn is_slot_local(l: &Local) -> bool {
    l.name.as_deref().map(|n| n.contains('#')).unwrap_or(false)
}



fn drop_container_code(vn: &str, ty: &str, owned: bool, 聚合元: bool) -> String {
    if ty.starts_with("map(") {
        
        let vstr = if container_elem_ty(ty).as_deref() == Some("str") { 1 } else { 0 };
        return format!("  if ({vn}) {{ prion_map_free({vn}, {vstr}); {vn} = 0; }}\n");
    }
    let ety = container_elem_ty(ty).unwrap_or_default();
    if ety == "str" {
        format!("  if ({vn}) {{ for (long long ri = 0; ri < {vn}->len; ri++) prion_str_free(((void**)({vn}->d))[ri]); free({vn}->d); free({vn}); {vn} = 0; }}\n")
    } else if (ety.ends_with('*') || 聚合元) && owned {
        format!("  if ({vn}) {{ for (long long ri = 0; ri < {vn}->len; ri++) free(((void**)({vn}->d))[ri]); free({vn}->d); free({vn}); {vn} = 0; }}\n")
    } else {
        format!("  if ({vn}) {{ free({vn}->d); free({vn}); {vn} = 0; }}\n")
    }
}



fn block_c(
    f: &Function,
    b: &Block,
    out: &mut String,
    used: &mut BTreeSet<&'static str>,
    is_void: bool,
    names: &BTreeMap<String, String>,
    types: &TypeMap,
) -> Result<(), CodegenError> {
    out.push_str(&format!("bb{}:\n", b.id));
    for (i, s) in b.stmts.iter().enumerate() {
        
        if let Some(sp) = b.spans.get(i) {
            if sp.line > 0 {
                if DEBUG_LINES.with(|c| c.get()) {
                    let 文件名 = SRC_FILE.with(|c| c.borrow().clone());
                    out.push_str(&format!("#line {} \"{}\"\n", sp.line, c_escape(&文件名)));
                    
                    let key = if f.file.is_empty() {
                        SRC_FILE.with(|c| c.borrow().clone())
                    } else {
                        f.file.clone()
                    };
                    let 行文本 = SRC_LINES
                        .with(|m| m.borrow().get(&key).and_then(|ls| ls.get(sp.line - 1).cloned()))
                        .unwrap_or_default();
                    if 行文本.is_empty() {
                        out.push_str("  prion_src = 0;\n");
                    } else {
                        out.push_str(&format!("  prion_src = \"{}\";\n", c_escape(&行文本)));
                    }
                    out.push_str(&format!(
                        "  prion_line = {}; prion_col = {}; prion_len = {};\n",
                        sp.line, sp.col, sp.len
                    ));
                }
            }
        }
        stmt_c(f, s, out, used, names, types)?;
    }
    match &b.exit {
        Exit::Return(Some(v)) => {
            
            
            let esc = match v {
                Value::Local(id) => f.locals.get(*id).filter(|l| l.fn_owned).and_then(|l| {
                    let b = l.ty.0.trim_end_matches('*').to_string();
                    types.get(&b).map(|(ct, _)| (*id, vname(*id), ct.clone()))
                }),
                _ => None,
            };
            if let Some((lid, vn, ct)) = &esc {
                let 已逃 = ESCAPED.with(|s| s.borrow().contains(lid));
                if !已逃 {
                    out.push_str(&format!(
                        "  {vn} = ({ct}*)prion_copy_to_parent({vn}, (long long)sizeof({ct}));\n",
                        vn = vn,
                        ct = ct
                    ));
                    ESCAPED.with(|s| s.borrow_mut().insert(*lid));
                }
            }
            out.push_str(&format!("  return {};\n", value_c(v)))
        }
        
        Exit::Return(None) => {
            out.push_str(if is_void { "  return;\n" } else { "  return 0;\n" })
        }
        Exit::Unreachable => out.push_str("  abort();\n"),
        Exit::Branch { cond, targets } => match targets.len() {
            0 => return Err(CodegenError { message: "分支没有目标块".into() , code: "E401".into() , line: 0 }),
            1 => {
                let t = targets[0].target;
                out.push_str(&边实参代码(f, &targets[0]));
                out.push_str(&format!("  goto bb{};\n", t))
            }
            2 => {
                let (t0, t1) = (targets[0].target, targets[1].target);
                let cond = value_c(cond);
                let a0 = 边实参代码(f, &targets[0]);
                let a1 = 边实参代码(f, &targets[1]);
                out.push_str(&format!(
                    "  if ({}) {{ {}goto bb{}; }} else {{ {}goto bb{}; }}\n",
                    cond, a0, t0, a1, t1
                ))
            }
            _ => {
                out.push_str(&format!("  switch ({}) {{\n", value_c(cond)));
                for (i, t) in targets.iter().enumerate() {
                    out.push_str(&边实参代码(f, t));
                    out.push_str(&format!("    case {}: goto bb{};\n", i, t.target));
                }
                out.push_str("  }\n");
            }
        },
    }
    Ok(())
}

fn support_section(used: &BTreeSet<&'static str>) -> String {
    let mut s = String::new();

    s
}


fn ffi_str_section(used: &BTreeSet<&'static str>) -> String {
    if !(used.contains("prion_to_c_str") || used.contains("prion_from_c_str")) {
        return String::new();
    }
    "const char* prion_to_c_str(const PrionStr* s) {\n  size_t n = (size_t)(s && s->p ? s->n : 0) + 1;\n  char* p = prion_str_alloc((long long)n);\n  if (s && s->p) memcpy(p, s->p, n - 1);\n  p[n - 1] = 0;\n  return p;\n}\nPrionStr prion_from_c_str(long long addr) {\n  const char* s = (const char*)(size_t)addr;\n  s = s ? s : \"\";\n  size_t n = strlen(s) + 1;\n  char* p = prion_str_alloc((long long)n);\n  memcpy(p, s, n);\n  return prion_str_mk(p, (long long)(n - 1));\n}\n".to_string()
}


const PRION_RUN_RT: &str = r#"
#ifndef _WIN32
#include <unistd.h>
#include <sys/wait.h>
#include <fcntl.h>
#include <errno.h>
long long prion_run(const PrionStr* prog_in, PrionList* args, const PrionStr* out_file_in, const PrionStr* err_file_in) {
  const char* prog = (prog_in && prog_in->p) ? prog_in->p : "";
  const char* out_file = (out_file_in && out_file_in->p) ? out_file_in->p : 0;
  const char* err_file = (err_file_in && err_file_in->p) ? err_file_in->p : 0;
  if (!prog[0]) return -404;
  long long n = args ? args->len : 0;
  char** av = (char**)malloc(sizeof(char*) * (size_t)(n + 2));
  if (!av) return -10086;
  av[0] = (char*)prog;
  long long i = 0;
  while (i < n) {
    PrionStr ea = prion_desc_at(prion_list_slot(args, i));
    av[i + 1] = (char*)(ea.p ? ea.p : "");
    i = i + 1;
  }
  av[n + 1] = 0;
  int errp[2];
  if (pipe(errp) != 0) { free(av); return -10086; }
  fcntl(errp[0], F_SETFD, FD_CLOEXEC);
  fcntl(errp[1], F_SETFD, FD_CLOEXEC);
  pid_t pid = fork();
  if (pid < 0) { close(errp[0]); close(errp[1]); free(av); return -10086; }
  if (pid == 0) {
    close(errp[0]);
    if (out_file && out_file[0]) {
      int fd = open(out_file, O_WRONLY | O_CREAT | O_TRUNC, 0644);
      if (fd < 0) { int e = errno; ssize_t w = write(errp[1], &e, sizeof e); (void)w; _exit(127); }
      dup2(fd, 1);
      close(fd);
    }
    if (err_file && err_file[0]) {
      int fd = open(err_file, O_WRONLY | O_CREAT | O_TRUNC, 0644);
      if (fd < 0) { int e = errno; ssize_t w = write(errp[1], &e, sizeof e); (void)w; _exit(127); }
      dup2(fd, 2);
      close(fd);
    }
    execvp(prog, av);
    { int e = errno; ssize_t w = write(errp[1], &e, sizeof e); (void)w; }
    _exit(127);
  }
  close(errp[1]);
  int e = 0;
  ssize_t got = read(errp[0], &e, sizeof e);
  close(errp[0]);
  free(av);
  int st = 0;
  while (waitpid(pid, &st, 0) < 0 && errno == EINTR) {}
  if (got == (ssize_t)sizeof e) return (e == ENOENT) ? -404 : -10086;
  if (WIFEXITED(st)) return (long long)WEXITSTATUS(st);
  return -4;
}
#endif
"#;



fn mark_runtime(name: &str, used: &mut BTreeSet<&'static str>) {
    match name {
        "prion_spawn" | "prion_spawn_detached" | "prion_join" | "join" => {
            used.insert("prion_spawn");
        }
        "prion_join_all" => {
            used.insert("prion_spawn");
            used.insert("prion_join_all");
        }
        "rnd" => {
            used.insert("prion_random");
        }

        "run" => {
            used.insert("prion_run");
        }
        "sle_ms" => {
            used.insert("prion_sleep");
        }
        "str_to_c" => {
            used.insert("prion_to_c_str");
        }
        "str_from_c" => {
            used.insert("prion_from_c_str");
        }
        _ => {}
    }
}




const PRION_COPY_RT: &str = r#"
static void* prion_copy_to_parent(const void* src, long long n) {
  void* d = malloc((size_t)(n));
  if (src && d) memcpy(d, src, (size_t)n);
  return d;
}

"#;


#[derive(Debug, Clone, Copy, PartialEq)]
enum 块类 {
    
    指令,
    
    声明,
    
    函数,
    
    全局,
    
    原型,
}


fn 顶层切块(s: &str) -> Result<Vec<(块类, &str)>, String> {
    let b = s.as_bytes();
    let n = b.len();
    let mut out: Vec<(块类, &str)> = Vec::new();
    let mut i = 0usize;
    while i < n {
        
        loop {
            while i < n && (b[i] as char).is_whitespace() {
                i += 1;
            }
            if i + 1 < n && b[i] == b'/' && b[i + 1] == b'/' {
                while i < n && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if i + 1 < n && b[i] == b'/' && b[i + 1] == b'*' {
                i += 2;
                while i + 1 < n && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(n);
                continue;
            }
            break;
        }
        if i >= n {
            break;
        }
        let 起 = i;
        
        if b[i] == b'#' {
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            out.push((块类::指令, &s[起..i]));
            continue;
        }
        let mut 深 = 0i32;
        let mut 串 = 0u8;
        let mut 行注 = false;
        let mut 块注 = false;
        let mut 终 = None;
        while i < n {
            let c = b[i];
            if 行注 {
                if c == b'\n' {
                    行注 = false;
                }
                i += 1;
                continue;
            }
            if 块注 {
                if c == b'*' && i + 1 < n && b[i + 1] == b'/' {
                    块注 = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if 串 != 0 {
                if c == b'\\' {
                    i += 2;
                    continue;
                }
                if c == 串 {
                    串 = 0;
                }
                i += 1;
                continue;
            }
            match c {
                b'"' | b'\'' => {
                    串 = c;
                    i += 1;
                }
                b'/' if i + 1 < n && b[i + 1] == b'/' => {
                    行注 = true;
                    i += 2;
                }
                b'/' if i + 1 < n && b[i + 1] == b'*' => {
                    块注 = true;
                    i += 2;
                }
                b'{' => {
                    深 += 1;
                    i += 1;
                }
                b'}' => {
                    深 -= 1;
                    i += 1;
                    if 深 < 0 {
                        return Err(format!("前置段括号不配对，偏移 {}", 起));
                    }
                    if 深 == 0 {
                        
                        let mut j = i;
                        while j < n && (b[j] as char).is_whitespace() {
                            j += 1;
                        }
                        if j < n && b[j] == b';' {
                            i = j + 1;
                        } else {
                            
                            let mut k = j;
                            while k < n && ((b[k] as char).is_ascii_alphanumeric() || b[k] == b'_') {
                                k += 1;
                            }
                            let mut m = k;
                            while m < n && (b[m] as char).is_whitespace() {
                                m += 1;
                            }
                            if k > j && m < n && b[m] == b';' {
                                i = m + 1;
                            }
                        }
                        终 = Some(i);
                        break;
                    }
                }
                b';' if 深 == 0 => {
                    i += 1;
                    终 = Some(i);
                    break;
                }
                _ => i += 1,
            }
        }
        let e = 终.ok_or_else(|| format!("前置段切块没找到结尾，偏移 {}", 起))?;
        let t = s[起..e].trim();
        out.push((判类(t)?, &s[起..e]));
        i = e;
    }
    Ok(out)
}


fn 判类(t: &str) -> Result<块类, String> {
    if t.starts_with('#') {
        return Ok(块类::指令);
    }
    for k in ["typedef", "struct", "enum", "union"] {
        if t.starts_with(k) {
            return Ok(块类::声明);
        }
    }
    if t.ends_with('}') && t.contains('(') {
        return Ok(块类::函数);
    }
    if t.ends_with(';') {
        
        if t.starts_with("static ") {
            return Ok(块类::全局);
        }
        return Ok(if t.contains('=') { 块类::全局 } else { 块类::原型 });
    }
    Err(format!("前置段有认不出的块：{}", &t[..t.len().min(60)]))
}


fn 转原型(t: &str) -> Option<String> {
    let p = t.find('{')?;
    let mut sig = t[..p].trim();
    if let Some(r) = sig.strip_prefix("static ") {
        sig = r.trim();
    }
    Some(format!("{};", sig))
}


fn 转外部(t: &str) -> String {
    let mut out = String::new();
    for 段 in t.split(';') {
        let 段 = 段.trim();
        if 段.is_empty() {
            continue;
        }
        let 头 = match 段.find('=') {
            Some(e) => 段[..e].trim(),
            None => 段,
        };
        out.push_str("extern ");
        out.push_str(头);
        out.push_str(";\n");
    }
    out
}





pub fn runtime_c() -> String {
    runtime_c_for("", "")
}


pub fn runtime_c_for(_file: &str, _src_text: &str) -> String {
    let mut used: BTreeSet<&'static str> = BTreeSet::new();
    for n in [
        "prion_sadd", "prion_ssub", "prion_smul", "prion_div", "prion_mod", "prion_run",
        "prion_list_at", "prion_list_push", "prion_map_at", "prion_str_eq", "prion_concat",
        "prion_str_index",
        "prion_read_file", "prion_save_file", "prion_args", "prion_sleep", "prion_random",
        "prion_to_c_str", "prion_from_c_str", "prion_spawn", "prion_join",
    ] {
        used.insert(n);
    }
    let mut s = String::new();
    s.push_str("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n\n");
    
    s.push_str("#ifdef PRION_NOGUARD\n#define PRION_GUARD(x)\n#else\n#define PRION_GUARD(x) x\n#endif\n\n");
    s.push_str("typedef struct { const char* p; long long n; } PrionStr;\n");
    s.push_str("void prion_at(void);\nvoid prion_die(int code, long long line, const char* msg, const char* fix);\nvoid prion_die_nc(int code, const char* msg, const char* fix);\nvoid prion_die_at(const char* file, long long line, long long col, const char* src, int code, const char* msg, const char* fix);\nvoid prion_task_fail_notify(int code, long long line, const char* msg, const char* fix);\nvoid prion_str_free(const char* p);\nchar* prion_str_alloc(long long n);\nPrionStr prion_str_empty(void);\n");
    s.push_str("static PrionStr prion_str_mk(const char* p, long long n) { PrionStr s; s.p = p; s.n = n; return s; }\n\n");
    s.push_str("long long prion_line = 0; long long prion_col = 0; long long prion_len = 0; const char* prion_cur_file = 0; const char* prion_src = 0;\n\n");
    s.push_str(&support_section(&used));
    s.push_str(PRION_COPY_RT);
    s.push_str(&ffi_str_section(&used));
    s.push_str(PRION_LIST_RT);
    s.push_str(PRION_RUN_RT);
    s.push_str(PRION_MAP_RT);
    s.replace("static long long prion_sadd(", "long long prion_sadd(")
        .replace("static long long prion_ssub(", "long long prion_ssub(")
        .replace("static long long prion_smul(", "long long prion_smul(")
        .replace("static void* prion_copy_to_parent(", "void* prion_copy_to_parent(")
        .replace("static void prion_str_iter_start(", "void prion_str_iter_start(")
        .replace("static PrionStr prion_str_iter_next(", "PrionStr prion_str_iter_next(")
}
