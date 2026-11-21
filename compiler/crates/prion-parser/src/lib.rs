










use prion_error::{render_parse, ParseCode, DiagParts, Span, Tone};
use prion_lexer::{Spanned, Token};


pub mod expr;

pub use expr::Expr;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
    pub code: String,

    pub parts: DiagParts,
    
    pub 可延后: bool,
}

impl ParseError {
    pub fn with_code(line: usize, code: ParseCode, parts: &DiagParts, tone: Tone) -> Self {
        Self {
            line,
            message: render_parse(code, parts, tone),
            code: code.number().to_string(),
            parts: parts.clone(),
            可延后: false,
        }
    }

    
    pub fn 延后(mut self) -> Self {
        self.可延后 = true;
        self
    }
}

fn 行表达式(toks: &TokStream) -> Vec<Expr> {
    let t: &[Token] = toks;
    let sp = toks.spans();
    let mut 深 = 0i32;
    let mut eq = None;
    for (k, tk) in t.iter().enumerate() {
        match tk {
            Token::Sym("(") | Token::Sym("[") | Token::Sym("{") => 深 += 1,
            Token::Sym(")") | Token::Sym("]") | Token::Sym("}") => 深 -= 1,
            Token::Sym("=") if 深 == 0 => {
                eq = Some(k);
                break;
            }
            _ => {}
        }
    }
    let 起 = eq.map(|e| e + 1).unwrap_or(0);
    
    let 起 = if matches!(t.get(起), Some(Token::Kw("give"))) { 起 + 1 } else { 起 };
    if 起 >= t.len() {
        return Vec::new();
    }
    let mut p = expr::Parser::new(&t[起..], &sp[起..], 1);
    match p.parse() {
        Ok(e) => vec![e],
        Err(_) => Vec::new(),
    }
}



#[derive(Debug, Clone, PartialEq)]
pub struct TokStream {
    toks: Vec<Token>,
    spans: Vec<(usize, usize)>,
}

impl TokStream {
    
    pub fn new(三元组: Vec<(Token, usize, usize)>) -> Self {
        let mut toks = Vec::with_capacity(三元组.len());
        let mut spans = Vec::with_capacity(三元组.len());
        for (t, c, l) in 三元组 {
            toks.push(t);
            spans.push((c, l));
        }
        debug_assert_eq!(toks.len(), spans.len());
        TokStream { toks, spans }
    }

    
    pub fn 无位(v: Vec<Token>) -> TokStream {
        let n = v.len();
        TokStream { toks: v, spans: vec![(0, 0); n] }
    }

    
    pub fn get2(&self, i: usize) -> Option<(&Token, (usize, usize))> {
        self.toks.get(i).map(|t| (t, self.spans[i]))
    }

    
    pub fn span(&self, i: usize) -> Option<(usize, usize)> {
        self.spans.get(i).copied()
    }

    
    pub fn spans(&self) -> &[(usize, usize)] {
        &self.spans
    }
}

impl std::ops::Deref for TokStream {
    type Target = [Token];
    fn deref(&self) -> &[Token] {
        &self.toks
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    
    Line { line: usize, col: usize, len: usize, kind: LineKind, exprs: Vec<Expr> },
    
    Struct { line: usize, col: usize, len: usize, kind: StructKind, head: TokStream, body: Vec<Stmt> },
    
    Fn { line: usize, col: usize, len: usize, head: TokStream, body: Vec<Stmt> },
}


#[derive(Debug, Clone, PartialEq)]
pub enum LineKind {
    Give(TokStream),
    
    AddVal(TokStream),
    
    AddType(TokStream),
    
    AddHide(TokStream),
    
    AddFn(TokStream),
    
    AddOutsFn(TokStream),
    
    CallFn(TokStream),
    
    CallBlock(TokStream),
    
    
    Assign(TokStream),
    
    BareCall(TokStream),
    
    Other(TokStream),
}

impl LineKind {
    
