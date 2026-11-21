


















use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod alloc_stat;
mod fmt;
mod lsp;


#[global_allocator]
static ALLOC: alloc_stat::计数 = alloc_stat::计数;

use prion_check::check;
#[cfg(feature = "llvm")]
use prion_llvm::emit_object;
#[cfg(feature = "backend")]
use prion_lower::build as build_ir;
use prion_lexer::lex;
use prion_parser::Program as AstProgram;

const VERSION: &str = env!("CARGO_PKG_VERSION");



const SYNTAX: &str = "none";
const TIER: &str = "none";
const PACKS: &[&str] = &[];


fn version_banner() -> String {
    let 包显示 = PACKS.iter().map(|p| format!("+{}", p)).collect::<String>();
    let 包机器 = PACKS.join(",");
    let 名 = match TIER {
        "dev" => format!("Com for Prion Dev-{}", SYNTAX),
        "lts" => format!("Com for Prion {} LTS{}", SYNTAX, 包显示),
        _ => "Com for Prion unreleased".to_string(),
    };
    let 机器 = match TIER {
        "dev" => format!("syntax=Dev-{} tier=dev compiler={}", SYNTAX, VERSION),
        "lts" if 包机器.is_empty() => {
            format!("syntax={} tier=lts compiler={}", SYNTAX, VERSION)
        }
        "lts" => {
            format!("syntax={} tier=lts packs={} compiler={}", SYNTAX, 包机器, VERSION)
        }
        _ => format!("tier=none compiler={}", VERSION),
    };
    format!("{} v{}\n{}", 名, VERSION, 机器)
}



#[cfg(feature = "backend")]
fn 找cc() -> String {
    if let Ok(v) = env::var("PRION_CC") {
        if !v.trim().is_empty() {
            return v;
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(根) = exe.parent().and_then(|p| p.parent()) {
            for 名 in ["clang.exe", "clang"] {
                let p = 根.join("tools").join("clang").join("bin").join(名);
                if p.is_file() {
                    return p.to_string_lossy().into_owned();
                }
            }
        }
    }
    "clang".to_string()
}

const USAGE: &str = "\
prionc - Prion compiler

Usage:
  prionc build [name] [--mode debug|release] [--opt 0-3]
  prionc run   [name] [--mode ...] [--opt ...] [-- args...]
  prionc check [name]
  prionc fmt   [path] [--check]
  prionc clean [name]
  prionc -V | prionc -h

Notes:
  [name]    Optional. Omitted = the project in the current directory (has lib.prion) or the single .an file;
            Given = resolved by name (package name in lib.prion, or a <name>.an file).
  --mode    debug (full guards) / release (optimized, all guards stripped -- out-of-bounds becomes undefined behavior). Sets the output directory.
  --opt     Optimization level 0-3; default follows mode (debug->0, release->3).
  --const-depth N  Max nesting depth for compile-time evaluation, default 256; 0 = no limit.
  --const-steps N  Max steps for compile-time evaluation, default 1280000; 0 = no limit.
  --no-loop-reclaim  Disable cycle-tail reclamation (enabled by default)
  Output    build/<mode>/<name>.exe; intermediates <name>.ll/<name>.o in the same directory.
  Exit codes  0 success / 101 failure / 1 usage error; run forwards the child's exit code.";

const COMMAND_HELP: &[(&str, &str)] = &[
    ("build", "Compile the source into an executable (compile + link in one step)\nOutput: build/<mode>/<name>.exe; intermediate <name>.c in the same directory"),
    ("run", "Build, then run immediately on success\nArguments after `--` are passed through unchanged to the program"),
    ("check", "Check the code for errors only; no files produced (fastest)"),
    ("fmt", "Add decorative indentation only; tokens are untouched; `--check` lists files that need formatting"),
    ("frontend", "Run the **front end** only (lex -> parse -> check -> IR); no files, no clang\nFor front-end timing: best of 3 runs, prints lines/bytes/time/throughput"),
    ("clean", "Delete everything generated under build/"),
];

fn main() -> ExitCode {
    let h = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(主逻辑)
        .expect("failed to start main thread");
    let code = match h.join() {
        Ok(c) => c,
        Err(_) => ExitCode::from(101),
    };
    alloc_stat::报告();
    code
}


fn 主逻辑() -> ExitCode {
    
    std::panic::set_hook(Box::new(|info| {
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".to_string());
        let code = prion_error::InternalCode::E701.number();
        eprintln!(
            "{}",
            render_nofile(code, "", &format!("internal compiler error: {}", msg), "")
        );
    }));
    let args: Vec<String> = env::args().skip(1).collect();
    dispatch(&args)
}




fn fail101(msg: impl AsRef<str>) -> ExitCode {
    let m = msg.as_ref();
    if m.starts_with('[') {
        eprintln!("{}", m);
    } else {
        eprintln!("error: {}", m);
    }
    ExitCode::from(101)
}



fn render_nofile(code: &str, 做: &str, 缺: &str, 改: &str) -> String {
    let (de, dd) = prion_error::code_defaults(code);
    let parts = prion_error::DiagParts::full(String::new(), 做, 缺, "", 改)
        .预期(de)
        .详解(dd);
    let cx = prion_error::RenderCtx { code, file: "", src_line: "" };
    prion_error::render_full(&parts, prion_error::Tone::Fix, &cx)
}


fn render_file(code: &str, 做: &str, 缺: &str, 改: &str, src: &std::path::Path) -> String {
    let file = src
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (de, dd) = prion_error::code_defaults(code);
    let parts = prion_error::DiagParts::full(String::new(), 做, 缺, "", 改)
        .预期(de)
        .详解(dd);
    let cx = prion_error::RenderCtx { code, file: &file, src_line: "" };
    prion_error::render_full(&parts, prion_error::Tone::Fix, &cx)
}


fn usage_err(rendered: impl AsRef<str>) -> ExitCode {
    eprintln!("{}", rendered.as_ref());
    eprintln!();
    eprintln!("{}", USAGE);
    ExitCode::from(1)
}


fn dispatch(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("{}", USAGE);
        return ExitCode::from(1);
    }
    match args[0].as_str() {
        "-V" | "--version" => {
            println!("{}", version_banner());
            ExitCode::SUCCESS
        }
        "-h" | "--help" => {
            println!("{}", USAGE);
            ExitCode::SUCCESS
        }
        "build" | "run" | "check" | "clean" | "frontend" => run_command(&args[0], &args[1..]),
        "fmt" => do_fmt(&args[1..]),
        "lsp" => {
            
            let 码 = lsp::服务();
            if 码 == 0 { ExitCode::SUCCESS } else { ExitCode::from(1) }
        }
        other => usage_err(render_nofile(
            prion_error::CliCode::E501.number(),
            "run a command",
            &format!("unknown command \"{}\"", other),
            "run `prionc -h` to see available commands (build / run / check / clean)",
        )),
    }
}


