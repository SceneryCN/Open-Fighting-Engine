# OFE 0.1.0-rc.1 安装与运行

在交付报告对应的 GitHub Actions 运行页底部找到 Artifacts。登录 GitHub 后，下载适合本机的游戏包和 `ofe-candidate-checksums`，先解开 GitHub 的外层 ZIP。游戏本体无需安装 Godot、Rust 或 Python。

|设备|游戏归档|解开后的入口|
|---|---|---|
|Windows 64 位 Intel / AMD|ofe-runtime-windows-x64.tar.gz|OFE.exe|
|Linux 64 位 Intel / AMD|ofe-runtime-linux-x64.tar.gz|./OFE|
|Apple 芯片 Mac|ofe-runtime-macos-arm64.tar.gz|OFE.app|
|Intel Mac|ofe-runtime-macos-x64.tar.gz|OFE.app|

先对 .tar.gz 计算 SHA-256，与 SHA256SUMS 中同名文件对应的值逐位比较。Windows PowerShell 可用 `Get-FileHash 文件名 -Algorithm SHA256`，macOS 可用 `shasum -a 256 文件名`，Linux 可用 `sha256sum 文件名`。一致后，用 `tar -xzf 文件名` 解开游戏，保留同目录下的库、资源和许可文件。打开上表入口可进入 Sunset Viaduct，选择 Local duel，在选人后依次点击 Continue、Fight。

一号位用 A/D 左右移动、W 跳跃、S 换线。J 或 O 普攻、K 防御、L 冲刺、U 特殊技、I 超必杀。二号位使用方向键和 F/G/H/R/T。Esc 可以暂停，Controls 可查看全部席位与手柄操作。

macOS 候选使用临时签名，Windows 候选未使用发行商签名。若系统或组织策略限制启动，可提供系统版本与拦截提示给项目负责人。检查平台及架构时，Mac 可打开“关于本机”，Windows 可打开“设置 → 系统 → 系统信息”，Linux 可运行 `uname -m`。

命令行模拟器对应 `ofe-sim-平台.tar.gz`，解包后从该目录执行 `./ofe-sim --script duel.frames`（Windows 为 `.\ofe-sim.exe --script duel.frames`）。制作新角色可另取 `ofe-author-kit.tar.gz`，从其中的 README.md 开始。验收后再由负责人决定正式公开。
