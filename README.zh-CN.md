<h1 align="center">Prion</h1>

<p align="center"><a href="https://github.com/yuxin-lin52/Prion/actions/workflows/build.yml"><img src="https://github.com/yuxin-lin52/Prion/actions/workflows/build.yml/badge.svg" alt="build"></a></p>

<p align="center"><a href="README.md">English</a> | 中文</p>

Prion 是一门 AOT 编译语言。内存由编译器管理：回收点从程序的块结构算出，运行时没有垃圾回收器、没有停顿、没有逐对象计数。

## 设计动机

Prion 建立在一个分工之上：**写得简单，实现得复杂。**

每门语言都要回答一个问题：内存谁负责？手动管理把正确性交给程序员；运行时管理省掉了这项工作，但带来额外负担和运行时卡顿；编译器检查的所有权保证了正确性，代价是程序员必须学会并满足一套规则。

Prion 三种取舍都不取。程序按块结构化，值长在声明它的块里，一切跨块数据流动都走显式通道。在此结构下，编译器算出每个值的寿命与回收点。程序员写块、声明数据流向，其余由编译器完成。

整个语言都遵循同一个分工。语法小而规整。直接内存访问、C 互操作与内联汇编只存在于 `unsafe` 块内。Prion 没有手动内存分配。内存的释放点由编译器从块结构算出：每块在自己的区域里工作，块的链跑完时，它的段一步交还给平台分配器。

## 语言概览

```anle
#: 主块 b
    add val "和" int = 0
    add val "i" int = 1
    while call val "i" <= 100:
        val "和" = call val "和" + call val "i"
        val "i" = call val "i" + 1
    ;
    print(call val "和")
#: 主块 d
```

输出 `5050`。这个例子里能看到的规则：

- 所有代码写在块里。块用 `#: 名 b`（born）开、`#: 名 d`（death）关。块是平的，不嵌套。
- 名字用 `add` 创建，用 `val "名" =` 改值（`add` 不重复出现）。
- 读值写 `call val "名"`。没有裸名：每次读取都声明类别，每次跨块引用都写明来源。
- 没有隐式转换。`int` 与 `float` 不显式转换不混算。
- 条件必须是真假值——`if 5:` 是编译错误。
- 下标从 1 起，与含两端的范围符一致（`for i in 1~5` 覆盖 1 到 5）。
- 控制流体用独占一行的 `;` 收束。缩进纯属装饰、不可能改变语义；`;` 不配对是响亮的解析错误。Prion 的编辑器扩展内置自动格式化，会保持缩进一致。

## 内存模型

- 内存按块实例分配。块结束时，其值一并释放，而不是逐对象回收。
- 回收点在编译期由块引用图定死。运行时没有 GC 线程、没有全局停顿、没有引用计数。
- 跨块数据有两种显式形态：**引用槽**（`call #:块 val "名"`——读到最新值，写入直达）与**深拷**（`take #:块 val "名"`——当时的值归调用者所有）。
- 引用环不泄漏，因为回收按块作用域进行，不依赖引用计数。

这套模型给所有程序员同一个简单默认：把代码写进块里，内存自己管理好——初学者可以把整个程序写在一个块里，零内存负担。块怎么划不影响正确性，只影响内存占用。对内存或性能敏感的项目，高阶开发者可以自行调优：把程序拆成更多块，每块的内存在块结束时更早释放，峰值随之降低。

## 下载

[releases 页](https://github.com/yuxin-lin52/Prion/releases) 提供 Windows x64 的预构建编译器。它使用 LLVM 后端，因此编译程序时还需要一个链接工具链：`PATH` 里有 `clang`，或用 `PRION_CC` 指向一个。LLVM 本身已链接进编译器，无需单独安装。

## 构建

编译器源码在 [`compiler/`](compiler)，是一个 Rust 工作区。LLVM 是唯一后端。

```
cd compiler
cargo build --release -p prionc
```

LLVM 后端静态链接 LLVM 22.1：把 `LLVM_SYS_221_PREFIX` 指向 LLVM 22.1 安装目录，再用 `RUSTFLAGS="-L native=<前缀>\deps"` 把该目录下的 `deps` 加进链接搜索路径——zlib、zstd、libxml2 的导入库在那里。本仓库不随附 LLVM，请从上游发布页获取：<https://github.com/llvm/llvm-project/releases>。

```
./target/release/prionc build hello
```

## 实现状态

- 编译器由 Rust 实现，源码随仓提供。自举处于早期：自举前端的词法器已完成，与宿主实现通过逐 token 对拍。
- 后端只有一条：**LLVM**，经 `clang` 链接。
- 语法**未冻结**。破坏性变更随时可能；针对本语言写的代码需要跟着更新。
- 标准库由约 40 个内置函数构成。

## 诊断格式

诊断带四段：发生了什么、源码原文、编译器预期、改法。以英文输出。真实样例：

```
[E202] error at example.an line 2, column 5, in assignment: "v" is declared as float, the assigned value is int; source: add val "v" float = 0 - 5. Expected: the assigned value must be of type float (the declaration decides). Fix: change it to a float value, or change the declaration to int. Details: Prion does not convert types silently: int does not become str automatically. To change the type, convert explicitly with int(value) / str(value) / float(value) / bool(value); if the declaration is wrong, change the declared type to match the value.
```

## 征集反馈

本仓库公开的目的是征集设计反馈。issue 入口开放——设计问题、示例中的粗糙之处、对上述取舍的异议都欢迎。[文档](docs/README.md#%E4%B8%AD%E6%96%87)是一份上手教程。

## License

Apache 2.0，见 [LICENSE](LICENSE)。
