# P0 独立 QA 验收记录 · 2026-11-26

**阶段状态 TESTING，独立 QA 建议签收，待项目负责人决定。** 首轮本地复核时 E8 因未关联远端被列为 BLOCKED。随后 main 的 push 触发托管 CI，本次追加复核确认 E8 通过。验证者为独立 QA Agent，使用 feature/qa 工作树与单独的干净 clone。项目负责人保留最终决定权。下表的「通过」表示该检查有相应的 QA 证据。

## 检查清单

编号依照 V1.0 第十七章各组原始顺序。依据保存在 [QA 运行摘录](docs/testing/evidence/qa-execution.txt)、[构建与复现说明](docs/development/GETTING_STARTED.md) 及 [桥接测试说明](docs/testing/BRIDGE.md)。

|编号|验收项|QA 结论与依据|
|---|---|---|
|A1|Core 独立|通过。cargo tree 的 normal 边仅有 ofe-combat-core 自身，源码使用 Rust std。|
|A2|唯一权威实现|通过。逐层审查：输入在 Godot 采集，C++ 适配，位置与帧数只在 Battle::step 更新。|
|A3|表现与核心职责|通过。Godot 读取状态后绘制，投影系数未进入模拟或哈希。|
|A4|模块依赖|通过。架构图与 Cargo、CMake 依赖一致。|
|A5|选型有 ADR|通过。复核 ADR-001、002、003 以及锁定的 Rust 与 Godot 版本。|
|A6|坐标规范|通过。i32 X 向右、Z 向后、H 向上；规范列有战线单位与投影公式。|
|A7|固定帧|通过。60 Hz 宿主、单 tick API、600 次恢复后逐帧比对。|
|A8|版本管理|通过。C 函数名前缀、size 与 version 校验；版本错误输出不动。|
|A9|所有权|通过。调用方持有输出内存，token 对应的实例由 Rust 持有，RAII 清理。|
|B1|加载扩展|通过。真实编辑器先导入、headless 运行可实例化 OFEBattle。|
|B2|创建实例|通过。Godot smoke 循环创建并检查返回值。|
|B3|读取初始结构|通过。frame=0、entity_id=1，x/z/h 均为 0。|
|B4|提交输入|通过。有效输入成功，非法轴及错误帧返回对应错误。|
|B5|推进|通过。submit(1,-1) 后 step 成功。|
|B6|读取更新|通过。frame=1 且 x=-1，C ABI 检查新状态及尾部哨兵。|
|B7|销毁|通过。256 次 Godot 循环覆盖显式 close 和释放最后一个 RefCounted 引用。|
|B8|错误路径|通过。C 测试覆盖 NULL、小缓冲区、错误版本及重复提交，失败后可重试。|
|B9|无效句柄|通过。旧 token 的读取、推进和二次销毁均能拒绝。|
|B10|重复生命周期|通过。C 调用方 100,000 次，独立 C++ 并发测试另建 4,096 个实例。|
|B11|泄漏检查|通过所测路径。Apple leaks 对 C 程序报告 0 bytes；受限进程可见性有限。结合对象所有权审查及重复运行结果，无已知实例泄漏。|
|C1|OFC Schema|通过。Draft 2020-12 元 Schema 校验。|
|C2|OFC 合法|通过。递归验证角色、招式、判定框、动画、授权与资源。|
|C3|OFC 非法定位|通过。错误包含文档路径、JSON Pointer、格式版本或资源引用链。|
|C4|OFA Schema|通过。分别测试 PNG 序列与 sprite sheet 数据。|
|C5|帧与资源结构|通过。验证帧时长、原点、事件、索引范围及 PNG 引用。|
|C6|OGP 结构|通过。完整项目 fixture 递归校验通过。|
|C7|OFL Schema|通过。六种屏幕、元素约束及重复 ID 诊断。|
|C8|版本字段|通过。四格式分别拒绝 1.1。|
|C9|合法样例|通过。valid 项目图以 CLI 实际读取。|
|C10|非法样例|通过。invalid 中每种格式均在预期字段失败。12 个格式测试全部通过。|
|D1|Battle|通过。new/submit/step/snapshot/restore 经 Rust 和 ABI 路径验证。|
|D2|Entity ID|通过。类型为 u32，P0 单实体 ID 1，恢复拒绝非法 ID。|
|D3|固定帧接口|通过。输入面向下一帧，空输入中立，非法操作原子失败。|
|D4|坐标类型|通过。Position/Snapshot 定义 X/Z/H 整数，边界溢出有测试。|
|D5|输入结构|通过。带帧号和方向，测试冲突和跨帧。|
|D6|事件结构|通过。成功移动后产生一次 MovementEvent，中立下一帧清除。|
|D7|快照接口|通过。待输入与模拟状态可恢复。独立计算的三组黄金哈希逐个通过。|
|D8|基础错误|通过。参数、帧冲突和溢出等错误均有类型和 ABI 映射。|
|D9|基础测试|通过。Rust 单元、集成及 nextest 均执行。|
|E1|Rust 单测|通过。rustfmt、Clippy -D warnings、cargo test、nextest。|
|E2|桥接集成|通过。C 程序、并发 C++ 程序与真 Godot 进程。|
|E3|headless|通过。安装扩展后，冷启动导入与场景自检均成功。|
|E4|Windows x64 构建|通过。MinGW 目标 PE32+ x86-64 DLL 具有 ofe_godot_init，QA 核对产物。|
|E5|Linux x64 构建|通过。ELF x86-64；在 Linux amd64 容器执行官方 Godot，OFE_P0_PASS。|
|E6|macOS ARM64 构建|通过。QA 自行在 macOS ARM64 拉取固定依赖并构建，从空缓存完成导入与执行。|
|E7|macOS x64 构建|通过。Mach-O x86_64；Rosetta 下 x86_64 Godot 加载并执行成功。|
|E8|CI 自动执行|通过。提交 1cb8609 的 push 自动触发 run 45783184524，5 个任务全部成功，列出四个平台产物。|
|E9|构建结果|通过。核对四目标格式，记录命令和运行层级。|
|E10|失败原因|通过。首次导入的 SIGSEGV 可复现，CMake 安装预注册并在全新 clone 复测通过。|
|F1|依赖许可|通过初审。核对 DEPENDENCIES、Cargo.lock、Python 哈希锁及单独的 validator-dependencies。|
|F2|素材政策|通过。LICENSE_AUDIT 规定权属、许可、署名；合成像素有样例声明。|
|F3|README|通过。入口连接技术验证、计划、构建与文档。|
|F4|架构文档|通过。模块图、数据与帧流、ABI、坐标、格式与风险均可追溯。|
|F5|环境指南|通过。在独立 clone 验证构建、安装、headless 路径。|
|F6|风险清单|通过。架构 overview、阶段状态以及本报告说明剩余风险。|
|F7|交付报告|通过。首轮已审阅 DELIVERY_REPORT.md（0cca3f2）中的运行层级与负责人权限，托管 CI 后续补证见下。|
|F8|独立 QA 报告|已出具。本文件由 QA 在独立工作树编写、保留独立执行证据。|

