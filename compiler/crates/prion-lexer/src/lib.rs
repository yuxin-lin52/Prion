











use prion_error::{render_lex, DiagParts, LexCode, Tone};

pub mod asm;


#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub line: usize,
    pub message: String,
    pub code: String,
    
    pub parts: DiagParts,
}

impl LexError {
    pub fn with_code(line: usize, code: LexCode, parts: &DiagParts, tone: Tone) -> Self {
        Self {
            line,
            message: render_lex(code, parts, tone),
            code: code.number().to_string(),
            parts: parts.clone(),
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(pub std::rc::Rc<str>);

impl std::ops::Deref for Name {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        &*self.0 == other.as_str()
    }
}

impl std::borrow::Borrow<str> for Name {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Default for Name {
    fn default() -> Self {
        Name("".into())
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Name(s.into())
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Name(s.into())
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    HashColon,         
    HashHashColon,     
    HashHashHashColon, 
    Kw(&'static str),  
    Ident(Name),       
    Str(Name),         
    RawStr(Name),      
    Int(i64),
    Float(f64),
    Sym(&'static str), 
    RawAsm(Name),      
    Newline,           
}


#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub tok: Token,
    pub line: usize,
    
    pub col: usize,
    
    pub len: usize,
}

impl Spanned {
    pub fn span(&self) -> prion_error::Span {
        prion_error::Span { line: self.line, col: self.col, len: self.len }
    }
}




const KEYWORDS: &[&str] = &[
    
    "add", "call", "take", "fn", "val", "type", "hide", "give",
    
    "outs",
    
    "if", "else", "for", "while",
    
    "match", "case",
    
    "unsafe", "pub", "sle", "ABI", "asm",
    
    "fix",
    
    "trait", "impl", "spawn", "share", "send",
    
    "int", "float", "bool", "str", "int_8", "int_16", "int_32", "int_64", "uint_8", "uint_16",
    "uint_32", "uint_64",
    
    "list", "map", "que", "stack",
    
    "and", "or", "not", "null",
    
    "re", "fa",
];


const SYMBOLS2: &[&str] = &["==", "!=", "<=", ">=", "<<", ">>"];

const SYMBOLS1: &[&str] = &[
    "=", ":", ";", "~", ",", ".", "(", ")", "[", "]", "{", "}", "+", "-", "*", "/", "%", "<", ">",
    "?", "&", "|", "^", "!",
];

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}
fn is_ident_cont(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}


fn is_comment_line(line: &str) -> bool {
    line.trim_start().starts_with("//")
}


fn asm_open_block(line: &str) -> Option<String> {
    let t = line.trim_start();
    if !t.starts_with("#:") {
        return None;
    }
    let w: Vec<&str> = t[2..].split_whitespace().collect();
    if w.len() < 3 || w[1] != "b" || !w[2..].iter().any(|x| *x == "asm") {
        return None;
    }
    Some(w[0].to_string())
}


fn is_asm_close_line(line: &str, name: &str) -> bool {
    let t = line.trim_start();
    if !t.starts_with("#:") {
        return false;
    }
    let w: Vec<&str> = t[2..].split_whitespace().collect();
    w.len() >= 2 && w[0] == name && w[1] == "d"
}


pub fn lex(src: &str) -> Result<Vec<Spanned>, LexError> {
    let mut out = Vec::new();
    let mut lines: Vec<&str> = src.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop(); 
    }

    let mut asm_blk: Option<String> = None;
    for (idx, raw) in lines.into_iter().enumerate() {
        let line_no = idx + 1;
        
        if let Some(name) = asm_blk.clone() {
            if is_asm_close_line(raw, &name) {
                asm_blk = None; 
            } else {
                out.push(Spanned { tok: Token::RawAsm(raw.to_string().into()), line: line_no, col: 1, len: raw.chars().count() });
                out.push(Spanned { tok: Token::Newline, line: line_no, col: raw.chars().count() + 1, len: 0 });
                continue;
            }
        }
        
        if asm_blk.is_none() {
            if let Some(n) = asm_open_block(raw) {
                asm_blk = Some(n);
            }
        }
        
        if raw.contains('\r') {
            return Err(LexError::with_code(
                line_no,
                LexCode::E006,
                &DiagParts::full(
                    format!("第 {} 行", line_no),
                    "读源文件",
                    "发现 CR 字符",
                    "CRLF 换行不合契约（须纯 LF）",
                    "把文件转成纯 LF 换行",
                ),
                Tone::Fix,
            ));
        }
        let line = raw;

        
        if is_comment_line(line) {
            out.push(Spanned { tok: Token::Newline, line: line_no, col: line.chars().count() + 1, len: 0 });
            continue;
        }

        lex_line(line, line_no, &mut out)?;
    }

    Ok(out)
}


fn lex_line(line: &str, line_no: usize, out: &mut Vec<Spanned>) -> Result<(), LexError> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];

        
        if c == ' ' || c == '\t' {
            i += 1;
            continue;
        }

        
        if c == '"' {
            let (s, ni) = read_string(&chars, i, line_no, false)?;
            out.push(Spanned { tok: Token::Str(s.into()), line: line_no, col: i + 1, len: ni - i });
            i = ni;
            continue;
        }

        
        if c == '@' {
            if chars.get(i + 1) == Some(&'"') {
                let (s, ni) = read_string(&chars, i + 1, line_no, true)?;
                out.push(Spanned { tok: Token::RawStr(s.into()), line: line_no, col: i + 1, len: ni - i });
                i = ni;
                continue;
            }
            return Err(LexError::with_code(
                line_no,
                LexCode::E005,
                &DiagParts::full(
                    format!("第 {} 行", line_no),
                    "写原样串",
                    "@ 后不是引号",
                    "@ 只能接双引号",
                    "写成 @\"…\"（原样字符串）",
                ),
                Tone::Suggest,
            ));
        }

        
        if c.is_ascii_digit() {
            let (tok, ni) = read_number(&chars, i, line_no)?;
            out.push(Spanned { tok, line: line_no, col: i + 1, len: ni - i });
            i = ni;
            continue;
        }

        
        if is_ident_start(c) {
            let mut j = i;
            while j < chars.len() && is_ident_cont(chars[j]) {
                j += 1;
            }
            let w: String = chars[i..j].iter().collect();
            let tok = match KEYWORDS.iter().find(|k| **k == w) {
            
                Some(kw) => Token::Kw(kw),
                None => Token::Ident(w.into()),
            };
            out.push(Spanned { tok, line: line_no, col: i + 1, len: j - i });
            i = j;
            continue;
        }

        
        if c == '#' {
            let rest: String = chars[i..].iter().collect();
            if rest.starts_with("###:") {
                out.push(Spanned { tok: Token::HashHashHashColon, line: line_no, col: i + 1, len: 4 });
                i += 4;
                continue;
            }
            if rest.starts_with("##:") {
                out.push(Spanned { tok: Token::HashHashColon, line: line_no, col: i + 1, len: 3 });
                i += 3;
                continue;
            }
            if rest.starts_with("#:") {
                out.push(Spanned { tok: Token::HashColon, line: line_no, col: i + 1, len: 2 });
                i += 2;
                continue;
            }
            return Err(unexpected_char(line_no, c));
        }

        
        if c == ';' {
            if chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                return Err(LexError::with_code(
                    line_no,
                    LexCode::E001,
                    &DiagParts::full(
                        format!("第 {} 行", line_no),
                        "写层级标记",
                        "行首 ;数字",
                        ";数字 深度前缀已废止",
                        "层级改用「; 收束」：结构行冒号开体、独占一行 ; 收体",
                    ),
                    Tone::Fix,
                ));
            }
            out.push(Spanned { tok: Token::Sym(";"), line: line_no, col: i + 1, len: 1 });
            i += 1;
            continue;
        }


