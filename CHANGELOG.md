# Changelog

## [0.3.0] - 2026-11-22

The first release under the **Prion** name, and the first with LLVM as the only backend. The language is not frozen; several changes are breaking.

### Breaking changes

- Renamed the project to **Prion** (formerly Anle): crate directories `anle-*` → `prion-*`, compiler binary `anlec` → `prionc`.
- Renamed environment variables: `ANLE_CC` → `PRION_CC`, `ANLE_STD` → `PRION_STD`.
- LLVM is the only backend. The C backend is removed, and the `--backend` option is gone; passing it is rejected.
- Removed the `kill` statement (early reclamation). Memory is released when the owning block ends; `unsafe` blocks now expose only raw address access, C interoperability and inline assembly.

### Added

- Diagnostics are now in English, in four parts: the problem, the source line, `Expected`, and `Fix`, plus `Details`.
- A tutorial-style bilingual guide (English / Chinese, 15 chapters each); every chapter's examples were compiled and run.
- CI builds the compiler, compiles the `physics-engine` demo, runs it and checks the output.

### Fixed

- LLVM backend: root-block parameters are zero-initialized by their own type. A `float` parameter previously triggered an internal error, so `physics-engine` did not build; it now runs and prints `collisions: 7343`.
- README, issue template and build instructions aligned with the LLVM-only, `prionc`-named compiler.

### Internal

- The compiler workspace has 12 crates, adding the `lower`, `walk` and `opt` stages.

---

# 更新日志

## [0.3.0] - 2026-11-22

首个以 **Prion** 名义发布、且只保留 LLVM 后端的版本。语言尚未冻结，本版含多项破坏性变更。

### 破坏性变更

- 正式更名为 **Prion**（旧称 Anle）：crate 目录 `anle-*` → `prion-*`，编译器可执行文件 `anlec` → `prionc`。
- 环境变量改名：`ANLE_CC` → `PRION_CC`，`ANLE_STD` → `PRION_STD`。
- 后端只保留 **LLVM**：C 后端移除，`--backend` 选项删除，传入会被拒绝。
- 移除 `kill` 语句（显式提前回收）。内存由所属块结束时释放；`unsafe` 块现在只提供裸地址读写、C 互操作与内联汇编。

### 新增

- 诊断全部改为**英文**，统一四段式：问题、源码行、`Expected`、`Fix`，另附 `Details`。
- 教程式双语文档（英文 / 中文各 15 章），每章示例均经编译运行核对。
- CI 覆盖「构建编译器 → 编译 physics-engine → 运行并校验输出」。

### 修复

- LLVM 后端：根块参数的零初始化改为按参数类型进行。此前 `float` 参数会触发内部错误，physics-engine 无法编译；现已恢复，输出 `collisions: 7343`。
- README、issue 模板、构建说明与「仅 LLVM、`prionc` 命名」的现状对齐。

### 内部

- 编译器工作区共 12 个 crate，新增 `lower` / `walk` / `opt` 阶段。
