
















pub mod disposition;
pub mod flat;
pub use disposition::形参观感;

use prion_error::Span;
use std::fmt;








pub type LocalId = usize;
pub type BlockId = usize;


pub type ObjId = u32;

pub use prion_types::StorageClass;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjEnt {
    pub class: StorageClass,
    pub ty: Ty,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ty(pub String);


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proj {
    Field(String),
    Index(i64),
    Deref,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub base: LocalId,
    pub proj: Vec<Proj>,
}

impl Place {
    pub fn local(id: LocalId) -> Self {
        Self { base: id, proj: Vec::new() }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Local(LocalId),
    Int(i64),
    Float(f64),
    Str(String),
    
    Bool(bool),
    Null,
}


#[derive(Debug, Clone, PartialEq)]
pub enum MapKey {
    Str(String),
    Int(i64),
}


#[derive(Debug, Clone, PartialEq)]
pub enum Rvalue {
    Use(Place),
    Const(Value),
    BinOp { op: BinOp, lhs: Value, rhs: Value },
    Call { name: String, args: Vec<Value> },
    
    FnRef { name: String },
    
    CallIndirect { callee: Value, args: Vec<Value> },
    
    ListLit { elems: Vec<Value> },
    
    ListGet { list: Value, index: Value },
    
    
    MapLit { pairs: Vec<(MapKey, Value)> },
    
    MapGet { map: Value, key: Value },
    
    VariantLit { ty: String, tag: usize, args: Vec<Value>, tys: Vec<Ty> },
    
    VariantTag(Value),
    
    VariantGet { value: Value, idx: usize, ty: Ty },
    
    Ref(Place),
    
    CtxArray { args: Vec<Value> },
    
    CtxGet { ctx: LocalId, idx: usize, ty: Ty },
}


#[derive(Debug, Clone, PartialEq)]
pub struct AsmForm {
    pub tmpl: String,
    
    pub outs: Vec<(String, LocalId, bool)>,
    
    pub ins: Vec<(String, Value)>,
    
    pub clobbers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    
    BitAnd,
    BitOr,
    BitXor,
    
    Shl,
    Shr,
}

impl BinOp {
    pub fn as_str(self) -> &'static str {
        match self {
            BinOp::Add => "add",
            BinOp::Sub => "sub",
            BinOp::Mul => "mul",
            BinOp::Div => "div",
            BinOp::Mod => "mod",
            BinOp::Eq => "eq",
            BinOp::Ne => "ne",
            BinOp::Lt => "lt",
            BinOp::Le => "le",
            BinOp::Gt => "gt",
            BinOp::Ge => "ge",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::BitAnd => "band",
            BinOp::BitOr => "bor",
            BinOp::BitXor => "bxor",
            BinOp::Shl => "shl",
            BinOp::Shr => "shr",
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Assign { lhs: Place, rhs: Rvalue },

    AssignRelease { lhs: Place, rhs: Rvalue },
    
    Call { name: String, args: Vec<Value> },
    
    Alloc { obj: ObjId, class: StorageClass, ty: Ty },
    
    Bind { var: LocalId, obj: ObjId },
    
    Drop { obj: ObjId },
    
    DropVar { lid: LocalId },
    
    Kill { block: String },
    
    ListSet { list: Value, index: Value, value: Value },
    
    MapSet { map: Value, key: Value, value: Value },
    AccessBegin(Place),
    AccessEnd(Place),
    
    Assert { cond: Value, msg: String, fix: String },
    
    Asm(AsmForm),
}

impl Stmt {
    pub fn 赋值体(&self) -> Option<(&Place, &Rvalue)> {
        match self {
            Stmt::Assign { lhs, rhs } | Stmt::AssignRelease { lhs, rhs } => Some((lhs, rhs)),
            _ => None,
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct Succ {
    pub target: BlockId,
    
    pub args: Vec<Value>,
}

impl Succ {
    
    pub fn new(target: BlockId) -> Self {
        Self { target, args: Vec::new() }
    }
    
    pub fn with_args(target: BlockId, args: Vec<Value>) -> Self {
        Self { target, args }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Exit {
    
    Branch { cond: Value, targets: Vec<Succ> },
    Return(Option<Value>),
    Unreachable,
}


#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub id: BlockId,
    
    pub params: Vec<(LocalId, Ty)>,
    pub stmts: Vec<Stmt>,
    
    pub spans: Vec<Span>,
    pub exit: Exit,
    
    pub max: Option<u64>,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    
    pub name: Option<String>,
    pub ty: Ty,
    pub is_param: bool,
    pub bare_ok: bool,
    
    pub line: usize,
    pub col: usize,
    
    pub loop_local: bool,
    
    pub fn_owned: bool,
    
    pub obj: Option<ObjId>,
}


#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    
    pub file: String,
    pub ret: Option<Ty>,
    pub locals: Vec<Local>,
    pub blocks: Vec<Block>,
    
    pub is_block: bool,
    
    pub is_pub: bool,
    
    pub is_sle: bool,
    
    pub is_kill: bool,
    
    pub is_abi: bool,
    
    pub is_naked: bool,
    
    pub asm_lines: Vec<String>,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructType {
    pub name: String,
    
    pub params: Vec<String>,
    pub fields: Vec<(String, Ty)>,
    
    pub variants: Vec<Variant>,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternFn {
    
    pub name: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
}


#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub functions: Vec<Function>,
    
    pub types: Vec<StructType>,
    
    pub externs: Vec<ExternFn>,
    
    pub objects: Vec<ObjEnt>,
    
    pub owned_elem: std::collections::BTreeMap<ObjId, Vec<ObjId>>,
    
    pub 值入列: std::collections::BTreeSet<ObjId>,
    
    pub 平铺: std::collections::BTreeMap<String, std::collections::BTreeSet<LocalId>>,
}


pub fn is_container_ty(t: &str) -> bool {
    let n = t.trim_end_matches('*');
    n.starts_with("list(") || n.starts_with("map(") || n.starts_with("que(") || n.starts_with("stack(")
}


pub fn 返回自有串(名: &str) -> bool {
    matches!(名, "concat" | "#str_copy" | "read_file" | "str_from_c")
}









fn local_name(f: &Function, id: LocalId) -> String {
    match f.locals.get(id).and_then(|l| l.name.clone()) {
        Some(n) => n,
        None => {
            
            let mut k = 0usize;
            for (i, l) in f.locals.iter().enumerate() {
                if l.name.is_none() {
                    if i == id {
                        return format!("%t{}", k);
                    }
                    k += 1;
                }
            }
            format!("%t{}", id)
        }
    }
}

fn place_str(f: &Function, p: &Place) -> String {
    let mut s = local_name(f, p.base);
    for pr in &p.proj {
        s.push_str(&match pr {
            Proj::Field(n) => format!(".{}", n),
            Proj::Index(i) => format!("[{}]", i),
            Proj::Deref => "*".to_string(),
        });
    }
    s
}


fn map_key_str(k: &MapKey) -> String {
    match k {
        MapKey::Str(s) => format!("\"{}\"", s),
        MapKey::Int(i) => i.to_string(),
    }
}

fn value_str(f: &Function, v: &Value) -> String {
    match v {
        Value::Local(id) => local_name(f, *id),
        Value::Int(i) => i.to_string(),
        Value::Float(x) => format!("{}", x),
        Value::Str(s) => format!("\"{}\"", escape_str(s)),
        Value::Bool(b) => (if *b { "re" } else { "fa" }).to_string(),
        Value::Null => "null".to_string(),
    }
}

fn escape_str(s: &str) -> String {
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

fn rvalue_str(f: &Function, r: &Rvalue) -> String {
    match r {
        Rvalue::Use(p) => place_str(f, p),
        Rvalue::Ref(p) => format!("ref({})", place_str(f, p)),
        Rvalue::VariantLit { ty, tag, args, .. } => format!(
            "variant {}({}; {})",
            ty,
            tag,
            args.iter().map(|a| value_str(f, a)).collect::<Vec<_>>().join(", ")
        ),
        Rvalue::VariantTag(v) => format!("vtag({})", value_str(f, v)),
        Rvalue::VariantGet { value, idx, .. } => format!("vget({}, {})", value_str(f, value), idx),
        Rvalue::Const(v) => value_str(f, v),
        Rvalue::BinOp { op, lhs, rhs } => {
            format!("{}({}, {})", op.as_str(), value_str(f, lhs), value_str(f, rhs))
        }
        Rvalue::Call { name, args } => format!(
            "call {}({})",
            name,
            args.iter().map(|a| value_str(f, a)).collect::<Vec<_>>().join(", ")
        ),
        Rvalue::FnRef { name } => format!("fn_ref({})", name),
        Rvalue::CallIndirect { callee, args } => format!(
            "call_indirect({}; {})",
            value_str(f, callee),
            args.iter().map(|a| value_str(f, a)).collect::<Vec<_>>().join(", ")
        ),
        Rvalue::ListLit { elems } => format!(
            "list_lit({})",
            elems.iter().map(|e| value_str(f, e)).collect::<Vec<_>>().join(", ")
        ),
        Rvalue::ListGet { list, index } => {
            format!("list_get({}, {})", value_str(f, list), value_str(f, index))
        }
        Rvalue::MapLit { pairs } => format!(
            "map_lit({})",
            pairs
                .iter()
                .map(|(k, v)| format!("{}: {}", map_key_str(k), value_str(f, v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Rvalue::MapGet { map, key } => {
            format!("map_get({}, {})", value_str(f, map), value_str(f, key))
        }
        Rvalue::CtxArray { args } => format!(
            "ctx[{}]",
            args.iter().map(|a| value_str(f, a)).collect::<Vec<_>>().join(", ")
        ),
        Rvalue::CtxGet { idx, ty, .. } => format!("ctx_get({}; {})", idx, ty.0),
    }
}

fn stmt_str(f: &Function, s: &Stmt) -> String {
    match s {
        Stmt::Assign { lhs, rhs } => format!("assign {}, {}", place_str(f, lhs), rvalue_str(f, rhs)),
        Stmt::AssignRelease { lhs, rhs } => format!(
            "assign_release {}, {}",
            place_str(f, lhs),
            rvalue_str(f, rhs)
        ),
        Stmt::Call { name, args } => format!(
            "call {}({})",
            name,
            args.iter().map(|a| value_str(f, a)).collect::<Vec<_>>().join(", ")
        ),
        Stmt::Alloc { obj, ty, .. } => format!("alloc o{} {}", obj, ty.0),
        Stmt::Bind { var, obj } => format!("bind {} o{}", local_name(f, *var), obj),
        Stmt::Drop { obj } => format!("drop o{}", obj),
        Stmt::DropVar { lid } => format!("dropvar {}", local_name(f, *lid)),
        Stmt::Kill { block } => format!("kill #:{}", block),
        Stmt::ListSet { list, index, value } => format!(
            "list_set({}, {}, {})",
            value_str(f, list),
            value_str(f, index),
            value_str(f, value)
        ),
        Stmt::MapSet { map, key, value } => format!(
            "map_set({}, {}, {})",
            value_str(f, map),
            value_str(f, key),
            value_str(f, value)
        ),
        Stmt::AccessBegin(p) => format!("access_begin {}", place_str(f, p)),
        Stmt::AccessEnd(p) => format!("access_end {}", place_str(f, p)),
        Stmt::Assert { cond, msg, fix } => {
            format!("assert {}, \"{}\", \"{}\"", value_str(f, cond), escape_str(msg), escape_str(fix))
        }
        Stmt::Asm(form) => format!("asm({})", form.tmpl),
    }
}

fn exit_str(f: &Function, e: &Exit) -> String {
    match e {
        
        Exit::Branch { targets, .. } if targets.len() == 1 => {
            let s = &targets[0];
            if s.args.is_empty() {
                format!("goto bb{}", s.target)
            } else {
                format!(
                    "goto bb{}({})",
                    s.target,
                    s.args.iter().map(|a| value_str(f, a)).collect::<Vec<_>>().join(", ")
                )
            }
        }
        Exit::Branch { cond, targets } => format!(
            "branch {} -> {}",
            value_str(f, cond),
            targets
                .iter()
                .map(|s| {
                    if s.args.is_empty() {
                        format!("bb{}", s.target)
                    } else {
                        format!(
                            "bb{}({})",
                            s.target,
                            s.args.iter().map(|a| value_str(f, a)).collect::<Vec<_>>().join(", ")
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Exit::Return(Some(v)) => format!("ret {}", value_str(f, v)),
        Exit::Return(None) => "ret".to_string(),
        Exit::Unreachable => "unreachable".to_string(),
    }
}

impl fmt::Display for Program {
    
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for t in &self.types {
            let fs: Vec<String> = t
                .fields
                .iter()
                .map(|(n, ty)| format!("{}:{}", n, ty.0))
                .collect();
            writeln!(f, "type {}({})", t.name, fs.join(", "))?;
        }
        if !self.types.is_empty() {
            writeln!(f)?;
        }
        for e in &self.externs {
            let ps: Vec<String> = e.params.iter().map(|t| t.0.clone()).collect();
            writeln!(f, "outs fn {}({}) -> {}", e.name, ps.join(", "), e.ret.0)?;
        }
        if !self.externs.is_empty() {
            writeln!(f)?;
        }
        for (fi, fun) in self.functions.iter().enumerate() {
            if fi > 0 {
                writeln!(f)?;
            }            let params: Vec<String> = fun
                .locals
                .iter()
                .enumerate()
                .filter(|(_, l)| l.is_param)
                .map(|(i, l)| format!("{}:{}", local_name(fun, i), l.ty.0))
                .collect();
            match &fun.ret {
                Some(t) => writeln!(f, "fn {}({}) -> {}", fun.name, params.join(", "), t.0)?,
                None => writeln!(f, "fn {}({})", fun.name, params.join(", "))?,
            }
            let locals: Vec<String> = fun
                .locals
                .iter()
                .enumerate()
                .filter(|(_, l)| !l.is_param)
                .map(|(i, l)| format!("{}:{}", local_name(fun, i), l.ty.0))
                .collect();
            if !locals.is_empty() {
                writeln!(f, "  locals: {}", locals.join(", "))?;
            }
            for b in &fun.blocks {
                let ps: Vec<String> = b
                    .params
                    .iter()
                    .map(|(id, t)| format!("{}:{}", local_name(fun, *id), t.0))
                    .collect();
                if ps.is_empty() {
                    writeln!(f, "  bb{}:", b.id)?;
                } else {
                    writeln!(f, "  bb{}({}):", b.id, ps.join(", "))?;
                }
                for s in &b.stmts {
                    writeln!(f, "    {}", stmt_str(fun, s))?;
                }
                writeln!(f, "    {}", exit_str(fun, &b.exit))?;
            }
        }
        Ok(())
    }
}

impl Program {
    
    pub fn dump(&self) -> String {
        format!("{}", self)
    }
}