struct Opts {
    name: Option<String>,
    mode: Option<String>,
    opt: Option<u32>,
    
    debug_info: Option<bool>,
    
    march_native: Option<bool>,
    
    const_depth: Option<usize>,
    const_steps: Option<usize>,
    loop_reclaim: bool,
    pass_args: Vec<String>,
    dump_ir: bool, 
    
    dump_tokens: bool,
    
    dump_ast: bool,
    help: bool,
}

impl Opts {
    fn new() -> Self {
        Self {
            name: None,
            mode: None,
            opt: None,
            march_native: None,
            debug_info: None,
            const_depth: None,
            const_steps: None,
            loop_reclaim: true,
            pass_args: Vec::new(),
            dump_ir: false,
            dump_tokens: false,
            dump_ast: false,
            help: false,
        }
    }
}

fn parse_opts(rest: &[String]) -> Result<Opts, String> {
    let mut o = Opts::new();
    let mut i = 0usize;
    while i < rest.len() {
        let a = rest[i].as_str();
        if a == "--" {
            o.pass_args = rest[i + 1..].to_vec();
            break;
        }
        if let Some(body) = a.strip_prefix("--") {
            let (k, inline) = match body.split_once('=') {
                Some((k, v)) => (k, Some(v.to_string())),
                None => (body, None),
            };
            
            macro_rules! val {
                () => {{
                    match &inline {
                        Some(v) => {
                            i += 1;
                            v.clone()
                        }
                        None => {
                            i += 1;
                            match rest.get(i) {
                                Some(v) => v.clone(),
                                None => return Err(format!("--{} requires a value", k)),
                            }
                        }
                    }
                }};
            }
            match k {
                "mode" => {
                    let v = val!();
                    if v != "debug" && v != "release" {
                        return Err(render_nofile(
                            prion_error::CliCode::E503.number(),
                            "option",
                            &format!("`--mode` accepts only debug|release, got {}", v),
                            "use `--mode debug` or `--mode release`",
                        ));
                    }
                    o.mode = Some(v);
                }
                "debug-info" | "g" => { o.debug_info = Some(true); }
                "march-native" => { o.march_native = Some(true); }
                "no-loop-reclaim" => { o.loop_reclaim = false; }
                "const-depth" | "const-steps" => {
                    let v = val!();
                    let n: usize = v.parse().map_err(|_| {
                        render_nofile(
                            prion_error::CliCode::E503.number(),
                            "option",
                            &format!("`--{}` accepts only a non-negative integer, got {}", k, v),
                            "use 0 or a positive integer; 0 means no limit",
                        )
                    })?;
                    if k == "const-depth" { o.const_depth = Some(n); }
                    else { o.const_steps = Some(n); }
                }
                "opt" => {
                    let v = val!();
                    let n: u32 = v.parse().map_err(|_| {
                        render_nofile(
                            prion_error::CliCode::E503.number(),
                            "option",
                            &format!("`--opt` accepts only 0-3, got {}", v),
                            "use a number between 0 and 3",
                        )
                    })?;
                    if n > 3 {
                        return Err(render_nofile(
                            prion_error::CliCode::E503.number(),
                            "option",
                            &format!("`--opt` accepts only 0-3, got {}", n),
                            "use a number between 0 and 3",
                        ));
                    }
                    o.opt = Some(n);
                }
                "backend" => {
                    let _ = val!();
                    return Err(render_nofile(
                        prion_error::CliCode::E503.number(),
                        "option",
                        "`--backend` has been removed: LLVM is the only backend",
                        "remove the `--backend` argument",
                    ));
                }
                "dump-ir" => o.dump_ir = true,
                "dump-tokens" => o.dump_tokens = true,
                "dump-ast" => o.dump_ast = true,
                "help" => o.help = true,
                other => {
                    return Err(render_nofile(
                        prion_error::CliCode::E502.number(),
                        "option",
                        &format!("unknown option --{}", other),
                        "run `prionc build -h` to see the options this command supports",
                    ))
                }
            }
        } else if a.starts_with('-') && a.len() > 1 {
            match a {
                "-h" => o.help = true,
                other => {
                    return Err(render_nofile(
                        prion_error::CliCode::E502.number(),
                        "option",
                        &format!("unknown option {}", other),
                        "run `prionc build -h` to see the options this command supports",
                    ))
                }
            }
        } else if o.name.is_none() {
            o.name = Some(a.to_string());
        } else {
            return Err(render_nofile(
                prion_error::CliCode::E504.number(),
                "command line",
                &format!("unexpected argument \"{}\"", a),
                "remove it - a command takes at most one name",
            ));
        }
        i += 1;
    }
    Ok(o)
}

