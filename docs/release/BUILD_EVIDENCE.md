# P0-07 实际构建和导出记录

2026-11-26（Asia/Shanghai）。集成源包含提交 1eec9a9 的 Rust 桥接及 5b89414 的并发测试。构建与导出脚本见提交 27c71d7。每个目标的 release 桥接库都链接了 Rust staticlib 和 godot-cpp。对外交付包各自包含目标架构的库。

| 目标 | 编译环境 | 独立导出目录 | 本轮执行记录 |
| --- | --- | --- | --- |
| macOS ARM64 | Apple M1 Max、macOS 27.3、Apple Clang 21、CMake 4.4.5、Rust aarch64-apple-darwin | macos-arm64/OFE.app | Mach-O arm64，CTest 2/2；导出后本机运行打印 OFE_P0_PASS，退出 0 |
| macOS x64 | ARM64 主机、Clang 指定 x86_64、Rust x86_64-apple-darwin | macos-x64/OFE.app | Mach-O x86_64，通过 Rosetta 执行 CTest 2/2，导出后打印 OFE_P0_PASS，退出 0 |
| Linux x64 | OrbStack 的 linux/amd64 Debian 12 容器，GCC 12.2、CMake 3.25.1、Rust x86_64-unknown-linux-gnu | linux-x64/OFE | ELF x86-64；容器内 CTest 2/2、Godot 4.7.2 编辑器导入和自检成功，导出后打印 OFE_P0_PASS，退出 0 |
| Windows x64 | macOS 上 MinGW-w64 GCC 16.2、Rust x86_64-pc-windows-gnu | windows-x64/OFE.exe，另有 OFE.console.exe | DLL、ABI 测试程序均链接成功；Godot 导出可执行文件、控制台入口、PCK 及 DLL；Windows 运行验证由独立 QA 记录 |

Rust 工具链均为 1.100.0 stable，Godot 4.7.2.stable.official.ed1daf0bf，godot-cpp 10.0.0-stable（507ed9d），选择 API 4.7。macOS 使用已签名的官方通用编辑器，导出应用仅作本地 ad-hoc 签名。Windows 和 Linux 模板由该编辑器跨平台导出。导出模板归档 SHA-256 为 f298490b8d44d934be425a5a65a51bf15f422428b229a06a6e11d9ffea248011。全部导出文件的路径和校验和见 [P0-SHA256SUMS.txt](P0-SHA256SUMS.txt)，在产物根目录执行 `shasum -a 256 -c SHA256SUMS`。

## 构建复核

以下命令从仓库根目录执行。交叉构建时先用 rustup 安装指定目标的标准库。Linux 在 Debian 的 amd64 用户空间内执行常规本机构建。CMake 默认通过 Git 下载固定版本的 godot-cpp。此次本地各架构共享该固定提交的干净 checkout，配置时使用了 `-DFETCHCONTENT_SOURCE_DIR_GODOT-CPP=<checkout>`。

```sh
cmake -S . -B build/arm -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES=arm64 -DOFE_RUST_TARGET=aarch64-apple-darwin
cmake -S . -B build/intel -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES=x86_64 -DOFE_RUST_TARGET=x86_64-apple-darwin
cmake -S . -B build/win -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE=scripts/build/mingw-x64.cmake
cmake -S . -B build/linux -G Ninja -DCMAKE_BUILD_TYPE=Release
```

各 build 目录执行 `cmake --build <dir> --config Release`，用 `cmake --install <dir> --config Release --prefix <project>` 安装到对应架构的 Godot 项目副本。可执行当前目标程序的环境执行 `ctest --test-dir <dir> -C Release --output-on-failure`。C 与 C++ 两个黑盒 ABI 测试分别覆盖 10 万次生命周期以及并发冲突。PoC 冒烟由 GDScript 调用公开接口，检查返回值并输出 OFE_P0_PASS。

## 已定位的构建问题

- 首次在空缓存执行编辑器导入，Godot 4.7.2 在发现扩展后的关闭阶段崩溃（SIGSEGV）。4.5 和 4.7 两种绑定均复现。CMake 安装现在在首次启动前将扩展添加到 Godot 自身的注册文件，保留其他条目；经此路径首次导入成功。
- 官方 macOS 模板仅包含 universal 引擎。按架构的导出预设直接请求 thin 二进制时失败。准备脚本从同一官方模板用 lipo 提取目标架构的 release 引擎；导出包保留原生架构库，应用包可实际启动。
- Debian 12 的 Wine 8 无法提供 Rust Windows 目标需要的 bcryptprimitives.dll。Windows 原生运行和实际托管 CI 结果须单独记录。四个本地构建产物及导出文件已留存。
