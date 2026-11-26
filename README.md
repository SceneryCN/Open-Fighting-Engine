# Open Fighting Engine

OFE 是面向离线 2D 双线格斗的开源引擎。当前 Rust 核心提供双人、2v2 和四人自由混战。终端模拟器显示逐帧动作、位置、生命、能量、连招、投射物及战斗事件。

```sh
cargo run --locked -p ofe-sim -- --mode 2v2 --script tools/ofe-sim/fixtures/arena.frames
cargo run --locked -p ofe-sim -- --mode ffa --script tools/ofe-sim/fixtures/arena.frames
```

四人交互对战省略脚本参数，每行给四个角色各输入一个动作。原有双人交互对战使用 `cargo run --locked -p ofe-sim`。每行输入双方各自的动作，例如 `A G` 表示玩家一发起普攻、玩家二保持防御。连续输入 `- G` 可观察攻击进入有效帧后的格挡结果。[操作与样例数据](tools/ofe-sim/README.md) · [多人核心接口](docs/architecture/ADR-005.md) · [双人兼容接口](docs/architecture/ADR-004.md)

## 桌面候选发行

0.1.0-rc.1 提供 Windows x64、Linux x64、Apple 芯片和 Intel Mac 游戏包。下载时按 CPU 选择，用随本次构建提供的 SHA-256 核对归档。[安装、启动与默认键位](docs/p8/INSTALL.md) · [候选交付记录](DELIVERY_REPORT.md)

## 启动 Sunset Viaduct

下载对应平台的 `ofe-runtime` 归档，解包后启动 OFE。从源码构建原生扩展后，也可以用 Godot 4.7.2 打开项目并按 F5。主菜单提供本地 1V1、2V2、四人混战、训练和设置。Esc 暂停，Controls 查看按键。[操作与训练指南](docs/p5/PLAY.md)

## 制作角色与运行自己的项目

从四平台构建页面下载 `ofe-author-kit` 和本机的 `ofe-runtime`。作者包提供角色与动画 Rust SDK、可编辑的 Rook 角色和 Amber Dojo 示例项目。通过命令行打包后，追加角色会出现在游戏的选人菜单。[作者入门与启动命令](docs/p7/AUTHORING.md)

## 录制与验证回放

训练场用 F8 开始或停止录制，F10 保存，F11 从磁盘读取。游戏和命令行共用同一录像格式。模拟器可独立验证游戏录下的每一帧，在首次分歧时报告帧号与两个状态哈希。[录像操作与复现命令](docs/p6/REPLAY.md)

## 使用外部角色

```sh
cargo build --locked --release -p ofe-sim
target/release/ofe-sim --character tools/ofe-sim/fixtures/amber --script tools/ofe-sim/fixtures/duel.frames
```

安装格式工具的 Python 依赖后，可将同一个目录打包成可分发的 `.ofc`，校验资源并从归档启动对战。[角色制作与运行](docs/p3/CHARACTERS.md)

## 从哪里开始

- 编译、启动样例和运行本地检查，见[开发环境与首次运行](docs/development/GETTING_STARTED.md)。
- 固定帧、坐标方向、快照、模块依赖和 ABI 设计，见[架构说明](docs/architecture/overview.md)。
- 校验角色、动画、游戏项目与布局文件，见[原生格式指南](docs/formats/README.md)。
- 导出四种桌面架构的验证包，见[导出验证](docs/release/README.md)。

## 工程边界

|路径|职责|
|---|---|
|engine/combat-core|60 Hz 帧推进、X/Z/H、多人动作、队伍、投射物与统一战斗结算；兼容 P0 单实体快照接口|
|engine/character-runtime、engine/animation-runtime、engine/resource-runtime|外部角色加载、逐帧动画播放、资源缓存与释放|
|integrations/godot/bridge|Rust C ABI v1、双人和多人对战 ABI v2、句柄与调用方缓冲区|
|integrations/godot/extension|C++ 类型适配、对象注册与生命周期|
|integrations/godot/runtime|示范游戏、四人对战、训练、双线舞台和音频；保留 P0 桥接验证场景|
|formats、tools/ofe-validator|四套 Draft 2020-12 Schema 与本地资源引用校验|
|tools/ofe-sim|读取外部角色目录、内置 OFC 与招式补充配置，支持双人和四人逐帧模拟|
|tests、scripts|格式样例、跨语言回归与可重复构建验证|

[项目总任务书 V1.0](docs/governance/PROJECT_CHARTER_V1.0.md) 是项目唯一总纲。P0 至 P2 的复核结论见[稳定性报告](docs/testing/P0_P2_REVIEW.md)。[P8 发行与键位决策](docs/architecture/ADR-011.md)、[项目状态](PROJECT_STATUS.md)和[交付报告](DELIVERY_REPORT.md)记录本阶段的范围与验证结果。

源代码和数据按模块许可。分发前查看[许可证分配](LICENSE)、[依赖清单](DEPENDENCIES.md)及[许可初审](LICENSE_AUDIT.md)。
