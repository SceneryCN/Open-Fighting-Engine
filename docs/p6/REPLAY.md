# 训练录像与逐帧验证

在训练场按 F8 开始录制，再按 F8 停止，按 F9 就地回看。F10 将本次录制写入 Godot 用户数据目录的 `training.ofr`。下次启动，选择相同角色和席位顺序进入训练，按 F11 读盘校验并播放。工具栏也能用鼠标操作。文件操作期间会暂停模拟，窗口继续绘制。校验失败时会显示错误，当前对局和已有录制保留。

|桌面系统|默认录像位置|
|---|---|
|macOS|`~/Library/Application Support/Godot/app_userdata/Open Fighting Engine/training.ofr`|
|Linux|`${XDG_DATA_HOME:-$HOME/.local/share}/godot/app_userdata/Open Fighting Engine/training.ofr`|
|Windows|`%APPDATA%\Godot\app_userdata\Open Fighting Engine\training.ofr`|

F10 成功时会替换同名文件。要保留多次练习，可复制该文件并重命名。训练界面支持最多 36,000 帧、从第 0 帧开始的双人录像；读盘依据解析后的角色战斗规则检查匹配性。图像和配乐由当前游戏提供。

## 在命令行复现

从仓库根目录运行以下命令，或使用对应桌面平台的 ofe-sim 发行包。第一条用种子 42 和仓库的四人输入脚本生成录像，第二条仅使用生成的文件完成独立验证。

```sh
cargo run --locked -p ofe-sim -- record --mode ffa --seed 42 --script tools/ofe-sim/fixtures/arena.frames --out match.ofr
cargo run --locked -p ofe-sim -- replay match.ofr
```

将第二条中的文件名换成游戏保存的 `training.ofr` 即可检查实际录下的操作。作者工具支持 `python tools/ofe-validator/ofe.py replay match.ofr --simulator ./target/release/ofe-sim`。退出码 0 表示验证成功，输出包含已验证帧数、末帧和哈希。预期状态与实算状态不一致时，返回非零并输出首个分歧帧。无效规则、输入和不连续帧会报告相应错误。

录像包含完整的初始战斗规则和状态、每帧已接受的输入及哈希，独立命令行验证无需原始角色目录。哈希按十六进制字符串保存，帧数和 PRNG 状态是精确整数。工具应保留 JSON 中的 64 位整数精度。共享读写层限制为 64 MiB、216,000 帧。该版本的哈希用于逐帧一致性比较。[录像 Schema](../../formats/replay/schema.json) · [核心接口、种子与哈希编码](../architecture/ADR-009.md)
