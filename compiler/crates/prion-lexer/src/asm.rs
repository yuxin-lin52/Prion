





use crate::Token;


pub fn 是asm词(t: &Token) -> bool {
    match t {
        Token::Ident(n) => n == "asm",
        Token::Kw(k) => *k == "asm",
        _ => false,
    }
}


pub fn 寄存器约束(名: &str) -> Option<&'static str> {
    let 小写 = 名.to_ascii_lowercase();
    let s = 小写.as_str();
    Some(match s {
        "al" | "ax" | "eax" | "rax" => "a",
        "bl" | "bx" | "ebx" | "rbx" => "b",
        "cl" | "cx" | "ecx" | "rcx" => "c",
        "dl" | "dx" | "edx" | "rdx" => "d",
        "si" | "esi" | "rsi" => "S",
        "di" | "edi" | "rdi" => "D",
        "r8" | "r8d" | "r8w" | "r8b" => "r8",
        "r9" | "r9d" | "r9w" | "r9b" => "r9",
        "r10" | "r10d" | "r10w" | "r10b" => "r10",
        "r11" | "r11d" | "r11w" | "r11b" => "r11",
        "r12" | "r12d" | "r12w" | "r12b" => "r12",
        "r13" | "r13d" | "r13w" | "r13b" => "r13",
        "r14" | "r14d" | "r14w" | "r14b" => "r14",
        "r15" | "r15d" | "r15w" | "r15b" => "r15",
        _ => return None,
    })
}


#[derive(Debug, Clone, PartialEq)]
pub struct AsmSections {
    
    pub tmpl: String,
    
    pub outs: Vec<(String, String, bool)>,
    
    pub ins: Vec<(String, Vec<Token>)>,
    
    pub clobbers: Vec<String>,
}


fn 切(inner: &[Token], 是分界: impl Fn(&Token) -> bool) -> Vec<Vec<Token>> {
    let mut 出 = Vec::new();
    let mut 当前 = Vec::new();
    let mut 层 = 0i32;
    for t in inner {
        match t {
            Token::Sym("(") | Token::Sym("[") | Token::Sym("{") => 层 += 1,
            Token::Sym(")") | Token::Sym("]") | Token::Sym("}") => 层 -= 1,
            _ => {}
        }
        if 层 == 0 && 是分界(t) {
            出.push(std::mem::take(&mut 当前));
            continue;
        }
        当前.push(t.clone());
    }
    if !当前.is_empty() {
        出.push(当前);
    }
    出.retain(|s| !s.is_empty());
    出
}

fn 顶层冒号(inner: &[Token]) -> Vec<usize> {
    let mut 层 = 0i32;
    let mut 出 = Vec::new();
    for (k, t) in inner.iter().enumerate() {
        match t {
            Token::Sym("(") | Token::Sym("[") | Token::Sym("{") => 层 += 1,
            Token::Sym(")") | Token::Sym("]") | Token::Sym("}") => 层 -= 1,
            Token::Sym(":") if 层 == 0 => 出.push(k),
            _ => {}
        }
    }
    出
}


fn 箭头(t: &[Token], i: usize) -> Option<(&'static str, usize)> {
    match (t.get(i), t.get(i + 1), t.get(i + 2)) {
        (Some(Token::Sym("-")), Some(Token::Sym(">")), _) => Some(("->", 2)),
        (Some(Token::Sym("<")), Some(Token::Sym("-")), Some(Token::Sym(">"))) => Some(("<->", 3)),
        (Some(Token::Sym("<")), Some(Token::Sym("-")), _) => Some(("<-", 2)),
        _ => None,
    }
}


