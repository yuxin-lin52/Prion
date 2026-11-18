








#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub len: usize,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Suggest,
    Fix,
}


#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagParts {
    pub 哪: String,
    pub 做: String,
    pub 缺: String,
    pub 致: String,
    pub 改: String,
    
    pub 预期: String,
    
    pub 详解: String,
    
    pub 可应用: String,
}

impl DiagParts {
    pub fn full(
        哪: impl Into<String>,
        做: impl Into<String>,
        缺: impl Into<String>,
        致: impl Into<String>,
        改: impl Into<String>,
    ) -> Self {
        Self {
            哪: 哪.into(),
            做: 做.into(),
            缺: 缺.into(),
            致: 致.into(),
            改: 改.into(),
            ..Default::default()
        }
    }

    
    pub fn 预期(mut self, v: impl Into<String>) -> Self {
        self.预期 = v.into();
        self
    }
    
    pub fn 详解(mut self, v: impl Into<String>) -> Self {
        self.详解 = v.into();
        self
    }
    
    pub fn 可应用(mut self, v: impl Into<String>) -> Self {
        self.可应用 = v.into();
        self
    }
}


pub struct RenderCtx<'a> {
    pub code: &'a str,
    pub file: &'a str,
    pub src_line: &'a str,
}



pub fn render_full(parts: &DiagParts, tone: Tone, cx: &RenderCtx) -> String {
    let mut s = String::new();
    
    if !cx.code.is_empty() {
        s.push_str(&format!("[{}] ", cx.code));
    }
    let 哪 = parts.哪.trim();
    match (cx.file.is_empty(), 哪.is_empty(), parts.做.is_empty()) {
        (false, false, false) => s.push_str(&format!(
            "位于 {} {}的「{}」发生错误",
            cx.file, 哪, parts.做
        )),
        
        (false, false, true) => s.push_str(&format!("位于 {} {}发生错误", cx.file, 哪)),
        
        (false, true, false) => {
            s.push_str(&format!("位于 {} 的「{}」发生错误", cx.file, parts.做))
        }
        (false, true, true) => s.push_str(&format!("位于 {} 发生错误", cx.file)),
        (true, false, false) => s.push_str(&format!("{}的「{}」发生错误", 哪, parts.做)),
        (true, false, true) => s.push_str(&format!("{}发生错误", 哪)),
        (_, true, false) => s.push_str(&format!("「{}」发生错误", parts.做)),
        _ => s.push_str("发生错误"),
    }
    
    if !cx.src_line.is_empty() {
        s.push_str(&format!("，源码为：{}", cx.src_line.trim()));
    }
    
    let mut 错 = parts.缺.clone();
    if !parts.致.is_empty() {
        错.push_str(&format!("「{}」", parts.致));
    }
    if !错.is_empty() {
        s.push_str(&format!("，错误为：{}", 错));
    }
    
    if !parts.预期.is_empty() {
        s.push_str(&format!("，编译器预期为：{}", parts.预期));
    }
    s.push('。');
    
    if !parts.改.is_empty() {
        s.push_str(match tone {
            Tone::Suggest => "建议：",
            Tone::Fix => "改法：",
        });
        s.push_str(&parts.改);
        if !parts.可应用.is_empty() {
            s.push_str(&format!("（可直接套用：{}）", parts.可应用));
        }
        s.push('。');
    }
    
    if !parts.详解.is_empty() {
        s.push_str(&format!("详细解释如下：{}", parts.详解));
    }
    s
}




