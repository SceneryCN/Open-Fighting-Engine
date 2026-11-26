# P0 工程交付报告 · 2026-11-26
**独立 QA 已完成。阶段状态 TESTING，待项目负责人签收。** 提交 1cb8609 推送到负责人指定的远端 main 后触发托管工作流。quality 与四个桌面 proof 作业均成功，四份导出产物已上传。P1 为 NOT_STARTED。[P0 #1](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45783184524)

## 交付范围
- P0-01/02：初始化前核实空目录与非 Git 状态；原始任务书、架构冻结决议、Rust 1.100.0 与 Cargo.lock、CMake、Godot 4.7.2 stable 及固定 godot-cpp 已入库。[审计](docs/development/INITIAL_AUDIT.md) · [架构](docs/architecture/overview.md)
- P0-03/05：独立 Rust crate 接收编号输入，以 60 Hz 固定步进更新整数坐标，提供快照、恢复、哈希和移动事件。Godot 的方向键经 C++ 调用 ABI v1；状态由 Rust 返回。接口支持错误码、结构大小校验、失效句柄和重复释放保护。[桥接验证](docs/testing/BRIDGE.md)
- P0-04：交付 OFC/OFA/OGP/OFL Schema、正反例和递归资源校验器。合法样例从 OGP 一路解析到角色、动画与实际 PNG。12 项格式测试通过。[格式指南](docs/formats/README.md)
- P0-06/07：Cargo test、nextest、Clippy 与 rustfmt 通过。C 宿主完成 100,000 次生命周期，独立 QA 另测并发；Godot 每次 smoke 包括 256 次创建、推进、读回及释放。四个目标均实际编译链接。构建类型、执行环境和剩余验收条件分列于下。[性能数据](docs/testing/PERFORMANCE.md)
- P0-08：提供开发、导出、测试、架构、许可、风险和状态文档。QA 的签注意见见 P0_ACCEPTANCE_REPORT.md。[环境指南](docs/development/GETTING_STARTED.md) · [风险](docs/testing/P0_RISKS.md) · [许可初审](LICENSE_AUDIT.md)

## 实际验证
|目标|扩展构建|运行环境|Godot 宿主 / 导出包自检|
|---|---|---|---|
|macOS ARM64|Mach-O arm64|本机 macOS 27.3|通过 / 通过|
|macOS x64|Mach-O x86_64|本机 Rosetta|通过 / 通过|
|Linux x64|ELF x86-64|OrbStack Linux amd64 容器|通过 / 通过|
|Windows x64|PE32+ x86-64，MinGW|Debian trixie amd64 + Wine 10|通过 / 通过|

Tech Lead 在新的本地 clone 中按默认 CMake 配置从上游获取绑定，构建、CTest、安装、第一次 headless 导入和技术场景均成功。独立 QA 另行复建。表中为本地验证的执行环境。托管工作流另在 Windows Server、Ubuntu、macOS ARM64 和 Intel runner 上完成验证。首次编辑器动态发现扩展曾在退出时产生信号 11；固定安装流程在启动前登记扩展。[ADR-003](docs/architecture/ADR-003.md)

## 本地交付位置
导出的四个技术样例位于 build/p0 下的 macos-arm64、macos-x64、linux-x64 与 windows-x64 目录。macOS 直接打开 OFE.app；Linux、Windows 运行 OFE 或 OFE.exe，保留同目录的扩展库和 PCK。左右方向键提交水平移动。SHA256SUMS 记录文件摘要，NOTICES 存放本地验证包的上游声明。产物由 Git 忽略，复建步骤随源码管理。逐项结论见 [独立 QA 验收报告](P0_ACCEPTANCE_REPORT.md)。

## 负责人决策与复核
本轮遵循任务书 V1.0，接口与格式先冻结。公开接口的变更通过 ADR，重大范围调整由负责人批准。许可证沿用总纲建议，模块分配留待负责人一并确认。托管运行与产物已核对，由负责人决定 P0 是否通过。