    pub fn tokens(&self) -> &TokStream {
        match self {
            LineKind::Give(t) | LineKind::AddVal(t) | LineKind::AddFn(t) | LineKind::AddOutsFn(t)
            | LineKind::CallFn(t) | LineKind::CallBlock(t)  | LineKind::Assign(t)
            | LineKind::BareCall(t) | LineKind::Other(t) => t,
            LineKind::AddType(t) | LineKind::AddHide(t) => t,
        }
    }
}


pub fn line_kind(toks: &TokStream) -> LineKind {
    let own = toks.clone();
    match toks.first() {
        Some(Token::Kw("give")) => LineKind::Give(own),
        Some(Token::Kw("add")) => match toks.get(1) {
            Some(Token::Kw("share")) if matches!(toks.get(2), Some(Token::Kw("val"))) => LineKind::AddVal(own),
            Some(Token::Kw("val")) | Some(Token::Kw("fix")) => LineKind::AddVal(own),
            Some(Token::Kw("type")) => LineKind::AddType(own),
            Some(Token::Kw("hide")) => match toks.get(2) {
                Some(Token::Kw("fn")) => LineKind::AddFn(own),
                _ => LineKind::AddHide(own),
            },
            Some(Token::Kw("fn")) => LineKind::AddFn(own),
            Some(Token::Kw("outs")) if matches!(toks.get(2), Some(Token::Kw("fn"))) => LineKind::AddOutsFn(own),
            _ => LineKind::Other(own),
        },
        Some(Token::Kw("fn")) => LineKind::AddFn(own),
        Some(Token::Kw("call")) => match toks.get(1) {
            Some(Token::Kw("fn")) => LineKind::CallFn(own),
            _ => LineKind::CallBlock(own),
        },
        Some(Token::Kw("val")) => LineKind::Assign(own),
        Some(Token::Ident(_)) if matches!(toks.get(1), Some(Token::Sym("("))) => LineKind::BareCall(own),
        _ => LineKind::Other(own),
    }
}


pub fn 建val修饰(段: &[Token]) -> (bool, bool) {
    let mut i = 0usize;
    if matches!(段.first(), Some(Token::Kw("add"))) {
        i = 1;
    }
    let share = matches!(段.get(i), Some(Token::Kw("share")));
    if share {
        i += 1;
    }
    if !matches!(段.get(i), Some(Token::Kw("val")) | Some(Token::Kw("fix"))) {
        return (false, false);
    }
    let send = matches!(段.get(i + 1), Some(Token::Sym("(")))
        && matches!(段.get(i + 2), Some(Token::Kw("send")))
        && matches!(段.get(i + 3), Some(Token::Sym(")")));
    (share, send)
}

pub fn 建val规范(段: &[Token]) -> Vec<Token> {
    let (out, _) = 建val规范带位(段, None);
    out
}

pub fn 建val规范流(段: &TokStream) -> TokStream {
    let (out, idx) = 建val规范带位(段, Some(段.spans()));
    let spans = idx.unwrap_or_else(|| vec![(0, 0); out.len()]);
    let 三元组 = out
        .into_iter()
        .zip(spans)
        .map(|(t, (c, l))| (t, c, l))
        .collect();
    TokStream::new(三元组)
}

fn 建val规范带位(段: &[Token], 源位: Option<&[(usize, usize)]>) -> (Vec<Token>, Option<Vec<(usize, usize)>>) {
    let mut out: Vec<Token> = Vec::with_capacity(段.len());
    let mut keep: Vec<usize> = Vec::with_capacity(段.len());
    let mut i = 0usize;
    let mut push = |k: usize, out: &mut Vec<Token>, keep: &mut Vec<usize>| {
        out.push(段[k].clone());
        keep.push(k);
    };
    if matches!(段.first(), Some(Token::Kw("add"))) {
        push(0, &mut out, &mut keep);
        i = 1;
    }
    if matches!(段.get(i), Some(Token::Kw("share"))) {
        i += 1;
    }
    if matches!(段.get(i), Some(Token::Kw("val")) | Some(Token::Kw("fix"))) {
        push(i, &mut out, &mut keep);
        i += 1;
    }
    if matches!(段.get(i), Some(Token::Sym("(")))
        && matches!(段.get(i + 1), Some(Token::Kw("send")))
        && matches!(段.get(i + 2), Some(Token::Sym(")")))
    {
        i += 3;
    }
    while i < 段.len() {
        push(i, &mut out, &mut keep);
        i += 1;
    }
    let spans = 源位.map(|sp| keep.iter().map(|&k| sp.get(k).copied().unwrap_or((0, 0))).collect());
    (out, spans)
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructKind {
    If,
    Else,
    While,
    Match,
    For,
    
    Case,
    Spawn,
}


pub fn struct_kind(head: &[Token]) -> StructKind {
    if head.iter().any(|t| matches!(t, Token::Kw("spawn"))) {
        return StructKind::Spawn;
    }
    match head.first() {
        Some(Token::Kw("else")) => StructKind::Else,
        Some(Token::Kw("while")) => StructKind::While,
        Some(Token::Kw("match")) => StructKind::Match,
        Some(Token::Kw("for")) => StructKind::For,
        Some(Token::Kw("case")) => StructKind::Case,
        _ => StructKind::If,
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub name: String,
    
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub stmts: Vec<Stmt>,
    
    pub attrs: Vec<String>,
    
    pub asm_lines: Vec<String>,
    
    pub module: String,
    
    pub file: String,
}


#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub blocks: Vec<Block>,
}


enum Kind {
    Block,
    Struct,
    Fn,
}

struct Frame {
    kind: Kind,
    name: String,
    head: TokStream,
    body: Vec<Stmt>,
    line: usize,
    col: usize,
    len: usize,
    
    attrs: Vec<String>,
    
    asm_lines: Vec<String>,
}





fn is_block_line(t: &[Token]) -> Option<(String, bool, Vec<String>)> {
    if t.first() != Some(&Token::HashColon) {
        return None;
    }
    let name = match t.get(1) {
        Some(Token::Ident(n)) | Some(Token::Str(n)) => n.to_string(),
        _ => return None,
    };
    let (open, rest) = match t.get(2) {
        Some(Token::Ident(w)) if w == "b" => (true, &t[3..]),
        Some(Token::Ident(w)) if w == "d" => (false, &t[3..]),
        _ => return None,
    };
    let mut attrs: Vec<String> = Vec::new();
    for a in rest {
        match a {
            Token::Kw(k) if *k == "unsafe" || *k == "pub" || *k == "sle" || *k == "ABI" || *k == "asm" => {
                attrs.push((*k).to_string())
            }
            _ => return None, 
        }
    }
    Some((name, open, attrs))
}


fn is_fn_head(t: &[Token]) -> bool {
    match (t.first(), t.get(1), t.get(2)) {
        (Some(Token::Kw("fn")), _, _) => true,
        (Some(Token::Kw("add")), Some(Token::Kw("fn")), _) => true,
        (Some(Token::Kw("add")), Some(Token::Kw("hide")), Some(Token::Kw("fn"))) => true,
        _ => false,
    }
}


fn is_struct_head(t: &[Token]) -> bool {
    matches!(t.last(), Some(Token::Sym(":")))
}


fn is_semi_line(t: &[Token]) -> bool {
    t.len() == 1 && t[0] == Token::Sym(";")
}


pub fn parse(spanned: Vec<Spanned>) -> Result<Program, ParseError> {
    let (ast, errs) = parse_all(spanned);
    match errs.into_iter().next() {
        Some(e) => Err(e),
        None => Ok(ast),
    }
}


pub fn parse_all(spanned: Vec<Spanned>) -> (Program, Vec<ParseError>) {
    let mut errs: Vec<ParseError> = Vec::new();
    
    macro_rules! 报错 {
        ($e:expr) => {{
            errs.push($e);
            continue;
        }};
    }
    let mut blocks: Vec<Block> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut after_semi = false; 

    
    
    let mut lines: Vec<(Span, TokStream)> = Vec::new();
    let mut cur: Vec<(Token, usize, usize)> = Vec::new();
    let mut cur_span = Span::default();
    for sp in spanned {
        if sp.tok == Token::Newline {
            if !cur.is_empty() {
                lines.push((cur_span, TokStream::new(std::mem::take(&mut cur))));
            }
            continue;
        }
        if cur.is_empty() {
            cur_span = sp.span();
        } else {
            cur_span.len = sp.col + sp.len - cur_span.col;
        }
        cur.push((sp.tok, sp.col, sp.len));
    }
    if !cur.is_empty() {
        lines.push((cur_span, TokStream::new(cur)));
    }

    for (span, toks) in lines {
        let line_no = span.line;
        if toks.is_empty() {
            continue;
        }

        
        if let Some((name, true, attrs)) = is_block_line(&toks) {

            if let Some(top) = stack.last() {
                if matches!(top.kind, Kind::Block) {
                    报错!(ParseError::with_code(
                        line_no,
                        ParseCode::E104,
                        &DiagParts::full(
                            format!("第 {} 行", line_no),
                            "开块",
                            format!("块「{}」里又开了块「{}」——块是平的，不许嵌套", top.name, name),
                            "",
                            "把内块移出来平铺：先 `#: 名 d` 关掉当前块，再开新块",
                        ),
                        Tone::Fix,
                    ));
                }
            }

            stack.push(Frame { kind: Kind::Block, name, head: TokStream::new(Vec::new()), body: Vec::new(), line: line_no, col: span.col, len: span.len, attrs, asm_lines: Vec::new() });
            after_semi = false;
            continue;
        }

        
        if let Some((name, false, close_attrs)) = is_block_line(&toks) {
        
            if let Some(top) = stack.last() {
                if matches!(top.kind, Kind::Struct | Kind::Fn) {
                    let l = top.line;
                    报错!(ParseError::with_code(
                        l,
                        ParseCode::E101,
                        &DiagParts::full(
                            format!("第 {} 行", l),
                            "开体",
                            "体没收束就关块了",
                            "缺独占一行的 ;",
                            "结构行开体后，用独占一行的 ; 收体，再 #: 名 d 关块",
                        ),
                        Tone::Fix,
                    ));
                }
            }
            let Some(f) = stack.pop() else {
                报错!(ParseError::with_code(
                    line_no,
                    ParseCode::E104,
                    &DiagParts::full(
                        format!("第 {} 行", line_no),
                        "关块",
                        "没有对应的开块",
                        name.clone(),
                        "检查 #: 名 b / d 是否成对",
                    ),
                    Tone::Fix,
                ))
            };
            if !matches!(f.kind, Kind::Block) || f.name != name {
                报错!(ParseError::with_code(
                    line_no,
                    ParseCode::E104,
                    &DiagParts::full(
                        format!("第 {} 行", line_no),
                        "关块",
                        "关块对不上",
                        format!("开的是「{}」，关的是「{}」", f.name, name),
                        "块名要相同，且先收完体内的体",
                    ),
                    Tone::Fix,
                ));
            }

            let mut all_attrs = f.attrs;
            all_attrs.extend(close_attrs.iter().cloned());
            blocks.push(Block {
                name: f.name,
                line: f.line,
                col: f.col,
                len: f.len,
                stmts: f.body,
                attrs: all_attrs,
                asm_lines: f.asm_lines,
                module: String::new(),
                file: String::new(),
            });
            after_semi = false;
            continue;
        }

        
        if is_semi_line(&toks) {
            let Some(f) = stack.pop() else {
                报错!(ParseError::with_code(
                    line_no,
                    ParseCode::E102,
                    &DiagParts::full(
                        format!("第 {} 行", line_no),
                        "收束",
                        "没有可收的体",
                        "这里多了一个 ;",
                        "只有结构行（以 : 结尾）才需要 ; 收束",
                    ),
                    Tone::Fix,
                ))
            };
            if !matches!(f.kind, Kind::Struct | Kind::Fn) {
                报错!(ParseError::with_code(
                    line_no,
                    ParseCode::E102,
                    &DiagParts::full(
                        format!("第 {} 行", line_no),
                        "收束",
                        "块体不需要 ;",
                        "块界由 #: 名 b / d 定",
                        "去掉这个 ;",
                    ),
                    Tone::Fix,
                ));
            }
            if f.body.is_empty() {
                报错!(ParseError::with_code(
                    line_no,
                    ParseCode::E103,
                    &DiagParts::full(
                        format!("第 {} 行", line_no),
                        "开体",
                        "体里是空的",
                        "开体后紧接着收束",
                        "体里至少写一条语句，或删掉这个结构行",
                    ),
                    Tone::Fix,
                ));
            }
            let st = match f.kind {
                Kind::Fn => Stmt::Fn { line: f.line, col: f.col, len: f.len, head: f.head, body: f.body },
                _ => Stmt::Struct { line: f.line, col: f.col, len: f.len, kind: struct_kind(&f.head), head: f.head, body: f.body },
            };
            match stack.last_mut() {
                Some(p) => p.body.push(st),
                None => {
                    报错!(ParseError::with_code(
                        line_no,
                        ParseCode::E104,
                        &DiagParts::full(
                            format!("第 {} 行", line_no),
                            "收束",
                            "收束到了块外",
                            "体不在块内",
                            "代码须写在 #: 名 b … d 之间",
                        ),
                        Tone::Fix,
                    ))
                }
            }
            after_semi = true;
            continue;
        }

        
        if is_struct_head(&toks) {
            if let Some(Token::Kw("else")) = toks.first() {
                if !after_semi {
                    报错!(ParseError::with_code(
                        line_no,
                        ParseCode::E105,
                        &DiagParts::full(
                            format!("第 {} 行", line_no),
                            "写 else",
                            "else 没紧跟上一个体的收束",
                            "中间夹了别的内容",
                            "else: 必须紧跟其 if 体的 ; 之后（中间只允许注释 / 空行）",
                        ),
                        Tone::Fix,
                    ));
                }
            }
            let kind = if is_fn_head(&toks) { Kind::Fn } else { Kind::Struct };
            stack.push(Frame { kind, name: String::new(), head: toks, body: Vec::new(), line: line_no, col: span.col, len: span.len, attrs: Vec::new(), asm_lines: Vec::new() });
            after_semi = false;
            continue;
        }

        
        if let [Token::RawAsm(s)] = &toks[..] {
            match stack.last_mut() {
                Some(p) if matches!(p.kind, Kind::Block) => p.asm_lines.push(s.to_string()),
                Some(_) => {
                    报错!(ParseError::with_code(
                        line_no,
                        ParseCode::E101,
                        &DiagParts::full(
                            format!("第 {} 行", line_no),
                            "写 asm 原文",
                            "asm 原文行跑进了结构体",
                            "它只能在 asm 块体里",
                            "把原文行移进 `#: 名 b asm` … `#: 名 d` 之间",
                        ),
                        Tone::Fix,
                    ));
                }
                None => {
                    报错!(ParseError::with_code(
                        line_no,
                        ParseCode::E104,
                        &DiagParts::full(
                            format!("第 {} 行", line_no),
                            "写 asm 原文",
                            "asm 原文行不在块内",
                            "上面没有开着的 asm 块",
                            "先把块开起来：`#: 名 b asm`",
                        ),
                        Tone::Fix,
                    ));
                }
            }
            after_semi = false;
            continue;
        }

        
        match stack.last_mut() {
            Some(p) => {
                let exprs = 行表达式(&toks);
                let kind = line_kind(&toks);
                p.body.push(Stmt::Line { line: line_no, col: span.col, len: span.len, kind, exprs })
            }
            None => {
                报错!(ParseError::with_code(
                    line_no,
                    ParseCode::E104,
                    &DiagParts::full(
                        format!("第 {} 行", line_no),
                        "写语句",
                        "语句不在块内",
                        "代码须在块内",
                        "把语句放进 #: 名 b … d 之间",
                    ),
                    Tone::Fix,
                ))
            }
        }
        after_semi = false;
    }


    while let Some(f) = stack.pop() {
        errs.push(match f.kind {
            Kind::Struct | Kind::Fn => ParseError::with_code(
                f.line,
                ParseCode::E101,
                &DiagParts::full(
                    format!("第 {} 行", f.line),
                    "开体",
                    "体没有收束",
                    "缺独占一行的 ;",
                    "结构行开体后，用独占一行的 ; 收体",
                ),
                Tone::Fix,
            ),
            Kind::Block => {
                blocks.push(Block {
                    name: f.name.clone(),
                    line: f.line,
                    col: f.col,
                    len: f.len,
                    stmts: f.body,
                    attrs: f.attrs,
                    asm_lines: f.asm_lines,
                    module: String::new(),
                    file: String::new(),
                });
                ParseError::with_code(
                    f.line,
                    ParseCode::E104,
                    &DiagParts::full(
                        format!("第 {} 行", f.line),
                        "开块",
                        "块没有关",
                        format!("块「{}」缺 #: {} d", f.name, f.name),
                        "补上收尾的 #: 名 d",
                    ),
                    Tone::Fix,
                )
                
                .延后()
            }
        });
    }

    (Program { blocks }, errs)
}