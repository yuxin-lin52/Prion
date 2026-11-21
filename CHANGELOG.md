# 更新日志

## 0.3.0 — 2026-11-22

本版将语言更名为 **Prion**（原 Anle）：编译器可执行文件由 `anlec` 改为 `prionc`，环境变量由 `ANLE_CC` / `ANLE_STD` 改为 `PRION_CC` / `PRION_STD`。语法尚未冻结，本版含多项破坏性变更。

### 新增

- 并发：`spawn` / `join` / `share` / `send`，支持任务失败随 `join` 传播、容器内 token 批量 join、非复制值单次移交。
- 表达式与类型：`if` 表达式、错误传播 `?`、可空类型 `?T`、精化类型（如 `int(>0)`）；`match` 支持 `case ... if` 守卫。
- 类型声明：类型别名（`add type "Len" = int`）；newtype（单字段位置取值，如 `.1`）。
- 常量：`fix` 的独立引用动词 `call fix`，支持跨块与跨模块可见性；编译期求值提供 `--const-depth` / `--const-steps` 配额。
- 模块：字段级 `hide` 的跨模块限制。
- 内置：`sqrt`、`print_raw`、`now`（单调纳秒时钟）。
- 语法：名字的引号可省略；词表加入 `trait` / `impl` / `spawn` / `share` / `send`，其中 `trait` 与 `impl` 为保留字，尚未实现。

### 优化

- LLVM 后端接入优化管线，并引入 SSA、循环不变量外提、常量折叠、内联与不可达块消除。
- 字符串改为指针加长度（胖指针），配合手写十进制转换、分级分配池与顺序游标，消除多处平方级开销。
- 容器：`map` 改为桶索引与插入序条目、`str` 键内联描述符；列表元素写直存、下标类型化直读、根容量自动预留。
- 编译：发布态去除追踪、按语句数并行编译、接入 LLVM 优化管线。
- 运行期：release 档护栏全抽；错误位置改为调用点常量传参；纯计算函数不再写文件名。
- 内置 `sqrt` 走 `llvm.sqrt.f64`（硬件开方，可内联与向量化）。
- 移除独立运行时后，既有的圈级回收、块单元释放、整树释放等回收能力保持不变。

### 修复

- 内存：修复块出口逐值释放、容器元素归属释放、跨块多值槽顺序错位、字符串旧值释放等多处释放与拷贝缺陷。
- 并发：修复容器捕获的悬垂与双释放；移交值只允许一次。
- 字符串：修复非法 UTF-8 下 `nth_char` 越界、`str(int)` 丢最高位、空路径转本地码页后释放导致的堆损坏。
- 后端：修复 `X*` 形参直接返回句柄、根槽初始化、`if` 表达式结果临时类型、`list[i].字段 = 值` 按值写回、跨模块泛型调用与函数值类型等缺陷。
- 修复根块参数的零初始化：此前 `float` 形参会触发编译器内部错误；现已按参数类型初始化。

### 移除

- 移除 C 后端；`--backend` 选项随之删除，LLVM 为唯一后端。
- 移除独立 C 运行时；运行时辅助改由编译器直接发射 IR。
- 移除 `kill` 语句与 `d kill` 后缀；内存回收完全由块作用域决定。
- 移除三元条件运算符 `?:`；`?` 专用于错误传播。

---

# Changelog

## 0.3.0 — 2026-11-22

This release renames the language to **Prion** (formerly Anle): the compiler binary changes from `anlec` to `prionc`, and the environment variables from `ANLE_CC` / `ANLE_STD` to `PRION_CC` / `PRION_STD`. The syntax is not frozen; this release contains several breaking changes.

### Added

- Concurrency: `spawn` / `join` / `share` / `send`, including failure propagation through `join`, batch join of token containers, and single-move semantics for non-copyable values.
- Expressions and types: `if` expressions, error propagation with `?`, nullable types `?T`, and refined types such as `int(>0)`; `match` supports `case ... if` guards.
- Type declarations: type aliases (`add type "Len" = int`); newtypes with positional access such as `.1`.
- Constants: the dedicated verb `call fix`, with cross-block and cross-module visibility; `--const-depth` / `--const-steps` quotas for compile-time evaluation.
- Modules: field-level `hide` enforced across modules.
- Builtins: `sqrt`, `print_raw`, and `now` (monotonic nanosecond clock).
- Syntax: names may be written without quotes; the keyword set gains `trait` / `impl` / `spawn` / `share` / `send`, of which `trait` and `impl` are reserved and not implemented in this release.

### Improved

- The LLVM backend now runs an optimization pipeline and uses SSA, loop-invariant code motion, constant folding, inlining and unreachable-block elimination.
- Strings use a pointer-plus-length representation (fat pointer), with handwritten decimal conversion, tiered allocation pools and sequential cursors, removing several quadratic cost paths.
- Containers: maps use bucket indexing with insertion-order entries and inline `str` key descriptors; list element writes are direct, indexed reads are typed, and root capacity is reserved automatically.
- Compilation: trace removal in release mode, per-statement parallel compilation, and the LLVM optimization pipeline.
- Runtime: all guards are stripped in release builds; runtime error positions are passed as call-site constants; pure functions no longer write a file name.
- `sqrt` lowers to `llvm.sqrt.f64` (hardware square root, inlinable and vectorizable).
- After the separate runtime was removed, the existing reclamation capabilities — cycle-level reclamation, block-unit release and whole-tree release — are preserved.

### Fixed

- Memory: multiple release and copy defects, including per-value release at block exit, ownership-based release of container elements, cross-block multi-value slot ordering, and release of overwritten string values.
- Concurrency: dangling handles and double frees when containers are captured; a moved value may now be moved only once.
- Strings: `nth_char` reading out of bounds on invalid UTF-8, `str(int)` dropping the high bit, and heap corruption when an empty path was freed after code-page conversion.
- Backend: `X*` parameters returning the handle directly, root-slot initialization, the temporary type of `if`-expression results, value-wise write-back for `list[i].field = value`, cross-module generic calls and function-value types.
- Root-block parameter zero-initialization: a `float` parameter previously triggered an internal compiler error; parameters are now initialized by their own type.

### Removed

- The C backend; the `--backend` option is gone and LLVM is the only backend.
- The separate C runtime; runtime helpers are now emitted directly as IR by the compiler.
- The `kill` statement and the `d kill` suffix; reclamation is determined entirely by block scope.
- The ternary conditional operator `?:`; `?` is now used only for error propagation.