fn command_help(cmd: &str) -> String {
    let what = COMMAND_HELP
        .iter()
        .find(|(c, _)| *c == cmd)
        .map(|(_, w)| (*w).to_string())
        .unwrap_or_default();
    format!("prionc {} —— {}", cmd, what)
}


struct Target {
    
    root: PathBuf,
    
    name: String,
    
    src: PathBuf,
}

fn resolve(name: Option<&str>) -> Result<Target, String> {
    let cwd = env::current_dir().map_err(|e| {
        render_nofile(
            prion_error::EnvCode::E602.number(),
            "read current directory",
            &format!("cannot get the current directory: {}", e),
            "check that the current directory exists and is accessible",
        )
    })?;

    if let Some(n) = name {
        let as_file = cwd.join(format!("{}.an", n));
        if as_file.is_file() {
            return single(as_file);
        }
        let as_dir = cwd.join(n);

        if as_dir.is_dir() {
            if as_dir.join("main.an").is_file() {
                
                let base = as_dir
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| n.to_string());
                return Ok(Target { root: as_dir.clone(), name: base, src: as_dir });
            }
            if as_dir.join("lib.prion").is_file() {
                return project(as_dir, n.to_string());
            }
            return Err(render_nofile(
                prion_error::CliCode::E505.number(),
                "locate project",
                &format!("directory \"{}\" has no main.an - the project entry is always main.an", n),
                "put a main.an in the directory root, or point the name to another project",
            ));
        }
        
        if cwd.join("lib.prion").is_file() {
            let kv = read_kv(&cwd.join("lib.prion"));
            if kv_get(&kv, "name") == Some(n) {
                return project(cwd.clone(), n.to_string());
            }
        }
        return Err(render_nofile(
            prion_error::CliCode::E505.number(),
            "locate project",
            &format!("cannot find \"{}\" - no {}.an in the current directory and no same-named project directory", n, n),
            &format!("check the spelling, or create {}.an first", n),
        ));
    }

    
    if cwd.join("lib.prion").is_file() {
        let kv = read_kv(&cwd.join("lib.prion"));
        let pkg = kv_get(&kv, "name")
            .map(String::from)
            .unwrap_or_else(|| file_name_of(&cwd));
        return project(cwd.clone(), pkg);
    }
    let mut ans = list_an(&cwd)?;
    match ans.len() {
        1 => single(ans.remove(0)),
        0 => Err(render_nofile(
            prion_error::CliCode::E505.number(),
            "locate source file",
            "the current directory has neither lib.prion nor a .an source file - nothing to compile",
            "pass a name (e.g. `prionc build hello`), or create a .an first",
        )),
        _ => Err(render_nofile(
            prion_error::CliCode::E505.number(),
            "locate source file",
            "the current directory has multiple .an files - cannot tell which to compile",
            "pass a name (e.g. `prionc build hello`)",
        )),
    }
}

fn single(f: PathBuf) -> Result<Target, String> {
    let name = f
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| {
            render_nofile(
                prion_error::InternalCode::E701.number(),
                "",
                &format!("cannot get file name: {}", f.display()),
                "",
            )
        })?;
    let root = f
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(Target { root, name, src: f })
}

fn project(root: PathBuf, name: String) -> Result<Target, String> {
    
    let mut ans = list_an(&root)?;
    let src = match ans.len() {
        1 => ans.remove(0),
        0 => {
            return Err(render_nofile(
                prion_error::CliCode::E505.number(),
                "locate source file",
                &format!("project \"{}\" has no .an source file", name),
                "the project root must contain one .an",
            ))
        }
        _ => {
            return Err(render_nofile(
                prion_error::CliCode::E505.number(),
                "locate source file",
                &format!("project \"{}\" has multiple .an files - one file per program", name),
                "keep only one .an",
            ))
        }
    };
    Ok(Target { root, name, src })
}

fn list_an(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let rd = fs::read_dir(dir).map_err(|e| {
        render_nofile(
            prion_error::EnvCode::E602.number(),
            "read directory",
            &format!("cannot read {}: {}", dir.display(), e),
            "check that the directory exists and is readable",
        )
    })?;
    let mut out = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "an").unwrap_or(false) {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

fn file_name_of(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}


fn read_kv(p: &Path) -> Vec<(String, String)> {
    let text = match fs::read_to_string(p) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.push((k.trim().to_string(), v.trim().trim_matches('"').to_string()));
        }
    }
    out
}

fn kv_get<'a>(kv: &'a [(String, String)], k: &str) -> Option<&'a str> {
    kv.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
}


