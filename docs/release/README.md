# P0 桌面导出验证

P0 导出物用于验证技术样例。构建与运行应分别记录宿主操作系统、CPU 架构、Rust target、Godot 版本和扩展文件。交叉编译、转译运行和原生运行分别标明环境。

1. 按[开发指南](../development/GETTING_STARTED.md)为目标 CPU 和系统构建扩展，安装至该目标的项目副本。
2. 安装 Godot 4.7.2 stable 对应的导出模板，并对该副本运行 headless 自检。
3. 选择下表中的预设，导出至项目副本以外的独立目录。
4. 在对应目标环境执行导出包自检。记录二进制架构、SHA-256、命令、退出码与 OFE_P0_PASS。其余生成文件保留在产物旁。

|目标|预设名|建议输出|
|---|---|---|
|macOS ARM64|macOS ARM64|OFE.app|
|macOS x64|macOS x64|OFE.app|
|Linux x64|Linux x64|OFE.x86_64|
|Windows x64|Windows x64|OFE.exe|

官方 macOS 模板包含 universal 二进制。prepare_macos.py 从已安装模板提取指定架构并准备预设引用的 zip；Intel 目标使用 --arch x86_64。

例如，在 macOS ARM64 上从仓库根目录运行。变量 GODOT 指向本机 4.7.2 编辑器。

```sh
python3 scripts/release/prepare_macos.py --arch arm64
mkdir -p build/export-arm
"$GODOT" --headless --path integrations/godot/runtime --export-release "macOS ARM64" "$PWD/build/export-arm/OFE.app"
```

导出后，读取应用包 Info.plist 中的 CFBundleExecutable，以该名称定位程序，并测试导出包本身。

```sh
executable=$(/usr/libexec/PlistBuddy -c 'Print CFBundleExecutable' build/export-arm/OFE.app/Contents/Info.plist)
python3 scripts/test/headless.py --exported "build/export-arm/OFE.app/Contents/MacOS/$executable"
```

Linux 和 Windows 对预设生成的可执行文件直接运行相同的 --exported 检查。正式分发时复核源码获取路径、Godot 和 godot-cpp 声明、Godot 的第三方组件清单及素材授权，详见[许可初审](../../LICENSE_AUDIT.md)。P0 实际执行到的验证层级以[独立验收报告](../../P0_ACCEPTANCE_REPORT.md)为准。
