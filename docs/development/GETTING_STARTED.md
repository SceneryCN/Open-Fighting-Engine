# 开发环境与首次运行

从仓库根目录执行以下命令。先运行 git status --short --branch 查看所在分支和工作区，再选择本机目标。

|工具|准备方式|
|---|---|
|Git、Rust|安装 Git 与 rustup。仓库的 rust-toolchain.toml 指定 1.100.0 stable、rustfmt 和 Clippy。|
|C++、CMake、Ninja|macOS 安装 Xcode Command Line Tools；Linux 使用 C++17 编译器和开发头文件；Windows 在 x64 Visual Studio 开发者终端运行。各环境使用 CMake 和 Ninja。|
|Python|使用 3.12，供 godot-cpp 生成绑定及 Schema 测试使用。|
|Godot|使用 Godot 4.7.2 stable 桌面编辑器。GUI 与 headless 使用同一可执行文件。导出需对应版本的模板。|

首次配置会从上游拉取固定的 godot-cpp 10.0.0-stable，并选择 Godot 4.7 API。Cargo 通过 Cargo.lock 固定 crate 版本。支持离线重建的前提是已缓存相同版本的依赖、目标标准库和编译工具。

## 编译并安装本机扩展

以下命令在 POSIX shell 和 Windows x64 开发者终端中均可从仓库根目录运行。

```sh
rustc --version
cargo --version
cmake -S . -B build/native -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build/native --config Release
cmake --install build/native --config Release --prefix integrations/godot/runtime
```

安装还会准备 Godot 的扩展列表，规避 4.7.2 在首次发现扩展时可能出现的编辑器退出崩溃。清除项目的 .godot 目录后重新执行 install，再运行导入检查。安装后检查 integrations/godot/runtime/bin 下与宿主匹配的 libofe_godot.dylib、libofe_godot.so 或 ofe_godot.dll。macOS 的两个架构使用同一安装文件名，为它们分别保留构建目录和用于运行或导出的项目副本。

## 首次导入、冒烟与手动验证

macOS 使用应用包内的可执行文件。Linux 将 GODOT 设为实际编辑器路径。Windows 可在 PowerShell 中将下列命令里的 "$GODOT" 替换成编辑器完整路径，最后一条通过调用运算符 & 启动。

```sh
GODOT=/Applications/Godot.app/Contents/MacOS/Godot
python3 scripts/test/headless.py "$GODOT"
"$GODOT" --path integrations/godot/runtime
```

源码自检会依次验证首次导入、P0 重入、外部角色目录、P0 与 P4 默认场景，最后输出 OFE_P4_PASS。缺少扩展类时检查安装目录、CPU 架构和导入日志。普通启动会进入双人键盘对战，按 F 让 P1 普攻，按 R 重开。单独启动旧桥接样例可运行 `"$GODOT" --path integrations/godot/runtime res://proof.tscn`。[完整按键与角色路径](../p4/RUNTIME.md)

## 测试与基准

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo nextest run --locked --workspace
cargo bench --locked -p ofe-combat-core --bench frame
```

nextest 可通过 cargo install cargo-nextest --version 0.9.152 --locked 安装。Criterion 基准输出本机采样结果。C 调用方、重复生命周期与泄漏检查命令见[桥接验证](../testing/BRIDGE.md)。Schema 工具的隔离环境、哈希锁定安装、合法及非法输入的复现命令见[格式指南](../formats/README.md)。
