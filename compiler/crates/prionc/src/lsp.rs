




use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};

use serde_json::{json, Value};


struct 状态 {
    docs: BTreeMap<String, String>,
    已保存: BTreeMap<String, bool>,
    缓存: BTreeMap<String, 分析>,
    关闭: bool,
}


struct 分析 {
    源: String,
    语法错: Vec<prion_parser::ParseError>,
    检查错: Vec<prion_check::Diag>,
    符号: Vec<prion_check::SymbolInfo>,
    
    原件: Vec<(usize, usize, usize, u32)>,
    
    局部: Vec<prion_check::LocalInfo>,
    
    域: Vec<(String, usize, usize)>,
}


fn 编码(原件: &[(usize, usize, usize, u32)], 起行: usize, 止行: usize) -> Vec<u32> {
    let mut 结果: Vec<u32> = Vec::new();
    
    let mut 前行 = 0usize;
    let mut 前列 = 0usize;
    for &(行, 列, 长, 类) in 原件 {
        if 行 < 起行 || 行 > 止行 {
            continue;
        }
        let d行 = 行 - 前行;
        let d列 = if d行 == 0 { 列.saturating_sub(前列) } else { 列 };
        结果.extend_from_slice(&[d行 as u32, d列 as u32, 长 as u32, 类, 0]);
        前行 = 行;
        前列 = 列;
    }
    结果
}