fn resolve_config(t: &Target, o: &Opts) -> Result<(String, u32), ExitCode> {
    let kv = read_kv(&t.root.join("build.prion"));
    let f_mode = kv_get(&kv, "mode").map(String::from);
    let f_opt = kv_get(&kv, "opt").and_then(|v| v.parse::<u32>().ok());

    let mode = o.mode.clone().or(f_mode).unwrap_or_else(|| "debug".into());
    if mode != "debug" && mode != "release" {
        return Err(usage_err(render_nofile(
            prion_error::CliCode::E503.number(),
            "option",
            &format!("`--mode` accepts only debug|release, got {} (same in `build.prion`)", mode),
            "use `--mode debug` or `--mode release`",
        )));
    }
    let opt = o.opt.or(f_opt).unwrap_or(if mode == "debug" { 0 } else { 3 });
    if opt > 3 {
        return Err(usage_err(render_nofile(
            prion_error::CliCode::E503.number(),
            "option",
            &format!("`--opt` accepts only 0-3, got {}", opt),
            "use a number between 0 and 3",
        )));
    }
    Ok((mode, opt))
}



fn do_fmt(args: &[String]) -> ExitCode {
    let 只查 = args.iter().any(|a| a == "--check");
    let 名 = args.iter().find(|a| !a.starts_with('-'));
    let 路径 = match 名 {
        Some(s) => {
            let p = PathBuf::from(s);
            if p.exists() {
                p
            } else {
                let 加扩展 = p.with_extension("an");
                if 加扩展.exists() {
                    加扩展
                } else {
                    match resolve(Some(s)) {
                        Ok(t) => t.src,
                        Err(e) => return fail101(e),
                    }
                }
            }
        }
        None => match resolve(None) {
            Ok(t) => t.src,
            Err(e) => return fail101(e),
        },
    };
    match fmt::执行(&路径, 只查) {
        Ok((数, 改)) => {
            for f in &改 {
                println!("{}", f.display());
            }
            if 只查 {
                if 改.is_empty() {
                    println!("{} file(s) already formatted", 数);
                    ExitCode::SUCCESS
                } else {
                    println!("{} / {} file(s) need formatting", 改.len(), 数);
                    ExitCode::from(1)
                }
            } else {
                println!("formatted {} / {} file(s)", 改.len(), 数);
                ExitCode::SUCCESS
            }
        }
        Err(e) => fail101(render_nofile(
            prion_error::EnvCode::E602.number(),
            "format",
            &e,
            "check that the path exists and is writable",
        )),
    }
}

fn run_command(cmd: &str, rest: &[String]) -> ExitCode {
    let o = match parse_opts(rest) {
        Ok(o) => o,
        Err(e) => return usage_err(e),
    };
    if o.help {
        println!("{}", command_help(cmd));
        return ExitCode::SUCCESS;
    }
    
    prion_lower::quota::设(
        o.const_depth.unwrap_or(prion_lower::quota::默认深度),
        o.const_steps.unwrap_or(prion_lower::quota::默认步数),
    );

    let t = match resolve(o.name.as_deref()) {
        Ok(t) => t,
        Err(e) => return fail101(e),
    };

    match cmd {
        "check" => {
            if o.dump_tokens {
                do_dump_tokens(&t)
            } else if o.dump_ast {
                do_dump_ast(&t)
            } else {
                do_check(&t)
            }
        }
        "frontend" => do_frontend(&t),
        "clean" => do_clean(&t),
        "build" | "run" => {
            let (mode, opt) = match resolve_config(&t, &o) {
                Ok(v) => v,
                Err(e) => return e,
            };
            
            let exe = match do_build(&t, &mode, opt, o.dump_ir, o.debug_info.unwrap_or(false), o.march_native.unwrap_or(false), o.loop_reclaim) {
                Ok((_, exe)) => exe,
                Err(e) => return e,
            };
            if cmd == "build" {
                ExitCode::SUCCESS
            } else {
                do_run(&exe, &o.pass_args)
            }
        }
        _ => unreachable!(),
    }
}