        if i + 1 < chars.len() {
            let two: String = chars[i..i + 2].iter().collect();
            if let Some(s) = SYMBOLS2.iter().find(|s| **s == two) {
                out.push(Spanned { tok: Token::Sym(s), line: line_no, col: i + 1, len: 2 });
                i += 2;
                continue;
            }
        }

        
        let one = c.to_string();
        if let Some(s) = SYMBOLS1.iter().find(|s| **s == one) {
            out.push(Spanned { tok: Token::Sym(s), line: line_no, col: i + 1, len: 1 });
            i += 1;
            continue;
        }

        return Err(unexpected_char(line_no, c));
    }

    out.push(Spanned { tok: Token::Newline, line: line_no, col: chars.len() + 1, len: 0 });
    Ok(())
}

fn unexpected_char(line_no: usize, c: char) -> LexError {
    LexError::with_code(
        line_no,
        LexCode::E001,
        &DiagParts::full(
            format!("第 {} 行", line_no),
            "解析",
            format!("不认识的字符「{}」", c),
            "不在 Prion 的词法表里",
            "检查拼写（中文近形字：报告 ≠ 报吿）或改用合法符号",
        ),
        Tone::Suggest,
    )
}


fn read_string(
    chars: &[char],
    quote_at: usize,
    line_no: usize,
    raw: bool,
) -> Result<(String, usize), LexError> {
    let mut i = quote_at + 1;
    let mut s = String::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
        
            if raw && chars.get(i + 1) == Some(&'"') {
                s.push('"');
                i += 2;
                continue;
            }
            return Ok((s, i + 1));
        }
        if !raw && c == '\\' {
            let e = chars.get(i + 1).copied().ok_or_else(|| {
                LexError::with_code(
                    line_no,
                    LexCode::E004,
                    &DiagParts::full(
                        format!("第 {} 行", line_no),
                        "写字符串",
                        "转义符后无字符",
                        "\\ 悬空",
                        "转义只支持 \\n \\t \\\\ \\\"",
                    ),
                    Tone::Suggest,
                )
            })?;
            s.push(match e {
                'n' => '\n',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                _ => {
                    return Err(LexError::with_code(
                        line_no,
                        LexCode::E004,
                        &DiagParts::full(
                            format!("第 {} 行", line_no),
                            "写字符串",
                            format!("不支持的转义 \\{}", e),
                            "转义只有四种",
                            "只写 \\n \\t \\\\ \\\"（原样串用 @\"…\"）",
                        ),
                        Tone::Suggest,
                    ))
                }
            });
            i += 2;
            continue;
        }
        s.push(c);
        i += 1;
    }
    Err(LexError::with_code(
        line_no,
        LexCode::E004,
        &DiagParts::full(
            format!("第 {} 行", line_no),
            "写字符串",
            "缺收尾双引号",
            "字符串没闭合",
            "补上收尾的 \"",
        ),
        Tone::Suggest,
    ))
}