pub fn code_defaults(code: &str) -> (&'static str, &'static str) {
    match code {
        "E202" => (
            "两边的类型要一致（要换类型就显式转换）",
            "Prion 不悄悄转换类型：int / float / bool / str 之间要换，就显式写 int(值) / \
             float(值) / str(值) / bool(值)；容器按 fn 边界拷贝，X* 复制把手共享同一个对象。",
        ),
        "E203" => (
            "遵守这条规则的正书面貌（见改法给出的写法）",
            "这一类是「写得不合契约」——不是程序错了，是写法超出规则；按改法给出的写法\
             即可，规则出处见设计正本对应章节。",
        ),
        "E204" => (
            "match 要覆盖和类型的所有变体（或写 case _: 兜底）",
            "和类型的每个变体都是一条真实可能走到的路径；漏掉一条，程序遇到它时就没有\
             行为。编译器在这里替你查——补上缺的 case，或用 case _: 一把兜住。",
        ),
        "E205" => (
            "编译期求值超过配额上限",
            "编译期求值跑在编译器里，得有个边界，否则写错的递归或循环会让构建一直不返回。\
             超限不是语法错，把 --const-depth / --const-steps 调高即可；给 0 表示不设上限。",
        ),
        "E201" => (
            "这里的名字应当是本块已建过的 val / 函数",
            "Prion 的规则是「先建后用」：名字要先由 add val（值）/ add fn（函数）/ \
             add type（类型）建出来，才能在表达式里读。名字在别的块里时：取 val 写 \
             call #:块 val \"名\"，调函数写 call #:块 fn \"名\"(…)，同块直接用。",
        ),
        "E101" => (
            "结构行开体之后，用独占一行的 ; 收体",
            "Prion 用「独占一行的 ;」表示结构体结束——if / while / for / match 都一样；\
             缩进只是装饰，收体必须靠那个单独的 ;。",
        ),
        "E102" => (
            "按正本的语句与层级写法书写",
            "结构行的层级用「冒号开体 + 独占一行 ; 收体」表达；深度前缀（;数字）已废止，\
             缩进不参与解析。",
        ),
        "E103" => (
            "名字要先创建才能改值",
            "Prion 只有 add 一条创建路：先 add val \"名\" 类型 = 初值，之后才能 \
             val \"名\" = 新值。没建过就直接改值 = 编译报错。",
        ),
        "E104" => (
            "块开关必须配对：每个 b 都要有一个对应的 d",
            "代码块用 #: 名 b 开、#: 名 d 关；少一个 d 会让整份源码的块结构错乱，\
             编译器无从判断哪些语句属于哪个块。",
        ),
        "E105" => (
            "行首 ; 之后只写层级收束，不再接别的",
            "; 独占一行才是收体；行首 ; 后面再跟内容不是合法语句。",
        ),
        "E001" => (
            "字符要在 Prion 的词法表里",
            "Prion 用通用 ASCII：英文关键字、运算符与井号、冒号、引号等 ASCII 符号；中文标点、\
             全角空格、特殊 Unicode 符号都不在词表里。若以为是拼写问题，注意中文近形字\
             （报告 ≠ 报吿）。",
        ),
        "E002" => (
            "nt 注释只能写在行首第 0 列，且后面接一个空格",
            "整行注释只此一种形态：行首裸写 nt + 空格 + 内容；前面有缩进、或写在行中，\
             都不算注释。",
        ),
        "E003" => (
            "数字字面量用合法进制与分隔",
            "支持 0x（十六进制）/ 0b（二进制）/ 0o（八进制）前缀与下划线分隔（如 1_000）；\
             十进制直接写。写错前缀或分隔位置就会到这里。",
        ),
        "E004" => (
            "字符串里只允许四种转义",
            "字符串字面量用双引号包裹；转义只有 \\n（换行）/ \\t（制表）/ \\\\（反斜杠）/ \
             \\\"（引号）四种，其他写法不解析。",
        ),
        "E005" => (
            "原样串写作 @\\\"…\\\"",
            "原样串（不解析转义）用 @ 加双引号开头，如 @\\\"C:\\\\path\\\"；少了 @ 或引号\
             不闭合会到这里。",
        ),
        "E006" => (
            "源文件须纯 LF 换行",
            "Prion 的源码契约：UTF-8 无 BOM + 每行 LF 结束。CRLF（Windows 常见）会让\
             词法器把 \\r 当非法字符——用编辑器另存为「UTF-8 + LF」即可。",
        ),
        "E007" => (
            "源文件要 UTF-8 无 BOM",
            "Prion 的源码契约：UTF-8 **无 BOM** + LF 换行。Windows 编辑器常默认加 BOM\
             （文件头三个字节 EF BB BF）——用编辑器把编码改成「UTF-8（不带 BOM）」再编。",
        ),
        "E301" => (
            "表达式与语句要写完整（每个结构有头有尾）",
            "这一类是「写到一半」：表达式没闭合、三元少了 :、一元负号后面没跟字面量。\
             顺着报错位置把结构补完整即可。",
        ),
        "E302" => (
            "容器操作要在该容器的能力范围内",
            "list / map / que / stack 各有各的操作面：que 与 stack 只进只出（push/pop），\
             按位置删改与排序只对 list；map 按键取用、增改用 put。用错了操作会到这里。",
        ),
        "E303" => (
            "字段访问与构造要符合类型定义",
            "自定义类型的字段名、个数、类型都由 add type 说了算：. 后面必须是该类型有的\
             字段；构造时实参个数要和字段数一致；带 * 的字段是把手（共享），不带是值拷贝。",
        ),
        "E304" => (
            "跨块取用前先满足前置条件",
            "取别块的 val / 调别块的函数，都要先让那个块跑起来（call #:块 把槽开出来）；\
             槽没开就取，或取了不存在的名字，都会在这里报。",
        ),
        "E305" => (
            "和类型的构造与判别要落在已声明的变体上",
            "变体名、携带字段的个数与类型都以 add type 的竖线清单为准；match 的 case 要\
             对上变体名，漏变体或写错名会到这里（漏变体另有穷尽性检查 E204）。",
        ),
        "E306" => (
            "类型标注要写成合法形态",
            "容器标注带元素类型：list(int) / map(str, int) / que(int)；自定义类型名直接写，\
             把手加 *（如 node*）。拼错或括号不闭合会到这里。",
        ),
        "E309" => (
            "按报错给出的写法修正源码",
            "这一条是中间表示层的兜底码——具体原因见「错误为」与「改法」。",
        ),
        "E401" => (
            "块之间 call 不能成环，也不能留下执行不到的块",
            "块 A call 块 B、块 B 又 call 回 A，两个块就都「只被对方引用」——\
             没有起点，程序跑不到它们。断开其中一处引用（或让其中一个块不再被别人引用）即可。",
        ),
        "E402" => (
            "用后端已支持的特性",
            "这一条是「后端还没实现这个特性」——不是你的代码错，是编译器没排上。\
             可先绕开该写法，或在项目里登记该特性需求。",
        ),
        "E403" => (
            "产出的中间产物要能被编译器接受",
            "后端把 IR 翻成目标产物（当前 C 后端翻成 C）之后交由系统里的编译器编；\
             这一步失败说明产物有问题（编译器内部缺陷）。请把源码与报错一起提交到项目 issue。",
        ),
        "E404" => (
            "后端按契约处理",
            "后端遇到了它不该遇到的状态（内部缺陷）。请把源码提交到项目 issue。",
        ),
        "E501" => (
            "命令要是 build / run / check / clean 之一",
            "`prionc -h` 会列出全部命令。命令是动词，后面跟一个可选的名字（源文件名或项目名）。",
        ),
        "E502" => (
            "选项要在该命令支持的清单里",
            "各命令的选项不同：`prionc build -h` 看 build 支持哪些。选项写作 `--名 值` 或 `--名=值`。",
        ),
        "E503" => (
            "选项取值要合法",
            "`--backend` 只收 `llvm`（缺省）与 `c`。",
        ),
        "E504" => (
            "一个命令最多给一个名字",
            "名字说明「编哪个」——省略 = 当前目录的项目或唯一 `.an`；给了 = `<名>.an` 或同名项目目录。",
        ),
        "E505" => (
            "名字要指向当前目录下的 `.an` 文件或同名项目目录",
            "查名字的顺序：`<名>.an` → `<名>/lib.anle` 项目 → 当前目录项目的包名。都没有就报这条。",
        ),
        "E601" => (
            "环境里要有一个能用的编译器",
            "Prion 产中间产物后交给后端编译器编。后端取法按顺序：`PRION_CC` → SDK 同根 \
             `tools/clang/bin` → PATH 里的 `clang`。三条都没有就报这条；\
             装了 SDK 的 clang 组件会被自动用上。clang 要能编 C 并链接 \
             （Windows 上 MSVC target 与 GNU target 均可），前提是它找得到系统 C 头文件与链接器。",
        ),
        "E602" => (
            "文件 / 目录要能读写",
            "这一步是编译器在读写你的文件或目录。检查路径是否存在、有没有读写权限、\
             文件有没有被别的程序占着。",
        ),
        "E603" => (
            "程序要能跑起来",
            "`run` 会把编好的可执行文件起成子进程。起不来通常是产物路径不可执行\
             （权限 / 被杀软拦 / 路径被占用）。",
        ),
        "E604" => (
            "程序的结束状态要能用退出码表达",
            "`run` 会把被运行程序的退出码原样交出去。0–255 能透传；退出码超出 8 位、\
             或被信号终止时没有可透传的退出码，所以在这里报告——这不是编译错误。",
        ),
        "E701" => (
            "这不该发生",
            "这是编译器的缺陷，不是你代码的问题。把源码与这条报错一起提交到项目 issue 即可。",
        ),
        _ => ("", ""),
    }
}