pub fn 服务() -> i32 {
    let stdin = io::stdin();
    let mut 读 = stdin.lock();
    let stdout = io::stdout();
    let mut 写 = stdout.lock();
    let mut 态 = 状态 { docs: BTreeMap::new(), 已保存: BTreeMap::new(), 缓存: BTreeMap::new(), 关闭: false };
    loop {
        let 报文 = match 读一帧(&mut 读) {
            Ok(Some(v)) => v,
            Ok(None) => break,
            Err(_) => break,
        };
        let 方法 = 报文.get("method").and_then(|m| m.as_str()).unwrap_or("").to_string();
        let id = 报文.get("id").cloned();
        let 参数 = 报文.get("params").cloned().unwrap_or(Value::Null);
        let mut 已回 = false;
        match 方法.as_str() {
            "initialize" => {
                if let Some(id) = id.clone() {
                    回(&mut 写, json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "capabilities": {
                            
                            "textDocumentSync": 1,
                            
                            "diagnosticProvider": Value::Null,
                            
                            "semanticTokensProvider": {
                                "legend": { "tokenTypes": 图例, "tokenModifiers": [] },
                                "full": true, "range": true
                            },
                            
                            "completionProvider": { "triggerCharacters": ["\"", " ", "."] },
                            
                            "documentFormattingProvider": true,
                            
                            "hoverProvider": true,
                            "definitionProvider": true,
                            
                            "renameProvider": { "prepareProvider": true },
                        } }
                    }));
                    已回 = true;
                }
            }
            "initialized" | "$/setTrace" | "workspace/didChangeConfiguration" => {}
            "textDocument/didOpen" => {
                if let Some((uri, text)) = 取文档(&参数, "textDocument", "text") {
                    态.docs.insert(uri.clone(), text);
                    态.已保存.insert(uri.clone(), false);
                    推诊断(&mut 写, &mut 态, &uri);
                }
            }
            "textDocument/didChange" => {
                if let Some((uri, text)) = 取变更(&参数) {
                    态.docs.insert(uri.clone(), text);
                    态.已保存.insert(uri.clone(), false);
                    推诊断(&mut 写, &mut 态, &uri);
                }
            }
            "textDocument/didSave" => {
                if let Some(uri) = 取uri(&参数) {
                    态.已保存.insert(uri.clone(), true);
                    推诊断(&mut 写, &mut 态, &uri);
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = 取uri(&参数) {
                    态.docs.remove(&uri);
                    态.缓存.remove(&uri);
                    回(&mut 写, json!({
                        "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                        "params": { "uri": uri, "diagnostics": [] }
                    }));
                }
            }
            "textDocument/semanticTokens/full" => {
                if let Some(id) = id.clone() {
                    let 数据 = 取语义(&mut 态, &参数);
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": { "data": 数据 } }));
                    已回 = true;
                }
            }
            "textDocument/semanticTokens/range" => {
                if let Some(id) = id.clone() {
                    let 数据 = 取语义(&mut 态, &参数);
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": { "data": 数据 } }));
                    已回 = true;
                }
            }
            "textDocument/completion" => {
                if let Some(id) = id.clone() {
                    let 项 = 取补全(&mut 态, &参数);
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id,
                        "result": { "isIncomplete": false, "items": 项 } }));
                    已回 = true;
                }
            }
            "textDocument/documentSymbol" => {
                if let Some(id) = id.clone() {
                    let 项 = 取大纲(&mut 态, &参数);
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": 项 }));
                    已回 = true;
                }
            }
            "textDocument/formatting" => {
                if let Some(id) = id.clone() {
                    let uri = 取uri(&参数).unwrap_or_default();
                    let 原文 = 态.docs.get(&uri).cloned().unwrap_or_default();
                    let 新文 = crate::fmt::format_source(&原文);
                    let 结果 = if 新文 == 原文 {
                        json!([])
                    } else {
                        let 末行 = 原文.matches('\n').count();
                        let 末列 = 原文.rsplit('\n').next().map(|s| s.chars().count()).unwrap_or(0);
                        json!([{
                            "range": {
                                "start": { "line": 0, "character": 0 },
                                "end": { "line": 末行, "character": 末列 }
                            },
                            "newText": 新文
                        }])
                    };
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": 结果 }));
                    已回 = true;
                }
            }
            "textDocument/hover" => {
                if let Some(id) = id.clone() {
                    let 果 = 取悬停(&mut 态, &参数);
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": 果 }));
                    已回 = true;
                }
            }
            "textDocument/definition" => {
                if let Some(id) = id.clone() {
                    let 果 = 取定义(&mut 态, &参数);
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": 果 }));
                    已回 = true;
                }
            }
            "textDocument/prepareRename" => {
                if let Some(id) = id.clone() {
                    let 果 = 取预备重命名(&mut 态, &参数);
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": 果 }));
                    已回 = true;
                }
            }
            "textDocument/rename" => {
                if let Some(id) = id.clone() {
                    let 新名 = 参数.get("newName").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let 果 = 取重命名(&mut 态, &参数, &新名);
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": 果 }));
                    已回 = true;
                }
            }
            "shutdown" => {
                if let Some(id) = id.clone() {
                    回(&mut 写, json!({ "jsonrpc": "2.0", "id": id, "result": Value::Null }));
                    已回 = true;
                }
            }
            "exit" => {
                态.关闭 = true;
            }
            _ => {
                
                if let Some(id) = id.clone() {
                    回(&mut 写, json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32601, "message": "方法未实现" }
                    }));
                    已回 = true;
                }
            }
        }
        let _ = 已回;
        if 态.关闭 {
            break;
        }
    }
    if 态.关闭 { 0 } else { 1 }
}


fn 读一帧(读: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut 长 = 0usize;
    let mut 有头 = false;
    loop {
        let mut 行 = String::new();
        let n = 读.read_line(&mut 行)?;
        if n == 0 {
            return Ok(None);
        }
        let 去空白 = 行.trim_end_matches(['\r', '\n']);
        if 去空白.is_empty() {
            break;
        }
        有头 = true;
        if let Some(v) = 去空白.strip_prefix("Content-Length:") {
            长 = v.trim().parse().unwrap_or(0);
        }
    }
    if !有头 || 长 == 0 {
        return Ok(None);
    }
    let mut 缓 = vec![0u8; 长];
    读.read_exact(&mut 缓)?;
    let 文本 = String::from_utf8_lossy(&缓).to_string();
    Ok(Some(serde_json::from_str(&文本).unwrap_or(Value::Null)))
}


fn 回(写: &mut impl Write, v: Value) {
    let 体 = v.to_string();
    let _ = write!(写, "Content-Length: {}\r\n\r\n{}", 体.len(), 体);
    let _ = 写.flush();
}


