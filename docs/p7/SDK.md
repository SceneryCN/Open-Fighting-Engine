# 用 Rust 制作第一个角色

工具包包含两个 MIT Rust 库和独立 Cargo 工程。使用仓库的 Rust 1.100 工具链，在解压后的根目录运行以下命令。输出路径的父目录须已存在。生成器只创建新目录。

```sh
cargo run --locked --manifest-path samples/sdk-author/Cargo.toml -- my-rook
python tools/ofe-validator/ofe.py validate my-rook
```

首次编译需要取得 Cargo.lock 中列出的依赖。已缓存依赖时可以添加 `--offline`。已安装的 Python 依赖、打包及项目启动步骤见[制作指南](AUTHORING.md)。源文件 [main.rs](../../samples/sdk-author/src/main.rs) 展示两个 SDK 的完整调用。生成结果可与 [Rook](../../samples/characters/rook) 比较。

## 动画

`ofe_animation_sdk::Animation` 描述剪辑的 ID、循环方式和帧来源。使用 `Frames::Sequence` 传入每帧的 PNG 相对路径。使用 `Frames::Sheet` 传入 PNG、列数、行数及从 0 开始的单元索引，按行排列。`Frame::new(image_or_index, duration, x, y)` 的持续时间以 60 Hz 模拟帧计，锚点以单帧左上角为原点，单位为像素。将事件 ID 加到该帧的 `events` 中，调用 `document()?` 生成标准 OFA JSON。示例 idle 使用两帧序列改变站姿锚点，attack 在图集第二格发出 contact 事件。

## 角色

`ofe_character_sdk::Character` 包含身份、许可、生命与能量上限、普通招式、碰撞框、OFA 文档及素材字节。向 `assets` 提供相对名称，如 `idle.png`。生成文件位于 `animations/assets/idle.png`，同级动画文档使用 `assets/idle.png` 引用。生成器持有原始 PNG，可以在空目录独立复现样例。调用 `write_new` 后可以继续用文本编辑器修改各 JSON。

`NormalMove` 生成同线、零能量消耗的地面普通技，`BodyBox` 的 x、h、宽、高使用战斗世界单位。Rook 有 96 点生命，straight 的启动、有效、收招时长分别为 2、3、5 帧，普通命中造成 12 点伤害。编辑生成后的 OFC 招式和配置可加入额外招式、取消窗及 P2 动作配置。完整字段以 [OFC](../../formats/ofc/schema.json) 和 [OFA](../../formats/ofa/schema.json) 为准。校验与打包命令检查完整资源图、格式版本和引用。