pub fn render(parts: &DiagParts, tone: Tone) -> String {
    let mut s = String::new();
    match (parts.哪.is_empty(), parts.做.is_empty()) {
        (false, false) => s.push_str(&format!("{} 处 {} 的 ", parts.哪, parts.做)),
        (false, true) => s.push_str(&format!("{} 处的 ", parts.哪)),
        (true, false) => s.push_str(&format!("执行 {} 时的 ", parts.做)),
        (true, true) => {}
    }
    s.push_str(&parts.缺);
    if !parts.致.is_empty() {
        s.push_str(&format!("「{}」", parts.致));
    }
    if !parts.改.is_empty() {
        s.push_str(match tone {
            Tone::Suggest => "，建议",
            Tone::Fix => "，正确做法是",
        });
        s.push_str(&parts.改);
    }
    s
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexCode {
    
    E001,
    
    E002,
    
    E003,
    
    E004,
    
    E005,
    
    E006,
    
    E007,
}

impl LexCode {
    pub fn number(self) -> &'static str {
        match self {
            LexCode::E001 => "E001",
            LexCode::E002 => "E002",
            LexCode::E003 => "E003",
            LexCode::E004 => "E004",
            LexCode::E005 => "E005",
            LexCode::E006 => "E006",
            LexCode::E007 => "E007",
        }
    }
}