fn 取文档(参: &Value, 文档键: &str, 文本键: &str) -> Option<(String, String)> {
    let d = 参.get(文档键)?;
    let uri = d.get("uri")?.as_str()?.to_string();
    let text = d.get(文本键)?.as_str()?.to_string();
    Some((uri, text))
}


fn 取变更(参: &Value) -> Option<(String, String)> {
    let uri = 取uri(参)?;
    let 变更 = 参.get("contentChanges")?.as_array()?;
    let 末 = 变更.last()?;
    let text = 末.get("text")?.as_str()?.to_string();
    Some((uri, text))
}


fn 取uri(参: &Value) -> Option<String> {
    Some(参.get("textDocument")?.get("uri")?.as_str()?.to_string())
}



fn 取分析<'a>(态: &'a mut 状态, uri: &str) -> Option<&'a 分析> {
    let 源 = 态.docs.get(uri)?.clone();
    let 要算 = match 态.缓存.get(uri) {
        Some(c) => c.源 != 源,
        None => true,
    };
    if 要算 {
        let Ok(段) = prion_lexer::lex(&源) else { return None };
        let (ast, 语法错) = prion_parser::parse_all(段);
        let (检查错, 符号, 局部) = if 语法错.is_empty() {
            let r = prion_check::check(&ast);
            (r.diags, r.symbols, r.locals)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        
        let 原件 = if ast.blocks.is_empty() {
            词法令牌(&prion_lexer::lex(&源).unwrap_or_default())
        } else {
            语义令牌(&ast, &符号)
        };
        
        let 域 = 参数域(&ast);
        态.缓存.insert(uri.to_string(), 分析 { 源, 语法错, 检查错, 符号, 原件, 局部, 域 });
    }
    态.缓存.get(uri)
}

fn 推诊断(写: &mut impl Write, 态: &mut 状态, uri: &str) {
    let mut 列表: Vec<Value> = Vec::new();
    let 已存 = *态.已保存.get(uri).unwrap_or(&false);
    if let Some(a) = 取分析(态, uri) {
        
        for e in &a.语法错 {
            if e.可延后 && !已存 {
                continue;
            }
            列表.push(一条(e.line, 1, 1, &e.message));
        }
        for d in &a.检查错 {
            if d.is_error {
                列表.push(一条(d.line, d.col, d.len, &d.message));
            }
        }
    }
    回(写, json!({
        "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
        "params": { "uri": uri, "diagnostics": 列表 }
    }));
}

fn 一条(行: usize, 列: usize, 长: usize, 文案: &str) -> Value {
    let l = 行.saturating_sub(1);
    let c = 列.saturating_sub(1);
    let w = if 长 == 0 { 1 } else { 长 };
    json!({
        "range": { "start": { "line": l, "character": c },
                   "end": { "line": l, "character": c + w } },
        "severity": 1,
        "source": "prionc",
        "message": 文案
    })
}


pub const 图例: [&str; 10] = [
    "keyword", "namespace", "type", "function", "variable", "parameter", "modifier", "string",
    "number", "method",
];


fn 关键字类(k: &str) -> u32 {
    match k {
        "if" | "else" | "while" | "for" | "match" | "case" | "call" | "give"
        | "add" | "kill" | "asm" | "unsafe" | "and" | "or" | "not" => 0,
        "fn" | "val" | "type" | "fix" => 0,
        "pub" | "sle" | "ABI" | "hide" | "outs" => 6,
        "int" | "float" | "bool" | "str" | "int_8" | "int_16" | "int_32" | "int_64"
        | "uint_8" | "uint_16" | "uint_32" | "uint_64" | "list" | "map" | "que" | "stack" => 2,
        "null" | "re" | "fa" => 0,
        _ => 0,
    }
}


fn 符号类(k: &prion_check::SymKind) -> u32 {
    use prion_check::SymKind as K;
    match k {
        K::Type | K::Variant => 2,
        K::Fn => 3,
        K::Val | K::Param => 4,
        K::Block => 1,
    }
}


