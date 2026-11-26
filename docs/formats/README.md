# OFE 开发目录格式 1.0

四个根文档使用 UTF-8 JSON、大小写敏感的 format 和精确字符串版本 1.0。对应 Schema 存放于 formats 下的同名小写目录。每个 Schema 使用 Draft 2020-12；x-ofe-resource 是校验器处理的本地资源类型标注。标准校验器可独立检查结构。OFE 工具额外执行引用、路径和跨字段检查。[Draft 2020-12](https://json-schema.org/draft/2020-12)

| 根格式 | 数据与单位 |
|---|---|
| OFC | manifest.json 索引角色元数据、最大生命/能量、招式、OFA、判定框、音效、资源、许可证与依赖。招式的启动、有效、收招、硬直、停止、无敌时间均以 tick 计。取消窗口起止均包含端点；armor 为吸收次数，能量与伤害为非负整数；pushback 为有符号 X 位移。lane_rule 使用四种既定枚举，只有 DEPTH_RANGE 需非负 depth_range。框的局部坐标以角色原点计，宽高均为正。 |
| OFA | source.kind 为 sequence 时，每帧 image 引用一张 PNG；为 sheet 时，source 包含 PNG、网格行列数，每帧按行优先、从零起的 index 索引网格。duration 为正整数 tick。origin 是相对该帧左上角的像素锚点，events 为表现事件标识符。 |
| OGP | characters 指向 OFC manifest，layout 指向 OFL，stages、audio、fonts、theme 指向本地资源。modes 可声明 training、1v1、2v2、ffa。dependencies 声明 id 和版本字符串，P0 无包解析器。 |
| OFL | 六个屏幕均须存在，每个屏幕列出元素；屏幕内元素 id 唯一。元素 type 为 label、image、button 或 bar，x 和 y 是像素布局坐标。可设置 text 和本地 asset。 |

单个资源路径从引用它的文档所在目录向下解析，使用 / 分隔。绝对路径、盘符、点路径段、空路径段、反斜线及越界符号链接均被拒绝。OFC 的相邻子目录通过 manifest 索引；动画图像可置于动画目录的 assets 子目录。校验递归检查有类型的引用；音频、字体、舞台与主题文件在 P0 检查文件存在性。PNG 附加签名检查。

1.0 校验器要求所有必填字段存在且拒绝额外字段。格式演进须发布新 Schema 和迁移说明，跨版本读取前应显式迁移。发布用 ZIP 打包及用运行时解释招式、动画、布局留待对应阶段。

## 复现

在仓库根使用 Python 3.12 创建虚拟环境。示例为 POSIX shell；Windows 激活文件为 `.venv\Scripts\Activate.ps1`。

```sh
python3.12 -m venv .venv
. .venv/bin/activate
python -m pip install --require-hashes -r tools/ofe-validator/requirements.txt
python tools/ofe-validator/ofe.py validate tests/fixtures/formats/valid/project.json
python tools/ofe-validator/ofe.py validate tests/fixtures/formats/invalid/ogp.json
python -m unittest discover -s tests/formats -v
```

合法项目将校验完整角色、动画与布局引用链。非法 OGP 在 `#/modes/0` 定位未登记的模式值 online。退出码 0 表示校验通过，1 表示输入、结构、文件或引用错误，2 表示命令参数错误。合法精灵表动画另见 tests/fixtures/formats/valid/character/animations/sheet.json。本版命令接收入口文档路径。

## P2 招式补充配置

OFC 1.0 描述基础帧数、伤害、硬直、击停、护甲、无敌、取消窗口与能量。P2 场景以相对路径配对 OFC 和独立的 OFE_MOVE_PROFILE 1.0 文件，补充招式种类、多段窗口、取消目标与投射物参数。校验器检查文件结构、ID 唯一性和有序窗口；模拟器在构造 Arena 前对照角色 manifest 检查 ID、取消目标及窗口是否位于相应招式时序内。[Schema](../../formats/ofe_move_profile/schema.json) · [可运行样例](../../tools/ofe-sim/fixtures/arena/move_profile.json)

## P3 角色目录与发布包

作者工具的 validate 支持 OFC 目录和 .ofc ZIP，inspect 输出作者、许可、动作、动画和依赖声明。package 只收集资源引用图和可选的 move_profile.json。run 解开并校验归档，再启动本地模拟器。动画资源检查 PNG 头部尺寸、图集网格和索引。完整命令见[角色使用指南](../p3/CHARACTERS.md)。

## P6 权威录像

OFE_REPLAY v1 保存起始规则及快照、每个提交帧的命令与预期哈希。Schema 描述磁盘形状和基本范围；核心恢复时核对所有槽位、数组对应关系、招式索引与有效窗口。运行 `ofe-sim replay FILE` 会逐帧复算并定位首次差异。[Schema](../../formats/replay/schema.json) · [快照和哈希契约](../architecture/ADR-009.md)
