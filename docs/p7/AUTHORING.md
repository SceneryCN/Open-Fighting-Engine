# 制作角色并在 Amber Dojo 中试玩

下载对应平台的游戏包 `ofe-runtime` 和通用作者包 `ofe-author-kit`，分别解开。在解包后的根目录运行下面的命令。需要 Python 3.12；从 Rust 源码生成角色另需 Rust 1.100。作者包的 `samples/characters/rook` 已含可直接编辑和运行的角色。

## 打包与试玩

```sh
python -m venv .venv
. .venv/bin/activate
python -m pip install --require-hashes -r tools/ofe-validator/requirements.txt
python tools/ofe-validator/ofe.py validate samples/projects/starter
python tools/ofe-validator/ofe.py inspect samples/characters/rook
python tools/ofe-validator/ofe.py package samples/projects/starter dojo.ogp
python tools/ofe-validator/ofe.py package samples/characters/rook rook.ofc
python tools/ofe-validator/ofe.py launch dojo.ogp --runtime /path/to/OFE --mod rook.ofc
```

把最后一行的路径替换为解包的游戏程序。macOS 可使用 `OFE.app/Contents/MacOS/Open Fighting Engine`，Windows 使用 `OFE.exe`，Windows PowerShell 的环境激活命令为 `.venv\Scripts\Activate.ps1`。路径含空格时加双引号。启动后在 **Amber Dojo → Local duel** 选择 Rook，点 Continue 和 Fight。主菜单还提供 Training。

## 修改角色

从 Rust 示例生成到新目录，或复制现成角色后编辑。可将修改后的目录直接传给 `--mod`。为每个角色设置唯一的 manifest.id；游戏显示 character.json 中的 name。

```sh
cargo run --locked --manifest-path samples/sdk-author/Cargo.toml -- my-rook
python tools/ofe-validator/ofe.py validate my-rook
python tools/ofe-validator/ofe.py launch samples/projects/starter --runtime /path/to/OFE --mod my-rook
```

在 attributes.json 调整最大生命和能量，在 moves 下修改招式的启动、有效、收招帧和伤害。对战按 60 tick/s 计算。OFA 的 frames[].duration 也以 tick 计。每帧可指定 origin 和 events；序列帧通过 image 引用 PNG，图集帧通过 index 索引 source 中定义的网格。素材路径相对于引用文件所在目录。

## 修改项目与分发

复制 samples/projects/starter，编辑 project.json 的 title、modes、characters 和 settings.depth_projection。characters 按顺序列出项目目录内的 OFC manifest；启动时可多次使用 `--mod` 依次追加。OGP 里的舞台、布局、音频、字体与主题会校验和打包；当前游戏用内置舞台、界面与音频呈现。启动支持本地、无待解析依赖的项目及角色。

用 package 分别产生项目归档和附加角色归档。接收方用同一条 launch 命令运行，CLI 在游戏关闭时释放临时文件。作者工具检查格式版本、资源引用和归档；可用 inspect 查看包内容摘要。两套 Rust SDK 和示例 Rust 代码采用 MIT，示例角色数据与原始美术采用 CC0。许可全文见作者包 LICENSE 与 LICENSES。