fn 扫语句(
    stmts: &[prion_parser::Stmt],
    行: &mut Vec<(usize, usize, usize, u32)>,
    表: &std::collections::BTreeMap<String, u32>,
) {
    use prion_parser::Stmt as S;
    for s in stmts {
        match s {
            S::Line { line, kind, .. } => 扫一行(*line, kind.tokens(), 行, 表),
            S::Struct { line, head, body, .. } | S::Fn { line, head, body, .. } => {
                扫一行(*line, head, 行, 表);
                扫语句(body, 行, 表);
            }
        }
    }
}


fn 扫一行(
    行号: usize,
    段: &prion_parser::TokStream,
    出: &mut Vec<(usize, usize, usize, u32)>,
    表: &std::collections::BTreeMap<String, u32>,
) {
    use prion_lexer::Token as T;
    for i in 0..段.len() {
        let Some((t, (c, l))) = 段.get2(i) else { continue };
        if l == 0 {
            continue;
        }
        let ty = match t {
            T::Kw(k) => 关键字类(k),
            T::Int(_) | T::Float(_) => 8,
            T::Str(s) | T::RawStr(s) => *表.get(s.as_ref()).unwrap_or(&7),
            T::Ident(s) => {
                
                if prion_check::BUILTINS.iter().any(|u| *u == s.as_ref()) {
                    9
                } else {
                    *表.get(s.as_ref()).unwrap_or(&4)
                }
            }
            _ => continue,
        };
        出.push((行号.saturating_sub(1), c.saturating_sub(1), l, ty));
    }
}


fn 声明表(ast: &prion_parser::Program) -> std::collections::BTreeMap<String, u32> {
    use prion_lexer::Token as T;
    use prion_parser::Stmt as S;
    fn 扫(stmts: &[S], 表: &mut std::collections::BTreeMap<String, u32>) {
        for s in stmts {
            let (head, body): (&[prion_lexer::Token], &[S]) = match s {
                
                S::Line { kind, .. } => (kind.tokens(), &[]),
                S::Struct { head, body, .. } | S::Fn { head, body, .. } => (head, body),
            };
            
            let 类别 = match (head.first(), head.get(1)) {
                (Some(T::Kw("add")), Some(T::Kw("fn"))) => Some(3u32),
                (Some(T::Kw("add")), Some(T::Kw("val"))) => Some(4u32),
                (Some(T::Kw("add")), Some(T::Kw("type"))) => Some(2u32),
                _ => None,
            };
            if let (Some(c), Some(T::Str(n) | T::Ident(n))) = (类别, head.get(2)) {
                表.insert(n.to_string(), c);
            }
            扫(body, 表);
        }
    }
    let mut 表 = std::collections::BTreeMap::new();
    for b in &ast.blocks {
        扫(&b.stmts, &mut 表);
    }
    表
}


pub fn 语义令牌(ast: &prion_parser::Program, 符号: &[prion_check::SymbolInfo]) -> Vec<(usize, usize, usize, u32)> {
    let mut 表: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for s in 符号 {
        表.insert(s.name.clone(), 符号类(&s.kind));
    }
    
    if 表.is_empty() {
        表 = 声明表(ast);
    }
    
    for (n, _, _) in 参数域(ast) {
        表.entry(n).or_insert(4);
    }
    let mut 出: Vec<(usize, usize, usize, u32)> = Vec::new();
    for b in &ast.blocks {
        扫语句(&b.stmts, &mut 出, &表);
    }
    出.sort();
    出
}


fn 取语义(态: &mut 状态, 参: &Value) -> Vec<u32> {
    let Some(uri) = 取uri(参) else { return Vec::new() };
    
    let (起, 止) = match 参.get("range") {
        Some(r) => (
            r.get("start").and_then(|x| x.get("line")).and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            r.get("end").and_then(|x| x.get("line")).and_then(|x| x.as_u64()).unwrap_or(u64::MAX) as usize,
        ),
        None => (0, usize::MAX),
    };
    match 取分析(态, &uri) {
        Some(a) => 编码(&a.原件, 起, 止),
        None => Vec::new(),
    }
}

