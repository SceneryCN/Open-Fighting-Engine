# Open Fighting Engine

OFE 是一套面向离线 2D 双线格斗的开源引擎。当前 P0 工程提供独立 Rust 模拟核心、带版本的 C ABI、C++ GDExtension 和一个可运行的 Godot 技术样例。在样例中按左右方向键，Rust 每个固定帧接收一次输入并更新 X 坐标；Godot 将读回的位置画成圆点。

## 从哪里开始

- 编译、启动样例和运行本地检查，见[开发环境与首次运行](docs/development/GETTING_STARTED.md)。
- 固定帧、坐标方向、快照、模块依赖和 ABI 设计，见[架构说明](docs/architecture/overview.md)。
- 校验角色、动画、游戏项目与布局文件，见[原生格式指南](docs/formats/README.md)。
- 导出四种桌面架构的验证包，见[导出验证](docs/release/README.md)。

## 工程边界

|路径|职责|
|---|---|
|engine/combat-core|60 Hz 帧推进、整数位置、编号输入、移动事件与状态快照|
|integrations/godot/bridge|Rust C ABI v1、句柄注册表与调用方缓冲区|
|integrations/godot/extension|C++ 类型适配、对象注册与生命周期|
|integrations/godot/runtime|技术验证场景、输入采集和 2D 表现|
|formats、tools/ofe-validator|四套 Draft 2020-12 Schema 与本地资源引用校验|
|tests、scripts|格式样例、跨语言回归与可重复构建验证|

[项目总任务书 V1.0](docs/governance/PROJECT_CHARTER_V1.0.md) 是项目唯一总纲。P0 的工作顺序和签收责任见[实施方案](docs/development/P0_PLAN.md)。[项目状态](PROJECT_STATUS.md)、[交付报告](DELIVERY_REPORT.md)和[独立验收报告](P0_ACCEPTANCE_REPORT.md)分别记录进度、证据和签收条件。负责人确认 P0 通过后，才能启动 P1。

源代码和数据按模块许可。分发前查看[许可证分配](LICENSE)、[依赖清单](DEPENDENCIES.md)及[许可初审](LICENSE_AUDIT.md)。
