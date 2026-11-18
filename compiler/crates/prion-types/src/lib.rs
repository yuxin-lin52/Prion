








use std::collections::HashMap;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TyId(pub u32);


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TyKind {
    
    Named(String),
    
    Applied(String, Vec<TyId>),
    
    Ptr(TyId),
}


#[derive(Debug, Default)]
pub struct Types {
    kinds: Vec<TyKind>,
    texts: Vec<String>,
    索引: HashMap<String, TyId>,
}

impl Types {
    pub fn new() -> Self {
        Self::default()
    }

    
    pub fn intern(&mut self, text: &str) -> TyId {
        let mut cur = 0usize;
        let cs: Vec<char> = text.chars().collect();
        let id = self.解析(&cs, &mut cur);
        id
    }

    pub fn kind(&self, id: TyId) -> &TyKind {
        &self.kinds[id.0 as usize]
    }

    
    pub fn text(&self, id: TyId) -> &str {
        &self.texts[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    
    pub fn 实例文本(&self, 名: &str, 实参: &[TyId]) -> String {
        let 段: Vec<&str> = 实参.iter().map(|a| self.text(*a)).collect();
        format!("{}({})", 名, 段.join(", "))
    }

    
    fn 注册(&mut self, kind: TyKind, 文本: String) -> TyId {
        if let Some(id) = self.索引.get(&文本) {
            return *id;
        }
        let id = TyId(self.kinds.len() as u32);
        self.kinds.push(kind);
        self.texts.push(文本.clone());
        self.索引.insert(文本, id);
        id
    }

    fn 解析(&mut self, cs: &[char], cur: &mut usize) -> TyId {
        let 名 = self.读名(cs, cur);
        let mut id = if *cur < cs.len() && cs[*cur] == '(' {
            *cur += 1;
            let mut 实参: Vec<TyId> = Vec::new();
            loop {
                self.跳空白(cs, cur);
                if *cur < cs.len() && cs[*cur] == ')' {
                    *cur += 1;
                    break;
                }
                let a = self.解析(cs, cur);
                实参.push(a);
                self.跳空白(cs, cur);
                if *cur < cs.len() && cs[*cur] == ',' {
                    *cur += 1;
                    continue;
                }
                if *cur < cs.len() && cs[*cur] == ')' {
                    *cur += 1;
                    break;
                }
                break;
            }
            let 文本 = {
                let 段: Vec<String> = 实参.iter().map(|a| self.text(*a).to_string()).collect();
                format!("{}({})", 名, 段.join(", "))
            };
            self.注册(TyKind::Applied(名, 实参), 文本)
        } else {
            let 文本 = 名.clone();
            self.注册(TyKind::Named(名), 文本)
        };
        self.跳空白(cs, cur);
        while *cur < cs.len() && cs[*cur] == '*' {
            *cur += 1;
            let 文本 = format!("{}*", self.text(id));
            id = self.注册(TyKind::Ptr(id), 文本);
            self.跳空白(cs, cur);
        }
        id
    }

    fn 读名(&mut self, cs: &[char], cur: &mut usize) -> String {
        self.跳空白(cs, cur);
        let mut s = String::new();
        while *cur < cs.len() {
            let c = cs[*cur];
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                *cur += 1;
            } else {
                break;
            }
        }
        s
    }

    fn 跳空白(&self, cs: &[char], cur: &mut usize) {
        while *cur < cs.len() && cs[*cur].is_whitespace() {
            *cur += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 同结构同身份() {
        let mut t = Types::new();
        let a = t.intern("对子(int,str)");
        let b = t.intern("对子(int, str)");
        let c = t.intern("  对子( int , str )  ");
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(t.text(a), "对子(int, str)");
    }

    #[test]
    fn 不同实参不同身份() {
        let mut t = Types::new();
        assert_ne!(t.intern("list(int)"), t.intern("list(str)"));
        assert_ne!(t.intern("对子(int, str)"), t.intern("对子(str, int)"));
    }

    #[test]
    fn 把手与嵌套() {
        let mut t = Types::new();
        let p = t.intern("点*");
        assert!(matches!(t.kind(p), TyKind::Ptr(_)));
        let m = t.intern("map(str, list(int))");
        assert_eq!(t.text(m), "map(str, list(int))");
        assert_eq!(m, t.intern("map(str,list(int))"));
    }

    #[test]
    fn 实例文本唯一出处() {
        let mut t = Types::new();
        let i = t.intern("int");
        let s = t.intern("str");
        let 名 = t.实例文本("对子", &[i, s]);
        assert_eq!(名, "对子(int, str)");
        assert_eq!(t.intern(&名), t.intern("对子(int, str)"));
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(pub u32);


#[derive(Debug, Default)]
pub struct Names {
    texts: Vec<String>,
    索引: HashMap<String, Sym>,
}

impl Names {
    pub fn new() -> Self {
        Self::default()
    }

    
    pub fn intern(&mut self, s: &str) -> Sym {
        if let Some(&id) = self.索引.get(s) {
            return id;
        }
        let id = Sym(self.texts.len() as u32);
        let owned = s.to_string();
        self.索引.insert(owned.clone(), id);
        self.texts.push(owned);
        id
    }

    
    pub fn lookup(&self, s: &str) -> Option<Sym> {
        self.索引.get(s).copied()
    }

    pub fn text(&self, id: Sym) -> &str {
        &self.texts[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.texts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageClass {
    Tmp,
    Keep,
    Unsafe,
}

impl StorageClass {
    pub fn as_str(self) -> &'static str {
        match self {
            StorageClass::Tmp => "tmp",
            StorageClass::Keep => "keep",
            StorageClass::Unsafe => "unsafe",
        }
    }
}