fn collect_module_refs(段: &[prion_lexer::Token], out: &mut Vec<String>) {
    let mut i = 0usize;
    while i < 段.len() {
        if matches!(段[i], prion_lexer::Token::HashHashColon) {
            
            let mut name = String::new();
            let mut j = i + 1;
            while let Some(tok) = 段.get(j) {
                match tok {
                    prion_lexer::Token::Ident(n) | prion_lexer::Token::Str(n) => {
                        if !name.is_empty() {
                            name.push('.');
                        }
                        name.push_str(n);
                        j += 1;
                        if matches!(段.get(j), Some(prion_lexer::Token::Sym("."))) {
                            j += 1;
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            if !name.is_empty() {
                out.push(name);
            }
            i = j.max(i + 1);
            continue;
        }
        i += 1;
    }
}


fn stmts_module_refs(stmts: &[prion_parser::Stmt], out: &mut Vec<String>) {
    for s in stmts {
        match s {
            prion_parser::Stmt::Line { kind, .. } => collect_module_refs(kind.tokens(), out),
            prion_parser::Stmt::Struct { head, body, .. }
            | prion_parser::Stmt::Fn { head, body, .. } => {
                collect_module_refs(head, out);
                stmts_module_refs(body, out);
            }
        }
    }
}



fn collect_by_tail(root: &Path, tail: &str, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect_by_tail(root, tail, &p, out);
        } else if p.extension().map(|x| x == "an").unwrap_or(false) {
            if p.file_name().map(|s| s == "main.an").unwrap_or(false) {
                continue;
            }
            let rel = match p.strip_prefix(root) {
                Ok(r) => r,
                Err(_) => continue,
            };
            
            let mut parts: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            let stem = parts.pop().unwrap_or_default();
            let stem = stem.trim_end_matches(".an").to_string();
            parts.push(stem);
            let full = parts.join(".");
            if full.rsplit('.').next().unwrap_or(&full) == tail {
                out.push((full, p));
            }
        }
    }
}


fn std_root() -> Option<PathBuf> {
    if let Ok(v) = env::var("PRION_STD") {
        if !v.trim().is_empty() {
            return Some(PathBuf::from(v));
        }
    }
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    for p in [
        dir.join("..").join("lib").join("prion"),
        dir.join("..").join("..").join("std"),
    ] {
        if p.join("strings.an").is_file() {
            return Some(p);
        }
    }
    None
}

fn load_project(dir: &Path) -> Result<AstProgram, ExitCode> {
    let entry = dir.join("main.an");
    if !entry.is_file() {
        return Err(fail101(render_nofile(
            prion_error::CliCode::E505.number(),
            "locate project entry",
            &format!("directory \"{}\" has no main.an - the project entry is always main.an", dir.display()),
            "put a main.an in the directory root",
        )));
    }
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut blocks: Vec<prion_parser::Block> = Vec::new();
    let mut queue: std::collections::VecDeque<(String, PathBuf)> = std::collections::VecDeque::new();
    queue.push_back((String::new(), entry));
    while let Some((modname, path)) = queue.pop_front() {
        if !seen.insert(modname.clone()) {
            continue; 
        }
        let text = fs::read_to_string(&path).map_err(|e| {
            fail101(render_nofile(
                prion_error::EnvCode::E602.number(),
                "read source file",
                &format!("cannot read {}: {}", path.display(), e),
                "check that the file exists and is readable",
            ))
        })?;
        let 段 = lex(&text)
            .map_err(|e| fail101(render_front(&e.code, &e.parts, &path, &text, e.line)))?;
        let (ast, errs) = prion_parser::parse_all(段);
        if !errs.is_empty() {
            let 全: Vec<String> = errs
                .iter()
                .map(|e| render_front(&e.code, &e.parts, &path, &text, e.line))
                .collect();
            return Err(fail101(格式语法错(&全)));
        }

        let mut deps: Vec<String> = Vec::new();
        for b in &ast.blocks {
            stmts_module_refs(&b.stmts, &mut deps);
        }
        
        
        let mut ast = ast;
        for b in &mut ast.blocks {
            b.module = modname.clone();
            
            b.file = path.display().to_string();
        }
        blocks.extend(ast.blocks); 
        for dep in deps {
            
            if let Some(rest) = dep.strip_prefix("std.") {
                let root = std_root().ok_or_else(|| {
                    fail101(render_nofile(
                        prion_error::CliCode::E505.number(),
                        "locate standard library",
                        "cannot find the standard library root - set `PRION_STD`, or place it at `<SDK>/lib/prion` per the SDK layout",
                        "put the standard library directory in that location, or set PRION_STD",
                    ))
                })?;
                let p = root
                    .join(rest.replace('.', std::path::MAIN_SEPARATOR_STR))
                    .with_extension("an");
                if p.is_file() {
                    queue.push_back((dep.clone(), p));
                    continue;
                }
                return Err(fail101(render_nofile(
                    prion_error::CliCode::E505.number(),
                    "locate standard library",
                    &format!("module \"{}\" not found in the standard library", dep),
                    &format!("confirm {} exists", p.display()),
                )));
            }
            let rel = dep.replace('.', std::path::MAIN_SEPARATOR_STR);
            let direct = dir.join(&rel).with_extension("an");
            let want = direct.display().to_string();

            
            let resolved: Option<(String, std::path::PathBuf)> = if direct.is_file() {
                Some((dep.clone(), direct))
            } else {
                let tail = dep.rsplit('.').next().unwrap_or(&dep).to_string();
                let mut hits: Vec<(String, std::path::PathBuf)> = Vec::new();
                collect_by_tail(dir, &tail, dir, &mut hits);
                match hits.len() {
                    1 => hits.into_iter().next(),
                    0 => None,
                    _ => {
                        let names: Vec<String> = hits.iter().map(|(m, _)| m.clone()).collect();
                        return Err(fail101(render_nofile(
                            prion_error::CliCode::E505.number(),
                            "locate module",
                            &format!(
                                "module \"{}\" is ambiguous - {} all use this name; write the full path",
                                dep,
                                names.join(", ")
                            ),
                            &format!("change the reference to `##:{}` (or another full path)", names[0]),
                        )));
                    }
                }
            };
            match resolved {
                Some((full, p)) => queue.push_back((full, p)),
                None => {
                    return Err(fail101(render_nofile(
                        prion_error::CliCode::E505.number(),
                        "locate module",
                        &format!("module \"{}\" not found - module names map to directory paths", dep),
                        &format!("confirm {} exists, or write the full reference", want),
                    )))
                }
            }
        }
    }
    Ok(AstProgram { blocks })
}

fn front(t: &Target) -> Result<AstProgram, ExitCode> {

    if t.src.is_dir() {
        return load_project(&t.src);
    }
    let text = match fs::read_to_string(&t.src) {
        Ok(x) => x,
        Err(e) => {
            return Err(fail101(render_nofile(
                prion_error::EnvCode::E602.number(),
                "read source file",
                &format!("cannot read {}: {}", t.src.display(), e),
                "check that the file exists and is readable",
            )))
        }
    };
    if text.starts_with('\u{feff}') {
        return Err(fail101(render_file(
            prion_error::LexCode::E007.number(),
            "read source file",
            "source file has a BOM - the contract requires UTF-8 without BOM",
            "set the editor encoding to \"UTF-8 (no BOM)\" and recompile",
            &t.src,
        )));
    }
    let 段 = match lex(&text) {
        Ok(x) => x,
        Err(e) => return Err(fail101(render_front(&e.code, &e.parts, &t.src, &text, e.line))),
    };
    let (ast, errs) = prion_parser::parse_all(段);
    if !errs.is_empty() {
        let 全: Vec<String> = errs
            .iter()
            .map(|e| render_front(&e.code, &e.parts, &t.src, &text, e.line))
            .collect();
        return Err(fail101(格式语法错(&全)));
    }
    Ok(ast)
}


fn render_coded(code: &str, message: &str, line: usize, err_file: &std::path::Path, text: &str) -> String {
    
    let file = err_file
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    
    let eff_text = if err_file.as_os_str().is_empty() {
        text.to_string()
    } else {
        std::fs::read_to_string(err_file).unwrap_or_default()
    };
    let lines: Vec<&str> = eff_text.lines().collect();
    let src_line = if line > 0 && line <= lines.len() {
        lines[line - 1]
    } else {
        ""
    };
    let 哪 = if line > 0 {
        format!("line {}", line)
    } else {
        String::new()
    };
    let (de, dd) = prion_error::code_defaults(code);
    let parts = prion_error::DiagParts::full(哪, "", message, "", "")
        .预期(de)
        .详解(dd);
    let cx = prion_error::RenderCtx { code, file: &file, src_line };
    prion_error::render_full(&parts, prion_error::Tone::Fix, &cx)
}


fn render_front(
    code: &str,
    parts: &prion_error::DiagParts,
    src: &std::path::Path,
    text: &str,
    line: usize,
) -> String {
    let file = src
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let src_line = if line > 0 && line <= lines.len() {
        lines[line - 1]
    } else {
        ""
    };
    let (de, dd) = prion_error::code_defaults(code);
    let mut p = parts.clone();
    if p.预期.is_empty() {
        p.预期 = de.to_string();
    }
    if p.详解.is_empty() {
        p.详解 = dd.to_string();
    }
    let cx = prion_error::RenderCtx { code, file: &file, src_line };
    prion_error::render_full(&p, prion_error::Tone::Fix, &cx)
}

fn print_diags(checks: &prion_check::CheckResult, src_path: &std::path::Path, src_text: &str) {
    eprintln!("check failed with {} error(s):", checks.errors().len());
    let mut cache: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for d in &checks.diags {
        
        let (file, text): (String, &str) = if d.file.is_empty() {
            (
                src_path
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                src_text,
            )
        } else {
            let key = d.file.clone();
            if !cache.contains_key(&key) {
                let t = fs::read_to_string(&key).unwrap_or_default();
                cache.insert(key.clone(), t);
            }
            let name = std::path::Path::new(&key)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| key.clone());
            (name, cache.get(&key).map(|s| s.as_str()).unwrap_or(""))
        };
        let lines: Vec<&str> = text.lines().collect();
        let src_line = if d.line > 0 && d.line <= lines.len() {
            lines[d.line - 1]
        } else {
            ""
        };
        let 哪 = if d.col > 0 {
            format!("line {}, column {}", d.line, d.col)
        } else if d.line > 0 {
            format!("line {}", d.line)
        } else {
            String::new()
        };
        let mut parts = d.parts.clone();
        if !哪.is_empty() {
            parts.哪 = 哪;
        }
        let ctx = prion_error::RenderCtx { code: &d.code, file: &file, src_line };
        let full = prion_error::render_full(&parts, d.tone, &ctx);
        if full.trim().is_empty() || parts.缺.is_empty() {
            eprintln!("  {}", d.message); 
        } else {
            eprintln!("  {}", full);
        }
    }
}



fn do_frontend(t: &Target) -> ExitCode {
    use std::time::Instant;
    let src_file = if t.src.is_dir() {
        t.src.join("main.an")
    } else {
        t.src.clone()
    };
    let src_text = fs::read_to_string(&src_file).unwrap_or_default();
    let lines = src_text.lines().count();
    let bytes = src_text.len();

    let mut best = f64::MAX;
    #[allow(unused_mut)]
    let mut n_fn = 0usize;
    for _ in 0..3 {
        let t0 = Instant::now();
        let ast = match front(t) {
            Ok(a) => a,
            Err(e) => return e,
        };
        let t_parse = t0.elapsed().as_secs_f64();
        let checks = check(&ast);
        let t_check = t0.elapsed().as_secs_f64();
        if !checks.ok() {
            print_diags(&checks, &src_file, &src_text);
            return ExitCode::from(101);
        }
        #[cfg(feature = "backend")]
        let ir = match build_ir(&ast, &checks.typeck) {
            Ok(p) => p,
            Err(e) => {
                return fail101(render_coded(
                    e.code.as_str(),
                    &e.message,
                    e.line,
                    std::path::Path::new(&e.file),
                    &src_text,
                ))
            }
        };
        let dt = t0.elapsed().as_secs_f64();
        eprintln!("[phases] parse {:.3}s | check {:.3}s | ir {:.3}s", t_parse, t_check - t_parse, dt - t_check);
        if dt < best {
            best = dt;
            #[cfg(feature = "backend")]
            {
                n_fn = ir.functions.len();
            }
        }
    }
    println!(
        "[frontend] time {:.3}s | lines {} | bytes {} | functions {} | throughput {:.0} lines/s",
        best,
        lines,
        bytes,
        n_fn,
        if best > 0.0 { lines as f64 / best } else { 0.0 }
    );
    ExitCode::SUCCESS
}


fn 令牌码(t: &prion_lexer::Token) -> u32 {
    use prion_lexer::Token as T;
    match t {
        T::Newline => 0,
        T::RawAsm(_) => 1,
        T::Str(_) => 2,
        T::RawStr(_) => 3,
        T::Int(_) => 4,
        T::Float(_) => 5,
        T::Ident(_) => 6,
        T::Kw(_) => 7,
        T::Sym(_) => 8,
        T::HashColon => 9,
        T::HashHashColon => 10,
        T::HashHashHashColon => 11,
    }
}


fn do_dump_tokens(t: &Target) -> ExitCode {
    let src_file = if t.src.is_dir() { t.src.join("main.an") } else { t.src.clone() };
    let src_text = match fs::read_to_string(&src_file) {
        Ok(s) => s,
        Err(e) => {
            return fail101(render_nofile(
                prion_error::EnvCode::E602.number(),
                "read source file",
                &format!("cannot read {}: {}", src_file.display(), e),
                "check that the path exists and is readable",
            ))
        }
    };
    match prion_lexer::lex(&src_text) {
        Ok(段) => {
            let mut 出 = String::new();
            for t in 段 {
                
                出.push_str(&format!("{}\n{}\n{}\n{}\n", t.line, t.col, t.len, 令牌码(&t.tok)));
            }
            print!("{}", 出);
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("ERR\n{}\n{}", e.line, e.code);
            ExitCode::from(101)
        }
    }
}


fn do_dump_ast(t: &Target) -> ExitCode {
    let src_file = if t.src.is_dir() { t.src.join("main.an") } else { t.src.clone() };
    let src_text = match fs::read_to_string(&src_file) {
        Ok(s) => s,
        Err(e) => {
            return fail101(render_nofile(
                prion_error::EnvCode::E602.number(),
                "read source file",
                &format!("cannot read {}: {}", src_file.display(), e),
                "check that the path exists and is readable",
            ))
        }
    };
    let 段 = match prion_lexer::lex(&src_text) {
        Ok(v) => v,
        Err(e) => {
            println!("ERR\n{}\n{}", e.line, e.code);
            return ExitCode::from(101);
        }
    };
    let (ast, errs) = prion_parser::parse_all(段);
    if let Some(e) = errs.first() {
        println!("ERR\n{}\n{}", e.line, e.code);
        return ExitCode::from(101);
    }
    let mut 出 = String::new();
    for b in &ast.blocks {
        快照块(b, &mut 出);
    }
    print!("{}", 出);
    ExitCode::SUCCESS
}


fn 快照块(b: &prion_parser::Block, out: &mut String) {
    for s in &b.stmts {
        快照句(s, 1, out);
    }
    let attrs = b.attrs.join(",");
    out.push_str(&format!(
        "B\t0\t{}\t{}\t{}\t{}\t{}\t{}\n",
        b.line,
        b.col,
        b.len,
        b.name,
        attrs,
        b.asm_lines.len()
    ));
}


fn 快照句(s: &prion_parser::Stmt, d: usize, out: &mut String) {
    use prion_parser::Stmt as S;
    let (kind, line, col, len, head, body) = match s {
        S::Line { line, col, len, kind, .. } => ("L", *line, *col, *len, kind.tokens(), None),
        S::Struct { line, col, len, head, body, .. } => ("S", *line, *col, *len, head, Some(body)),
        S::Fn { line, col, len, head, body } => ("F", *line, *col, *len, head, Some(body)),
    };
    if let Some(body) = body {
        for x in body {
            快照句(x, d + 1, out);
        }
    }
    out.push_str(&format!("{}\t{}\t{}\t{}\t{}\t{}", kind, d, line, col, len, head.len()));
    for i in 0..head.len() {
        let (c, l) = head.span(i).unwrap_or((0, 0));
        out.push_str(&format!("\t{}\t{}\t{}", c, l, 令牌码(&head[i])));
    }
    out.push('\n');
}

fn do_check(t: &Target) -> ExitCode {
    let ast = match front(t) {
        Ok(a) => a,
        Err(e) => return e,
    };
    let checks = check(&ast);
    if checks.ok() {
        println!("check passed: {}", t.src.display());
        ExitCode::SUCCESS
    } else {
        let src_file = if t.src.is_dir() { t.src.join("main.an") } else { t.src.clone() };
        let src_text = fs::read_to_string(&src_file).unwrap_or_default();
        print_diags(&checks, &src_file, &src_text);
        ExitCode::from(101)
    }
}



fn cc_opt_args(mode: &str, opt: u32, native: bool) -> Vec<String> {
    let mut v = if mode == "debug" {
        
        vec!["-O0".to_string(), "-g".to_string()]
    } else {
        
        vec![format!("-O{}", opt), "-DPRION_NOGUARD".to_string(), "-s".to_string()]
    };
    if native {
        v.push("-march=native".to_string());
    }
    v
}

#[cfg(feature = "llvm")]
fn do_build_llvm(
    ir: &prion_ir::Program,
    src_text: &str,
    src_file: &str,
    mode: &str,
    opt: u32,
    dir: &Path,
    exe: &Path,
    name: &str,
) -> Result<(), ExitCode> {
    let cc = 找cc();
    let obj = dir.join(format!("{}.o", name));
    let 发射 = if std::env::var("PRION_TREE_LLVM").as_deref() == Ok("1") {
        prion_llvm::emit_object(ir, &obj, opt, mode == "debug", src_file)
    } else {
        let flat = prion_ir::flat::from_program(ir);
        prion_llvm::flat_emit::emit_flat_object(ir, &flat, &obj, opt, mode == "debug", src_file)
    };
    if let Err(e) = 发射 {
        return Err(fail101(render_nofile(
            prion_error::GenCode::E401.number(),
            "LLVM backend",
            &e,
            "fix the source and retry",
        )));
    }
    let mut link = Command::new(&cc);
    link.arg(&obj).arg("-o").arg(exe);
    if !cfg!(windows) {
        link.arg("-lm");
    }
    let st = link.status();
    match st {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err(fail101(render_nofile(
            prion_error::GenCode::E401.number(),
            "link",
            "failed to link the LLVM object file",
            "check that clang is available",
        ))),
        Err(e) => Err(fail101(render_nofile(
            prion_error::EnvCode::E601.number(),
            "invoke compiler",
            &format!("cannot invoke compiler \"{}\": {}", cc, e),
            "set PRION_CC to a working clang",
        ))),
    }
}



#[cfg(not(feature = "backend"))]
fn do_build(
    _t: &Target,
    _mode: &str,
    _opt: u32,
    _dump_ir: bool,
    _dbg: bool,
    _native: bool,
    _loop_reclaim: bool,
) -> Result<(PathBuf, PathBuf), ExitCode> {
    Err(fail101(
        "this prionc is a slim build (diagnostics and language service only) without code generation - use the full build to compile",
    ))
}

#[cfg(feature = "backend")]
fn do_build(t: &Target, mode: &str, opt: u32, dump_ir: bool, dbg: bool, native: bool, loop_reclaim: bool) -> Result<(PathBuf, PathBuf), ExitCode> {
    let ast = front(t)?;
    
    let src_file = if t.src.is_dir() { t.src.join("main.an") } else { t.src.clone() };
    let src_text = fs::read_to_string(&src_file).unwrap_or_default();
    let checks = check(&ast);
    if !checks.ok() {
        print_diags(&checks, &src_file, &src_text);
        return Err(ExitCode::from(101));
    }
    let mut ir = match build_ir(&ast, &checks.typeck) {
        Ok(p) => p,
        Err(e) => return Err(fail101(render_coded(e.code.as_str(), &e.message, e.line, std::path::Path::new(&e.file), &src_text))),
    };
    
    prion_opt::run(&mut ir);
    prion_walk::drop_pass::对象挂名(&mut ir, loop_reclaim);
    if dump_ir {
        println!("{}", ir.dump());
        let flat = prion_ir::flat::from_program(&ir);
        println!("{}", prion_ir::flat::dump(&flat));
    }
    let dir = t.root.join("build").join(mode);
    if let Err(e) = fs::create_dir_all(&dir) {
        return Err(fail101(render_nofile(
            prion_error::EnvCode::E602.number(),
            "create output directory",
            &format!("cannot create output directory {}: {}", dir.display(), e),
            "check that the path is writable",
        )));
    }
    let mut exe = dir.join(&t.name);
    if cfg!(windows) {
        exe.set_extension("exe");
    }

    #[cfg(feature = "llvm")]
    {
        let fname = src_file.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        do_build_llvm(&ir, &src_text, &fname, mode, opt, &dir, &exe, &t.name)?;
        return Ok((dir, exe));
    }
    #[cfg(not(feature = "llvm"))]
    {
        let _ = (mode, opt, dir, exe);
        return Err(fail101(
            "this prionc build has the LLVM backend disabled - rebuild with the default features",
        ));
    }
}

fn do_run(exe: &Path, args: &[String]) -> ExitCode {
    let st = match Command::new(exe).args(args).status() {
        Ok(s) => s,
        Err(e) => {
            return fail101(render_nofile(
                prion_error::EnvCode::E603.number(),
                "run program",
                &format!("cannot run {}: {}", exe.display(), e),
                "check that the output exists and is executable",
            ))
        }
    };
    match st.code() {
        Some(0) => ExitCode::SUCCESS,
        Some(c) if (1..=255).contains(&c) => ExitCode::from(c as u8),
        Some(c) => fail101(render_nofile(
            prion_error::EnvCode::E604.number(),
            "forward exit code",
            &format!("program exit code {} (out of 8-bit range, cannot forward)", c),
            "the exit code must be between 0 and 255 to be forwarded as-is",
        )),
        None => fail101(render_nofile(
            prion_error::EnvCode::E604.number(),
            "forward exit code",
            "program was terminated by a signal (no exit code to forward)",
            "check whether the program was interrupted by Ctrl-C, killed by the system, or crashed",
        )),
    }
}



fn do_clean(t: &Target) -> ExitCode {
    let bd = t.root.join("build");
    if !bd.exists() {
        println!("nothing to clean ({} does not exist)", bd.display());
        return ExitCode::SUCCESS;
    }
    match fs::remove_dir_all(&bd) {
        Ok(_) => {
            println!("cleaned: {}", bd.display());
            ExitCode::SUCCESS
        }
        Err(e) => fail101(render_nofile(
            prion_error::EnvCode::E602.number(),
            "clean output directory",
            &format!("cannot clean {}: {}", bd.display(), e),
            "check permissions and whether the directory is in use by another program",
        )),
    }
}

fn 格式语法错(各: &[String]) -> String {
    if 各.len() == 1 {
        return 各[0].clone();
    }
    format!("{} ({} syntax error(s) total)", 各.join("\n"), 各.len())
}