pub fn render_lex(code: LexCode, parts: &DiagParts, tone: Tone) -> String {
    format!("[{}] {}", code.number(), render(parts, tone))
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseCode {
    
    E101,
    
    E102,
    
    E103,
    
    E104,
    
    E105,
}

impl ParseCode {
    pub fn number(self) -> &'static str {
        match self {
            ParseCode::E101 => "E101",
            ParseCode::E102 => "E102",
            ParseCode::E103 => "E103",
            ParseCode::E104 => "E104",
            ParseCode::E105 => "E105",
        }
    }
}


pub fn render_parse(code: ParseCode, parts: &DiagParts, tone: Tone) -> String {
    format!("[{}] {}", code.number(), render(parts, tone))
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckCode {
    
    E201,
    
    E202,
    
    E203,
    
    E204,
}

impl CheckCode {
    pub fn number(self) -> &'static str {
        match self {
            CheckCode::E201 => "E201",
            CheckCode::E202 => "E202",
            CheckCode::E203 => "E203",
            CheckCode::E204 => "E204",
        }
    }
}


pub fn render_check(code: CheckCode, parts: &DiagParts, tone: Tone) -> String {
    format!("[{}] {}", code.number(), render(parts, tone))
}




#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrCode {
    
    E301,
    
    E302,
    
    E303,
    
    E304,
    
    E305,
    
    E306,
    
    E309,
}

impl IrCode {
    pub fn number(self) -> &'static str {
        match self {
            IrCode::E301 => "E301",
            IrCode::E302 => "E302",
            IrCode::E303 => "E303",
            IrCode::E304 => "E304",
            IrCode::E305 => "E305",
            IrCode::E306 => "E306",
            IrCode::E309 => "E309",
        }
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenCode {
    
    E401,
    
    E402,
    
    E403,
    
    E404,
}

impl GenCode {
    pub fn number(self) -> &'static str {
        match self {
            GenCode::E401 => "E401",
            GenCode::E402 => "E402",
            GenCode::E403 => "E403",
            GenCode::E404 => "E404",
        }
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliCode {
    
    E501,
    
    E502,
    
    E503,
    
    E504,
    
    E505,
}

impl CliCode {
    pub fn number(self) -> &'static str {
        match self {
            CliCode::E501 => "E501",
            CliCode::E502 => "E502",
            CliCode::E503 => "E503",
            CliCode::E504 => "E504",
            CliCode::E505 => "E505",
        }
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvCode {
    
    E601,
    
    E602,
    
    E603,
    
    E604,
}

impl EnvCode {
    pub fn number(self) -> &'static str {
        match self {
            EnvCode::E601 => "E601",
            EnvCode::E602 => "E602",
            EnvCode::E603 => "E603",
            EnvCode::E604 => "E604",
        }
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalCode {
    
    E701,
}

impl InternalCode {
    pub fn number(self) -> &'static str {
        match self {
            InternalCode::E701 => "E701",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    E001, E002, E003, E004, E005, E006, E007,
    E101, E102, E103, E104, E105,
    E201, E202, E203, E204, E205,
    E301, E302, E303, E304, E305, E306, E309,
    E401, E402, E403, E404,
    E501, E502, E503, E504, E505,
    E601, E602, E603, E604,
    E701,
}

impl Code {
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Code::E001 => "E001", Code::E002 => "E002", Code::E003 => "E003",
            Code::E004 => "E004", Code::E005 => "E005", Code::E006 => "E006",
            Code::E007 => "E007", Code::E101 => "E101", Code::E102 => "E102",
            Code::E103 => "E103", Code::E104 => "E104", Code::E105 => "E105",
            Code::E201 => "E201", Code::E202 => "E202", Code::E203 => "E203",
            Code::E204 => "E204", Code::E205 => "E205", Code::E301 => "E301", Code::E302 => "E302",
            Code::E303 => "E303", Code::E304 => "E304", Code::E305 => "E305",
            Code::E306 => "E306", Code::E309 => "E309", Code::E401 => "E401",
            Code::E402 => "E402", Code::E403 => "E403", Code::E404 => "E404",
            Code::E501 => "E501", Code::E502 => "E502", Code::E503 => "E503",
            Code::E504 => "E504", Code::E505 => "E505", Code::E601 => "E601",
            Code::E602 => "E602", Code::E603 => "E603", Code::E604 => "E604",
            Code::E701 => "E701",
        }
    }
}