fn 词法令牌(段: &[prion_lexer::Spanned]) -> Vec<(usize, usize, usize, u32)> {
    use prion_lexer::Token as T;
    let mut 出: Vec<(usize, usize, usize, u32)> = Vec::new();
    for t in 段 {
        if t.len == 0 {
            continue;
        }
        let 类 = match &t.tok {
            T::Kw(k) => 关键字类(k),
            T::Int(_) | T::Float(_) => 8,
            T::Str(_) => 7,
            T::Ident(_) => 4,
            _ => continue,
        };
        出.push((t.line.saturating_sub(1), t.col.saturating_sub(1), t.len, 类));
    }
    出
}


fn 取补全(态: &mut 状态, 参: &Value) -> Vec<Value> {
    let Some(uri) = 取uri(参) else { return Vec::new() };
    let mut 名: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let Some(a) = 取分析(态, &uri) {
        for k in 关键字全集() {
            名.insert(k.to_string());
        }
        for s in &a.符号 {
            名.insert(s.name.clone());
        }
    }
    名.into_iter()
        .map(|n| json!({ "label": n, "kind": 6 }))
        .collect()
}

fn 关键字全集() -> &'static [&'static str] {
    &[
        "add", "call", "fn", "val", "type", "hide", "give", "outs", "if", "else", "for",
        "while", "match", "case", "unsafe", "pub", "sle", "ABI", "kill", "asm", "fix", "int",
        "float", "bool", "str", "int_8", "int_16", "int_32", "int_64", "uint_8", "uint_16",
        "uint_32", "uint_64", "list", "map", "que", "stack", "and", "or", "not", "null", "re", "fa",
    ]
}


fn 取大纲(态: &mut 状态, 参: &Value) -> Vec<Value> {
    let Some(uri) = 取uri(参) else { return Vec::new() };
    let Some(a) = 取分析(态, &uri) else { return Vec::new() };
    a.符号
        .iter()
        .map(|s| {
            let l = s.line.saturating_sub(1);
            let c = s.col.saturating_sub(1);
            json!({
                "name": s.name,
                "kind": 类别码(&s.kind),
                "range": { "start": { "line": l, "character": c },
                           "end": { "line": l, "character": c + s.len.max(1) } },
                "selectionRange": { "start": { "line": l, "character": c },
                                    "end": { "line": l, "character": c + s.len.max(1) } }
            })
        })
        .collect()
}

fn 取位置(参: &Value) -> (usize, usize) {
    let p = 参.get("position");
    let l = p.and_then(|x| x.get("line")).and_then(|x| x.as_u64()).unwrap_or(0) as usize;
    let c = p.and_then(|x| x.get("character")).and_then(|x| x.as_u64()).unwrap_or(0) as usize;
    (l, c)
}


fn 取词(a: &分析, 行: usize, 列: usize, 长: usize) -> Option<(String, usize, usize)> {
    let 文 = a.源.lines().nth(行)?;
    let 原: String = 文.chars().skip(列).take(长).collect();
    let 数 = 原.chars().count();
    
    let (跳, 取) = if 原.starts_with("@\"") && 数 >= 3 && 原.ends_with('"') {
        (2, 数 - 3)
    } else if 原.starts_with('"') && 数 >= 2 && 原.ends_with('"') {
        (1, 数 - 2)
    } else {
        (0, 数)
    };
    let 词: String = 原.chars().skip(跳).take(取).collect();
    if 词.is_empty() {
        return None;
    }
    Some((词, 列 + 跳, 取))
}


fn 找词(a: &分析, 行: usize, 列: usize) -> Option<(String, u32, usize, usize, usize)> {
    for &(l, c, n, 类) in &a.原件 {
        if l != 行 || 列 < c || 列 >= c + n {
            continue;
        }
        let (词, 起, 长) = 取词(a, l, c, n)?;
        return Some((词, 类, l, 起, 长));
    }
    None
}


