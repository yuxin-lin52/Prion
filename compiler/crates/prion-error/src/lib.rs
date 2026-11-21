








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
            "error at {} {}, in {}",
            cx.file, 哪, parts.做
        )),
        (false, false, true) => s.push_str(&format!("error at {} {}", cx.file, 哪)),
        (false, true, false) => {
            s.push_str(&format!("error in {} at {}", cx.file, parts.做))
        }
        (false, true, true) => s.push_str(&format!("error in {}", cx.file)),
        (true, false, false) => s.push_str(&format!("error at {}, in {}", 哪, parts.做)),
        (true, false, true) => s.push_str(&format!("error at {}", 哪)),
        (_, true, false) => s.push_str(&format!("error in {}", parts.做)),
        _ => s.push_str("error"),
    }
    
    let mut bad = parts.缺.clone();
    if !parts.致.is_empty() {
        bad.push_str(&format!(" \"{}\"", parts.致));
    }
    if !bad.is_empty() {
        s.push_str(": ");
        s.push_str(&bad);
    }
    if !cx.src_line.is_empty() {
        s.push_str(&format!("; source: {}", cx.src_line.trim()));
    }
    s.push('.');
    
    if !parts.预期.is_empty() {
        s.push_str(&format!(" Expected: {}.", parts.预期));
    }
    
    if !parts.改.is_empty() {
        s.push(' ');
        s.push_str(match tone {
            Tone::Suggest => "Suggestion: ",
            Tone::Fix => "Fix: ",
        });
        s.push_str(&parts.改);
        if !parts.可应用.is_empty() {
            s.push_str(&format!(" (apply as-is: {})", parts.可应用));
        }
        s.push('.');
    }
    
    if !parts.详解.is_empty() {
        s.push_str(&format!(" Details: {}", parts.详解));
    }
    s
}