fn read_number(chars: &[char], start: usize, line_no: usize) -> Result<(Token, usize), LexError> {
    let mut i = start;
    
    if chars[i] == '0' {
        if let Some(&p) = chars.get(i + 1) {
            let radix = match p {
                'x' | 'X' => Some(16),
                'b' | 'B' => Some(2),
                'o' | 'O' => Some(8),
                _ => None,
            };
            if let Some(radix) = radix {
                i += 2;
                let mut digits = String::new();
                while i < chars.len() && (chars[i] == '_' || chars[i].is_digit(radix)) {
                    if chars[i] != '_' {
                        digits.push(chars[i]);
                    }
                    i += 1;
                }
                let v = i64::from_str_radix(&digits, radix).map_err(|_| num_err(line_no))?;
                return Ok((Token::Int(v), i));
            }
        }
    }

    
    let mut text = String::new();
    while i < chars.len() && (chars[i] == '_' || chars[i].is_ascii_digit()) {
        if chars[i] != '_' {
            text.push(chars[i]);
        }
        i += 1;
    }
    let mut is_float = false;
    
    if chars.get(i) == Some(&'.') && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
        is_float = true;
        text.push('.');
        i += 1;
        while i < chars.len() && (chars[i] == '_' || chars[i].is_ascii_digit()) {
            if chars[i] != '_' {
                text.push(chars[i]);
            }
            i += 1;
        }
    }
    
    if matches!(chars.get(i), Some('e') | Some('E')) {
        let mut j = i + 1;
        if matches!(chars.get(j), Some('+') | Some('-')) {
            j += 1;
        }
        if chars.get(j).is_some_and(|d| d.is_ascii_digit()) {
            is_float = true;
            text.push('e');
            
            if chars.get(i + 1) == Some(&'+') || chars.get(i + 1) == Some(&'-') {
                text.push(chars[i + 1]);
            }
            let mut k = j;
            while k < chars.len() && (chars[k] == '_' || chars[k].is_ascii_digit()) {
                if chars[k] != '_' {
                    text.push(chars[k]);
                }
                k += 1;
            }
            i = k;
        }
    }

    if is_float {
        let v: f64 = text.parse().map_err(|_| num_err(line_no))?;
        Ok((Token::Float(v), i))
    } else {
        let v: i64 = text.parse().map_err(|_| num_err(line_no))?;
        Ok((Token::Int(v), i))
    }
}

fn num_err(line_no: usize) -> LexError {
    LexError::with_code(
        line_no,
        LexCode::E003,
        &DiagParts::full(
            format!("第 {} 行", line_no),
            "写数字字面量",
            "数字格式不对",
            "无法解析",
            "检查进制前缀（0x/0b/0o）与下划线分隔",
        ),
        Tone::Suggest,
    )
}