fn 参数域(ast: &prion_parser::Program) -> Vec<(String, usize, usize)> {
    use prion_lexer::Token as T;
    use prion_parser::Stmt as S;
    
    fn 最大行(stmts: &[S]) -> usize {
        let mut 大 = 0;
        for s in stmts {
            let (行, 体) = match s {
                S::Line { line, .. } => (*line, None),
                S::Struct { line, body, .. } | S::Fn { line, body, .. } => (*line, Some(body)),
            };
            大 = 大.max(行);
            if let Some(体) = 体 {
                大 = 大.max(最大行(体));
            }
        }
        大
    }
    
    fn 收参(段: &[prion_lexer::Token], 起: usize, 止: usize, 出: &mut Vec<(String, usize, usize)>) {
        let mut 深 = 0i32;
        let mut 段起 = false;
        for t in 段.iter() {
            match t {
                T::Sym("(") => {
                    深 += 1;
                    
                    if 深 == 1 {
                        段起 = true;
                    }
                }
                T::Sym(")") => {
                    深 -= 1;
                    
                    if 深 == 0 {
                        break;
                    }
                }
                T::Sym(",") if 深 == 1 => 段起 = true,
                T::Ident(n) if 深 == 1 && 段起 => {
                    出.push((n.to_string(), 起, 止));
                    段起 = false;
                }
                _ => {}
            }
        }
    }
    
    fn 走(stmts: &[S], 出: &mut Vec<(String, usize, usize)>) {
        for s in stmts {
            match s {
                S::Fn { line, head, body, .. } => {
                    let 止 = 最大行(body).max(*line);
                    收参(head, *line, 止, 出);
                    走(body, 出);
                }
                S::Struct { body, .. } => 走(body, 出),
                
                S::Line { line, kind, .. } => {
                    let 段 = kind.tokens();
                    if matches!(段.first(), Some(T::Kw("add")) | Some(T::Kw("fix")))
                        && matches!(段.get(1), Some(T::Kw("fn")))
                    {
                        收参(段, *line, *line, 出);
                    }
                }
            }
        }
    }
    let mut 出 = Vec::new();
    for b in &ast.blocks {
        走(&b.stmts, &mut 出);
    }
    出
}


fn 找域(a: &分析, 词: &str, 行: usize) -> Option<(usize, usize)> {
    let 行1 = 行 + 1;
    a.域
        .iter()
        .filter(|(n, s, e)| n == 词 && *s <= 行1 && 行1 <= *e)
        .min_by_key(|(_, s, e)| e - s)
        .map(|(_, s, e)| (s.saturating_sub(1), e.saturating_sub(1)))
}


fn 选符号<'a>(a: &'a 分析, 词: &str) -> Option<&'a prion_check::SymbolInfo> {
    use prion_check::SymKind as K;
    a.符号
        .iter()
        .find(|s| s.name == 词 && !matches!(s.kind, K::Param))
        .or_else(|| a.符号.iter().find(|s| s.name == 词))
}


fn 种类名(k: &prion_check::SymKind) -> &'static str {
    use prion_check::SymKind as K;
    match k {
        K::Type => "类型",
        K::Variant => "变体",
        K::Fn => "函数",
        K::Val => "值",
        K::Param => "参数",
        K::Block => "块",
    }
}


fn 取悬停(态: &mut 状态, 参: &Value) -> Value {
    let Some(uri) = 取uri(参) else { return Value::Null };
    let (行, 列) = 取位置(参);
    let Some(a) = 取分析(态, &uri) else { return Value::Null };
    let Some((词, 类, l, c, n)) = 找词(a, 行, 列) else { return Value::Null };
    
    if 类 == 7 || 类 == 8 {
        return Value::Null;
    }
    let 值 = match 选符号(a, &词) {
        Some(s) => format!("**{}** `{}`\n\n定义于第 {} 行第 {} 列", 种类名(&s.kind), 词, s.line, s.col),
        
        None if 类 == 9 => format!("**内置函数** `{}`", 词),
        
        None => match a.局部.iter().find(|l| l.name == 词) {
            Some(l) => format!("**值** `{}` : {}", 词, l.ty),
            None => return Value::Null,
        },
    };
    json!({
        "contents": { "kind": "markdown", "value": 值 },
        "range": { "start": { "line": l, "character": c },
                   "end": { "line": l, "character": c + n } }
    })
}