pub fn code_defaults(code: &str) -> (&'static str, &'static str) {
    match code {
        "E202" => (
            "the operand types must be the same (convert explicitly to change a type)",
            "Prion never converts types implicitly: to change between int / float / bool / str, write int(value) / float(value) / str(value) / bool(value) explicitly; containers are copied at function boundaries, and an X* copy shares the same object.",
        ),
        "E203" => (
            "follow the documented form (see Fix for the expected form)",
            "This means the code breaks a contract rather than being logically wrong; use the form given under Fix.",
        ),
        "E204" => (
            "a match must cover every variant of the sum type (or add a `case _:` fallback)",
            "Every variant is a reachable path; a missing case leaves that path without behavior. Add the missing cases, or use `case _:` to cover the rest.",
        ),
        "E205" => (
            "compile-time evaluation exceeded its limit",
            "Compile-time evaluation runs inside the compiler and must be bounded, otherwise a mistaken recursion or loop would make the build never return. This is not a syntax error; raise --const-depth / --const-steps (0 means no limit).",
        ),
        "E201" => (
            "this name must be a val or function already declared in the current block",
            "Prion requires declare-before-use: a name must first be created with add val (value) / add fn (function) / add type (type) before it can be read. For a name in another block, read a value with call #:block val \"name\" and call a function with call #:block fn \"name\"(...); within the same block, use it directly.",
        ),
        "E101" => (
            "after a construct opens its body, close it with a `;` on its own line",
            "Prion ends a construct body with a `;` on its own line — the same for if / while / for / match. Indentation is cosmetic; the body closes only at that lone `;`.",
        ),
        "E102" => (
            "write statements and nesting as documented",
            "Nesting is expressed by opening a body with `:` and closing it with a `;` on its own line; depth prefixes (`;number`) are removed, and indentation is not parsed.",
        ),
        "E103" => (
            "a name must be created before its value can be changed",
            "Prion has a single creation path, add: first add val \"name\" type = initial, then val \"name\" = new. Changing the value of a name that was never created is a compile error.",
        ),
        "E104" => (
            "block markers must pair: every b needs a matching d",
            "A block opens with #: name b and closes with #: name d. A missing d corrupts the block structure of the whole file, so the compiler cannot tell which statements belong to which block.",
        ),
        "E105" => (
            "after a leading `;` write only the nesting close, nothing else",
            "A lone `;` on its own line closes a body; a leading `;` followed by more content is not a valid statement.",
        ),
        "E001" => (
            "characters must be in Prion's lexical set",
            "Prion uses plain ASCII: English keywords, operators, and symbols such as #, :, and quotes. Chinese punctuation, full-width spaces, and special Unicode symbols are not in the set. Watch for look-alike characters.",
        ),
        "E002" => (
            "an nt comment must start at column 0 followed by a space",
            "A full-line comment has exactly one form: `nt` at the start of the line, then a space, then the text. Indentation before it, or placing it mid-line, is not a comment.",
        ),
        "E003" => (
            "use a valid base and separator for numeric literals",
            "Supported prefixes are 0x (hex), 0b (binary), 0o (octal), with `_` separators (e.g. 1_000); decimal is written directly. A wrong prefix or misplaced separator lands here.",
        ),
        "E004" => (
            "a string allows only four escapes",
            "A string literal is wrapped in double quotes; the only escapes are newline, tab, backslash, and quote. Anything else is not parsed.",
        ),
        "E005" => (
            "a raw string starts with @ followed by a double quote",
            "A raw string (escapes not parsed) starts with an at-sign and a double quote. A missing @ or an unclosed quote lands here.",
        ),
        "E006" => (
            "source files must use LF line endings",
            "Prion requires UTF-8 without BOM and LF line endings. CRLF (common on Windows) makes the lexer treat a carriage return as an illegal character; save the file as UTF-8 with LF.",
        ),
        "E007" => (
            "source files must be UTF-8 without BOM",
            "Prion requires UTF-8 without BOM and LF line endings. Windows editors often add a BOM (the three bytes EF BB BF); switch the encoding to UTF-8 without BOM and rebuild.",
        ),
        "E301" => (
            "write expressions and statements completely (every construct has an opening and a close)",
            "This category means the code stops halfway: an unclosed expression, an if expression without else, or a unary minus with no operand. Complete the construct at the reported location.",
        ),
        "E302" => (
            "use only operations the container supports",
            "list / map / que / stack each expose different operations: que and stack are push/pop only; positional insert/remove and sorting apply only to list; map is accessed by key with put. Using a wrong operation lands here.",
        ),
        "E303" => (
            "field access and construction must match the type definition",
            "For a user type, field names, counts, and types come from add type: after `.` must be a field of that type; a constructor must pass exactly as many arguments as there are fields; a field with `*` is a handle (shared), one without is a value copy.",
        ),
        "E304" => (
            "satisfy the prerequisites before cross-block access",
            "Reading another block's val or calling its function requires that block to have run first (call #:block opens its slots). Reading before the slots exist, or reading a name that does not exist, is reported here.",
        ),
        "E305" => (
            "constructing and matching a sum type must use declared variants",
            "Variant names and their carried fields come from the `|` list in add type; a match case must name a declared variant. A missing variant is reported here (exhaustiveness has its own check, E204).",
        ),
        "E306" => (
            "write type annotations in a valid form",
            "Container annotations carry an element type: list(int) / map(str, int) / que(int); user type names are written directly, with `*` for a handle (e.g. node*). A typo or unclosed bracket lands here.",
        ),
        "E309" => (
            "fix the source as the message indicates",
            "This is a fallback code from the intermediate representation layer; see the message and Fix for the concrete cause.",
        ),
        "E401" => (
            "block calls must not form a cycle, and no block may be unreachable",
            "If block A calls block B and block B calls back to A, both are referenced only by each other, so neither has a start and the program cannot reach them. Break one of the references (or make one block unreferenced).",
        ),
        "E402" => (
            "use a feature the backend supports",
            "This means the backend does not implement the feature yet; it is not an error in your code. Work around it, or file a feature request.",
        ),
        "E403" => (
            "the produced artifact must be accepted by the compiler",
            "After lowering IR to a target artifact, the system compiler compiles it; a failure here means the artifact is malformed (an internal defect). Please file the source and this message as an issue.",
        ),
        "E404" => (
            "the backend must handle input by contract",
            "The backend reached a state it should never reach (an internal defect). Please file the source as an issue.",
        ),
        "E501" => (
            "the command must be one of build / run / check / clean",
            "`prionc -h` lists all commands. A command is a verb, optionally followed by a name (source file or project).",
        ),
        "E502" => (
            "the option must be one the command supports",
            "Each command has its own options: see `prionc build -h`. Options are written as `--name value` or `--name=value`.",
        ),
        "E503" => (
            "the option value must be valid",
            "`--mode` accepts only `debug` / `release`; `--opt` accepts 0-3; `--backend` has been removed, and LLVM is the only backend.",
        ),
        "E504" => (
            "a command takes at most one name",
            "The name selects what to build: omit it for the current directory's project or sole `.an`; provide it for `<name>.an` or a project directory of the same name.",
        ),
        "E505" => (
            "the name must refer to a `.an` file or a project directory in the current directory",
            "Lookup order: `<name>.an` -> `<name>/lib.prion` project -> the current project's package name. If none exists, this is reported.",
        ),
        "E601" => (
            "a usable compiler must be available in the environment",
            "Prion produces an intermediate artifact and hands it to a backend compiler. The backend is resolved in order: `PRION_CC` -> SDK's sibling `tools/clang/bin` -> `clang` on PATH. If none is found, this is reported; the SDK's clang is used automatically if installed. clang must compile and link C (MSVC or GNU target on Windows), provided it can find the system C headers and linker.",
        ),
        "E602" => (
            "files and directories must be readable/writable",
            "The compiler is reading or writing your files or directories. Check that the path exists, has read/write permission, and is not held by another program.",
        ),
        "E603" => (
            "the program must start",
            "`run` starts the built executable as a child process. Failure to start usually means the artifact path is not executable (permissions, antivirus, or a busy path).",
        ),
        "E604" => (
            "the program's exit status must be expressible as an exit code",
            "`run` forwards the child's exit code. 0-255 pass through; an exit code beyond 8 bits, or termination by a signal, has no code to forward, so it is reported here - this is not a compile error.",
        ),
        "E701" => (
            "this should not happen",
            "This is a compiler defect, not an issue in your code. File the source and this message as an issue.",
        ),
        _ => ("", ""),
    }
}

pub fn render(parts: &DiagParts, tone: Tone) -> String {
    let mut s = String::new();
    match (parts.哪.is_empty(), parts.做.is_empty()) {
        (false, false) => s.push_str(&format!("{} at {}: ", parts.做, parts.哪)),
        (false, true) => s.push_str(&format!("at {}: ", parts.哪)),
        (true, false) => s.push_str(&format!("in {}: ", parts.做)),
        (true, true) => {}
    }
    s.push_str(&parts.缺);
    if !parts.致.is_empty() {
        s.push_str(&format!(" \"{}\"", parts.致));
    }
    if !parts.改.is_empty() {
        s.push_str(match tone {
            Tone::Suggest => ", suggestion: ",
            Tone::Fix => ", fix: ",
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
