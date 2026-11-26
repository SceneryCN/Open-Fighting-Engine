# OFE 依赖清单
核心与 ABI 仅使用标准库和本仓库代码。P1 模拟器另使用 serde、serde_json 及其依赖解析内置 OFC 样例。可运行 cargo tree -p ofe-sim -e normal 查看实际依赖树。模拟器归档随附这棵依赖树中的上游许可、样例许可与对应源码地址。Criterion 用于测试与基准。所有 Cargo 源包由 Cargo.lock 记录版本及校验和。

|组件|版本 / 来源|用途|许可与再分发条件|
|---|---|---|---|
|Rust / Cargo|[1.100.0](https://github.com/rust-lang/rust/tree/1.100.0)|核心、静态库和测试|MIT / Apache-2.0；随标准库再分发保留适用声明，另见工具链各组件通知|
|Godot|[4.7.2 stable](https://github.com/godotengine/godot/tree/4.7.2-stable)|宿主、验证与导出模板|MIT；保留版权、LICENSE.txt 和各集成组件的 COPYRIGHT.txt|
|godot-cpp|[10.0.0-stable，507ed9d](https://github.com/godotengine/godot-cpp/tree/507ed9d840c01a3c5b2a39af8bb4000bfac30bf5)|C++ 绑定，指定 4.7 API，运行于 4.7.2 宿主|MIT；链接时保留上游版权和许可|
|CMake|[上游](https://cmake.org/licensing/)，3.22+|生成本机构建|BSD-3-Clause 及捆绑组件；工具本身不随 PoC 发行|
|Ninja|[上游](https://github.com/ninja-build/ninja)|构建执行器|Apache-2.0；工具本身不随 PoC 发行|
|cargo-nextest|[0.9.152](https://github.com/nextest-rs/nextest)|独立 Rust 测试运行器|MIT / Apache-2.0；工具本身不随 PoC 发行|
|clang-format|[LLVM](https://llvm.org/)|C++ 格式检查|Apache-2.0 WITH LLVM-exception；工具本身不随 PoC 发行|

## Cargo 依赖（含开发及所有目标）
下表来自锁定环境的 cargo metadata --locked。各版本页面链接其源代码与许可。所列许可允许在满足各自条件后再分发；常见条件包括保留许可、版权和适用 NOTICE。Unicode 数据另附相应声明。本次不分发这些工具的二进制。

|名称和版本|上游声明的许可|
|---|---|
|[aho-corasick 1.1.5](https://crates.io/crates/aho-corasick/1.1.5)|Unlicense OR MIT|
|[anes 0.1.6](https://crates.io/crates/anes/0.1.6)|MIT OR Apache-2.0|
|[anstyle 1.0.14](https://crates.io/crates/anstyle/1.0.14)|MIT OR Apache-2.0|
|[autocfg 1.5.1](https://crates.io/crates/autocfg/1.5.1)|Apache-2.0 OR MIT|
|[cast 0.3.0](https://crates.io/crates/cast/0.3.0)|MIT OR Apache-2.0|
|[cfg-if 1.0.5](https://crates.io/crates/cfg-if/1.0.5)|MIT OR Apache-2.0|
|[ciborium 0.2.2](https://crates.io/crates/ciborium/0.2.2)|Apache-2.0|
|[ciborium-io 0.2.2](https://crates.io/crates/ciborium-io/0.2.2)|Apache-2.0|
|[ciborium-ll 0.2.2](https://crates.io/crates/ciborium-ll/0.2.2)|Apache-2.0|
|[clap 4.6.10](https://crates.io/crates/clap/4.6.10)|MIT OR Apache-2.0|
|[clap_builder 4.6.10](https://crates.io/crates/clap_builder/4.6.10)|MIT OR Apache-2.0|
|[clap_lex 1.1.2](https://crates.io/crates/clap_lex/1.1.2)|MIT OR Apache-2.0|
|[criterion 0.5.1](https://crates.io/crates/criterion/0.5.1)|Apache-2.0 OR MIT|
|[criterion-plot 0.5.0](https://crates.io/crates/criterion-plot/0.5.0)|MIT/Apache-2.0|
|[crunchy 0.2.4](https://crates.io/crates/crunchy/0.2.4)|MIT|
|[either 1.19.0](https://crates.io/crates/either/1.19.0)|MIT OR Apache-2.0|
|[half 2.7.1](https://crates.io/crates/half/2.7.1)|MIT OR Apache-2.0|
|[hermit-abi 0.5.3](https://crates.io/crates/hermit-abi/0.5.3)|MIT OR Apache-2.0|
|[is-terminal 0.4.17](https://crates.io/crates/is-terminal/0.4.17)|MIT|
|[itertools 0.10.5](https://crates.io/crates/itertools/0.10.5)|MIT/Apache-2.0|
|[itoa 1.0.19](https://crates.io/crates/itoa/1.0.19)|MIT OR Apache-2.0|
|[libc 0.2.191](https://crates.io/crates/libc/0.2.191)|MIT OR Apache-2.0|
|[memchr 2.8.3](https://crates.io/crates/memchr/2.8.3)|Unlicense OR MIT|
|[num-traits 0.2.19](https://crates.io/crates/num-traits/0.2.19)|MIT OR Apache-2.0|
|[once_cell 1.21.4](https://crates.io/crates/once_cell/1.21.4)|MIT OR Apache-2.0|
|[oorandom 11.1.5](https://crates.io/crates/oorandom/11.1.5)|MIT|
|[proc-macro2 1.0.107](https://crates.io/crates/proc-macro2/1.0.107)|MIT OR Apache-2.0|
|[quote 1.0.47](https://crates.io/crates/quote/1.0.47)|MIT OR Apache-2.0|
|[regex 1.13.2](https://crates.io/crates/regex/1.13.2)|MIT OR Apache-2.0|
|[regex-automata 0.4.19](https://crates.io/crates/regex-automata/0.4.19)|MIT OR Apache-2.0|
|[regex-syntax 0.8.12](https://crates.io/crates/regex-syntax/0.8.12)|MIT OR Apache-2.0|
|[same-file 1.0.6](https://crates.io/crates/same-file/1.0.6)|Unlicense/MIT|
|[serde 1.0.229](https://crates.io/crates/serde/1.0.229)|MIT OR Apache-2.0|
|[serde_core 1.0.229](https://crates.io/crates/serde_core/1.0.229)|MIT OR Apache-2.0|
|[serde_derive 1.0.229](https://crates.io/crates/serde_derive/1.0.229)|MIT OR Apache-2.0|
|[serde_json 1.0.154](https://crates.io/crates/serde_json/1.0.154)|MIT OR Apache-2.0|
|[syn 2.0.119](https://crates.io/crates/syn/2.0.119)|MIT OR Apache-2.0|
|[syn 3.0.10](https://crates.io/crates/syn/3.0.10)|MIT OR Apache-2.0|
|[tinytemplate 1.2.1](https://crates.io/crates/tinytemplate/1.2.1)|Apache-2.0 OR MIT|
|[unicode-ident 1.0.27](https://crates.io/crates/unicode-ident/1.0.27)|(MIT OR Apache-2.0) AND Unicode-3.0|
|[walkdir 2.5.1](https://crates.io/crates/walkdir/2.5.1)|Unlicense/MIT|
|[winapi-util 0.1.11](https://crates.io/crates/winapi-util/0.1.11)|Unlicense OR MIT|
|[windows-link 0.2.1](https://crates.io/crates/windows-link/0.2.1)|MIT OR Apache-2.0|
|[windows-sys 0.61.2](https://crates.io/crates/windows-sys/0.61.2)|MIT OR Apache-2.0|
|[zerocopy 0.8.66](https://crates.io/crates/zerocopy/0.8.66)|BSD-2-Clause OR Apache-2.0 OR MIT|
|[zerocopy-derive 0.8.66](https://crates.io/crates/zerocopy-derive/0.8.66)|BSD-2-Clause OR Apache-2.0 OR MIT|
|[zmij 1.0.23](https://crates.io/crates/zmij/1.0.23)|MIT|

Python 校验器及所有间接依赖的版本、源站与再分发条件见 [格式工具依赖初审](docs/formats/validator-dependencies.md)。

本地 build host 实测 CMake 4.4.5、Ninja 1.13.2、clang-format 23.1.5 和 Apple Clang 21.0.0。平台编译器、转译及容器环境的实测组合以 QA 报告和构建证据为准。上述开发工具只用来构建或检查本地样例。

P3 的 ZIP 读写、临时目录和 CRC 校验使用 Python 标准库。新增 Rust 工作区模块复用锁文件中的 serde 和 serde_json。角色装载、动画时序和文件读取位于引擎独立模块，战斗核心的运行时依赖仍只有 Rust 标准库。

P4 的 GDExtension 静态链接角色、动画和资源模块，快照编码复用锁定版 serde_json；本轮没有新增第三方 crate。运行时归档按 ofe-bridge 的 Cargo 正常依赖树收集许可，Godot 4.7.2 与 godot-cpp 的原始声明位于 integrations/godot/licenses。角色图集来源和授权见 [P4 素材记录](docs/p4/RUNTIME.md)。