fn 取定义(态: &mut 状态, 参: &Value) -> Value {
    let Some(uri) = 取uri(参) else { return Value::Null };
    let (行, 列) = 取位置(参);
    let Some(a) = 取分析(态, &uri) else { return Value::Null };
    let Some((词, 类, ..)) = 找词(a, 行, 列) else { return Value::Null };
    
    if 类 == 9 || 类 == 7 || 类 == 8 {
        return Value::Null;
    }
    let (sl, sc, sn) = match 选符号(a, &词) {
        Some(s) => (s.line.saturating_sub(1), s.col.saturating_sub(1), s.len.max(1)),
        None => {
            
            let Some((起, 止)) = 找域(a, &词, 行) else { return Value::Null };
            let mut 首: Option<(usize, usize, usize)> = None;
            for &(l, c, n, k) in &a.原件 {
                if k != 4 || l < 起 || l > 止 {
                    continue;
                }
                let Some((本, 起列, 长)) = 取词(a, l, c, n) else { continue };
                if 本 != 词 {
                    continue;
                }
                if 首.map_or(true, |旧| (l, 起列) < (旧.0, 旧.1)) {
                    首 = Some((l, 起列, 长));
                }
            }
            let Some(首) = 首 else { return Value::Null };
            首
        }
    };
    json!({
        "uri": uri,
        "range": { "start": { "line": sl, "character": sc },
                   "end": { "line": sl, "character": sc + sn } }
    })
}


fn 可改(类: u32) -> bool {
    (1..=4).contains(&类)
}


fn 取预备重命名(态: &mut 状态, 参: &Value) -> Value {
    let Some(uri) = 取uri(参) else { return Value::Null };
    let (行, 列) = 取位置(参);
    let Some(a) = 取分析(态, &uri) else { return Value::Null };
    let Some((词, 类, l, c, n)) = 找词(a, 行, 列) else { return Value::Null };
    let 可 = 选符号(a, &词).is_some() || (类 == 4 && 找域(a, &词, 行).is_some());
    if !可改(类) || !可 {
        return Value::Null;
    }
    json!({
        "range": { "start": { "line": l, "character": c },
                   "end": { "line": l, "character": c + n } },
        "placeholder": 词
    })
}


fn 取重命名(态: &mut 状态, 参: &Value, 新名: &str) -> Value {
    let Some(uri) = 取uri(参) else { return Value::Null };
    let (行, 列) = 取位置(参);
    let Some(a) = 取分析(态, &uri) else { return Value::Null };
    let Some((词, 类, ..)) = 找词(a, 行, 列) else { return Value::Null };
    let 域 = if 类 == 4 { 找域(a, &词, 行) } else { None };
    let 可 = 选符号(a, &词).is_some() || 域.is_some();
    if !可改(类) || 新名.is_empty() || !可 {
        return Value::Null;
    }
    let mut 编辑: Vec<Value> = Vec::new();
    for &(l, c, n, 本类) in &a.原件 {
        if !可改(本类) {
            continue;
        }
        
        if let Some((起, 止)) = 域 {
            if l < 起 || l > 止 {
                continue;
            }
        }
        let Some((本, 起, 长)) = 取词(a, l, c, n) else { continue };
        if 本 != 词 {
            continue;
        }
        编辑.push(json!({
            "range": { "start": { "line": l, "character": 起 },
                       "end": { "line": l, "character": 起 + 长 } },
            "newText": 新名
        }));
    }
    if 编辑.is_empty() {
        return Value::Null;
    }
    json!({ "changes": { uri: 编辑 } })
}


fn 类别码(k: &prion_check::SymKind) -> u32 {
    use prion_check::SymKind as K;
    match k {
        K::Type => 5,
        K::Variant => 22,
        K::Fn => 12,
        K::Val => 13,
        K::Param => 13,
        K::Block => 3,
    }
}