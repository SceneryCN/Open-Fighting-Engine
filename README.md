# Open Fighting Engine

OFE 是面向离线 2D 双线格斗的开源引擎。当前 Rust 核心提供双人、2v2 和四人自由混战。终端模拟器显示逐帧动作、位置、生命、能量、连招、投射物及战斗事件。

```sh
cargo run --locked -p ofe-sim -- --mode 2v2 --script tools/ofe-sim/fixtures/arena.frames
cargo run --locked -p ofe-sim -- --mode ffa --script tools/ofe-sim/fixtures/arena.frames
```

四人交互对战省略脚本参数，每行给四个角色各输入一个动作。原有双人交互对战使用 `cargo run --locked -p ofe-sim`。每行输入双方各自的动作，例如 `A G` 表示玩家一发起普攻、玩家二保持防御。连续输入 `- G` 可观察攻击进入有效帧后的格挡结果。[操作与样例数据](tools/ofe-sim/README.md) · [多人核心接口](docs/architecture/ADR-005.md) · [双人兼容接口](docs/architecture/ADR-004.md)

## 从哪里开始

- 编译、启动样例和运行本地检查，见[开发环境与首次运行](docs/development/GETTING_STARTED.md)。
- 固定帧、坐标方向、快照、模块依赖和 ABI 设计，见[架构说明](docs/architecture/overview.md)。
- 校验角色、动画、游戏项目与布局文件，见[原生格式指南](docs/formats/README.md)。
- 导出四种桌面架构的验证包，见[导出验证](docs/release/README.md)。

## 工程边界

|路径|职责|
|---|---|
|engine/combat-core|60 Hz 帧推进、X/Z/H、多人动作、队伍、投射物与统一战斗结算；兼容 P0 单实体快照接口|
|integrations/godot/bridge|Rust C ABI v1、句柄注册表与调用方缓冲区|
|integrations/godot/extension|C++ 类型适配、对象注册与生命周期|
|integrations/godot/runtime|技术验证场景、输入采集和 2D 表现|
|formats、tools/ofe-validator|四套 Draft 2020-12 Schema 与本地资源引用校验|
|tools/ofe-sim|读取内置 OFC 与招式补充配置，支持双人和四人逐帧模拟|
|tests、scripts|格式样例、跨语言回归与可重复构建验证|

[项目总任务书 V1.0](docs/governance/PROJECT_CHARTER_V1.0.md) 是项目唯一总纲。P0、P1 已签收。[P2 实施方案](docs/p2/PLAN.md)、[项目状态](PROJECT_STATUS.md)和[交付报告](DELIVERY_REPORT.md)记录本阶段的范围与验证结果。

源代码和数据按模块许可。分发前查看[许可证分配](LICENSE)、[依赖清单](DEPENDENCIES.md)及[许可初审](LICENSE_AUDIT.md)。