## 首次导入故障与复测

macOS ARM64 上，在未生成扩展注册清单的项目中，Godot 4.7.2 首次 editor --import 扫描到扩展后，退出时触发 signal 11。在 4.5 和 4.7 两套绑定下均复现。首次失败产生的缓存可让第二次启动成功，因此只重复已有缓存的导入不能验证修复。最终 CMake 安装阶段预注册 res://ofe.gdextension；QA 从独立 clone 拉取固定 godot-cpp、生成扩展、安装、首次导入并执行成功。手动删除 .godot 后应重新安装扩展。

## 跨平台产物复核

同一套 Godot 4.7.2 运行时自检均打印 OFE_P0_PASS。QA 独立打开每个导出的程序，以退出码及成功标记复核。测试日期为 2026-11-26，宿主为 macOS 27.3 ARM64。

|目标|实际运行位置|编辑器场景 / 导出程序|
|---|---|---|
|macOS ARM64|本机 arm64|通过 / 通过，Mach-O arm64|
|macOS x64|macOS Rosetta|通过 / 通过，Mach-O x86_64|
|Linux x64|OrbStack Linux amd64 容器|通过 / 通过，ELF x86-64|
|Windows x64|Debian trixie amd64 容器，Wine 10.0|通过 / 通过，PE32+。ABI 和并发测试 EXE 也通过|

