# Open Fighting Engine

OFE 是面向离线 2D 双线格斗的开源引擎。P1 提供独立的 Rust 双人格斗核心和逐帧终端模拟器。运行内置对局脚本可以看到双方的动作、坐标、生命和战斗事件。

```sh
cargo run --locked -p ofe-sim -- --script tools/ofe-sim/fixtures/duel.frames
```

交互对战使用 `cargo run --locked -p ofe-sim`。每行输入双方各自的动作，例如 `A G` 表示玩家一发起普攻、玩家二保持防御。连续输入 `- G` 可观察攻击进入有效帧后的格挡结果。[操作与样例数据](tools/ofe-sim/README.md) · [双人核心接口](docs/architecture/ADR-004.md)

## 从哪里开始

- 编译、启动样例和运行本地检查，见[开发环境与首次运行](docs/development/GETTING_STARTED.md)。
- 固定帧、坐标方向、快照、模块依赖和 ABI 设计，见[架构说明](docs/architecture/overview.md)。
- 校验角色、动画、游戏项目与布局文件，见[原生格式指南](docs/formats/README.md)。
- 导出四种桌面架构的验证包，见[导出验证](docs/release/README.md)。

## 工程边界

|路径|职责|
|---|---|
|engine/combat-core|60 Hz 帧推进、X/Z/H、双人状态机、判定框与战斗结算；兼容 P0 单实体快照接口|
|integrations/godot/bridge|Rust C ABI v1、句柄注册表与调用方缓冲区|
|integrations/godot/extension|C++ 类型适配、对象注册与生命周期|
|integrations/godot/runtime|技术验证场景、输入采集和 2D 表现|
|formats、tools/ofe-validator|四套 Draft 2020-12 Schema 与本地资源引用校验|
|tools/ofe-sim|读取两份内置 OFC 样例，将玩家输入提交给双人核心|
|tests、scripts|格式样例、跨语言回归与可重复构建验证|

[项目总任务书 V1.0](docs/governance/PROJECT_CHARTER_V1.0.md) 是项目唯一总纲。P0 已签收。[P1 实施方案](docs/p1/PLAN.md)、[项目状态](PROJECT_STATUS.md)和[交付报告](DELIVERY_REPORT.md)记录本阶段的范围与验证结果。

源代码和数据按模块许可。分发前查看[许可证分配](LICENSE)、[依赖清单](DEPENDENCIES.md)及[许可初审](LICENSE_AUDIT.md)。