pub fn 解析隔离区(inner: &[Token]) -> Result<AsmSections, String> {
    
    for t in inner {
        let 文本 = match t {
            Token::Str(s) | Token::RawStr(s) | Token::Ident(s) => &**s,
            _ => continue,
        };
        if let Some(c) = 文本.chars().find(|c| !c.is_ascii()) {
            return Err(format!(
                "asm isolation region allows ASCII only - \"{}\" in \"{}\" is not an ASCII character",
                文本, c
            ));
        }
    }
    let 冒号 = 顶层冒号(inner);
    
    let mut 段: Vec<&[Token]> = Vec::new();
    let mut 起 = 0usize;
    for k in 0..=冒号.len() {
        let 止 = if k < 冒号.len() { 冒号[k] } else { inner.len() };
        段.push(&inner[起..止]);
        if k < 冒号.len() {
            起 = 冒号[k] + 1;
        }
    }
    if 段.len() > 4 {
        return Err("asm has at most four segments (template : output : input : clobber)".into());
    }

    
    let mut 模板 = String::new();
    for t in 段[0] {
        match t {
            Token::Str(s) | Token::RawStr(s) => {
                if !模板.is_empty() {
                    模板.push_str("\n\t");
                }
                模板.push_str(s);
            }
            _ => return Err("asm's first segment is the instruction template string".into()),
        }
    }

    let mut 出项 = Vec::new();
    let mut 入项 = Vec::new();
    let mut 破坏 = Vec::new();

    if 段.len() >= 2 {
        for 项 in 切(段[1], |t| *t == Token::Sym(",")) {
            let (约束, 名, 读写, 步) = 读绑定(&项)?;
            let _ = 步;
            if 约束.is_empty() {
                return Err("the output segment is written `register -> name` (or `register <-> name`)".into());
            }
            if 名.is_empty() {
                return Err("the output segment must write a Prion name".into());
            }
            出项.push((约束, 名, 读写));
        }
    }
    if 段.len() >= 3 {
        for 项 in 切(段[2], |t| *t == Token::Sym(",")) {
            
            let 寄 = match 项.first() {
                Some(Token::Ident(n)) => n.clone(),
                _ => return Err("the input segment is written `register <- expression`".into()),
            };
            match 箭头(&项, 1) {
                Some(("<-", 步)) => {
                    let 约束 = 寄存器约束(&寄)
                        .ok_or_else(|| format!("register \"{}\" is not in the mapping table", 寄))?
                        .to_string();
                    入项.push((约束, 项[1 + 步..].to_vec()));
                }
                _ => return Err("the input segment is written `register <- expression`".into()),
            }
        }
    }
    if 段.len() >= 4 {
        for 项 in 切(段[3], |t| *t == Token::Sym(",")) {
            match 项.first() {
                Some(Token::Ident(n)) => 破坏.push(n.to_string()),
                _ => return Err("the clobber segment lists memory / cc / register names".into()),
            }
        }
    }

    Ok(AsmSections {
        tmpl: 模板,
        outs: 出项,
        ins: 入项,
        clobbers: 破坏,
    })
}


fn 读绑定(项: &[Token]) -> Result<(String, String, bool, usize), String> {
    let 寄 = match 项.first() {
        Some(Token::Ident(n)) => n.clone(),
        _ => return Err("a binding entry must start with a register name (e.g. `al -> name`)".into()),
    };
    let (形, 步) = match 箭头(项, 1) {
        Some((形, 步)) if 形 == "->" || 形 == "<->" => (形, 步),
        _ => return Err("the output segment uses `register -> name` (write-only) or `register <-> name` (read-write)".into()),
    };
    let 名 = match 项.get(1 + 步) {
        Some(Token::Ident(n)) | Some(Token::Str(n)) => n.clone(),
        _ => return Err("the right side of the arrow must be a Prion name".into()),
    };
    let 约束 = 寄存器约束(&寄).ok_or_else(|| format!("register \"{}\" is not in the mapping table", 寄))?;
    Ok((约束.to_string(), 名.to_string(), 形 == "<->", 步))
}


pub fn 括号内(toks: &[Token], 左括号: usize) -> Option<&[Token]> {
    let mut 层 = 0i32;
    for (k, t) in toks.iter().enumerate().skip(左括号) {
        match t {
            Token::Sym("(") => 层 += 1,
            Token::Sym(")") => {
                层 -= 1;
                if 层 == 0 {
                    return Some(&toks[左括号 + 1..k]);
                }
            }
            _ => {}
        }
    }
    None
}


pub fn 映射表说明() -> &'static str {
    "register mapping: al/ax/eax/rax -> a; bl/bx/ebx/rbx -> b; cl/cx/ecx/rcx -> c; dl/dx/edx/rdx -> d; si/esi/rsi -> S; di/edi/rdi -> D; r8-r15 -> r8...r15"
}