Wine 8 的首次检验因兼容层缺少 bcryptprimitives.dll 失败。升级到独立 Wine 10 容器后原 DLL 和原 EXE 均可运行。后续托管验证通过 Windows Server 2022 x64 与 macOS 15 Intel runner。

## 九项否决门槛

已验证 Godot 调用 Rust、Core 独立、规则唯一、固定帧接口、四格式 Schema、自动化测试、四目标可执行构建流程、ABI 所有权与干净源树的实际构建。第九项的复建环境为新源目录及空 CMake 目录，复用了主机编译器、SDK 和 Cargo 下载缓存。后续托管工作流在四种 runner 中分别检出源码、构建、运行 CTest、安装扩展、执行 Godot 场景与导出程序自检。

## GitHub Actions 独立补证

QA 从 GitHub 运行详情、各 job 步骤和产物列表交叉核对。提交 `1cb8609b845fa3672c5b451c93b025fd4260ad1c` 的 push 自动触发 P0 工作流，第 1 次尝试的总结果为 Success。[运行详情](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45783184524)

|验证目标|托管 runner|任务记录|
|---|---|---|
|macos-arm64|macos-15|[通过 · 139783380466](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45783184524/job/139783380466)|
|windows-x64|windows-2022|[通过 · 139783380500](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45783184524/job/139783380500)|
|linux-x64|ubuntu-24.04|[通过 · 139783380633](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45783184524/job/139783380633)|
|macos-x64|macos-15-intel|[通过 · 139783380696](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45783184524/job/139783380696)|
|质量检查|ubuntu-24.04|[通过 · 139783380785](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45783184524/job/139783380785)|

质量任务完成格式检查、Schema 测试、Rust 检查、单测及 nextest。四个平台任务的构建、编辑器与导出程序验证、上传步骤均成功。下表记录 GitHub 显示的压缩包大小与 SHA-256。本轮按公开元数据核对，未独立下载并重新计算归档哈希。

|产物|大小|归档 SHA-256|
|---|---:|---|
|ofe-p0-linux-x64|28.1 MB|`b23f1ebf528b583066761fc5f5f783d0f4f01520c569672a1c212293d3ae8a73`|
|ofe-p0-macos-arm64|27.9 MB|`7e2494cacfe2a1dfedc15fef2b0f9e24b4bf29fcc2cac3a1a632a0b06c913268`|
|ofe-p0-macos-x64|30.7 MB|`28d9c55ab14226320608e874e4c44398cf42c6b39c6d09ea63ff4ccddfb01e3c`|
|ofe-p0-windows-x64|36.3 MB|`39085d13aa1dd484945252ec5b15d031f05808af615243e8fb0713de0f924779`|

## 签收

独立 QA 建议项目负责人签收 P0。第十七章的技术检查已全部取得验证记录，原 E8 阻塞已解除。项目负责人决定是否批准本阶段；当前尚无负责人签收记录，P1 未启动。

## 项目负责人签收

2026-11-26，项目负责人在收到独立 QA 结论和托管 CI 结果后确认通过并授权启动 P1。阶段最终状态为 ACCEPTED。以上独立 QA 记录保留原貌。
