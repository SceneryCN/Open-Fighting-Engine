# P4 Godot Runtime 交付报告

默认主场景现在直接启动真实双人对战。Rust 管理对局并在固定 60 Hz 推进双方输入；Godot 显示 OFA 图集、角色朝向与双线位置、生命与能量，并根据命中和格挡事件播放音效。R 可重开，离树再入树会重建对局。P0 的原生 ABI 和桥接验证场景保留。[运行方法与控制](docs/p4/RUNTIME.md) · [ABI 与场景契约](docs/architecture/ADR-007.md)

独立外部角色用例将双方初始生命分别设为 137 和 191，第一方伤害设为 23；使用真实 Godot 输入后，第二方的 Rust 生命与画面血条均为 168。原生 ABI 测试核对了句柄隔离、调用方缓冲区边界，以及无效输入后的逐字节状态一致性。74 项 Rust 测试和 3 项 CTest 均通过。

发布包包含游戏、当前源码获取路径、OFE、Godot、godot-cpp 和 Rust 依赖的许可声明。P4 的交付边界为双人 Godot 技术场景；角色选择、VS、回合与胜负流程由 P5 负责。[托管构建与下载](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45802105150) · [P3 交付归档](docs/p3/DELIVERY_REPORT.md)

独立 QA 在 Apple M1 Max 的真实窗口测量四轮，每轮预热 120 帧后采集 600 个绘制间隔。持续攻防与每 64 tick 重开两类负载的平均值为 59.846–60.023 FPS。持续攻防的物理处理峰值为 6.645 ms；频繁重开时最高 15.576 ms。绘制间隔峰值达 60.255 ms，仍能观察到个别长帧。[详细测试与分位数](docs/testing/P4_ACCEPTANCE_REPORT.md) · [实际对战截图](docs/testing/evidence/p4-match.png)

正常启动沿用设备刷新设置，战斗时钟固定为 60 Hz。另一次不传 `--max-fps` 的本机持续攻防实测，在 5.655 秒内绘制 600 帧并推进 338 个物理 tick，平均绘制 106.11 FPS。该结果与上述明确限制绘制为 60 FPS 的四轮采样分别记录。

完整桌面 CI 已通过。Linux x64、Windows x64、macOS ARM64 和 macOS x64 的编辑器与导出程序均实际运行了对战自检，四套 ofe-runtime 归档与四套 ofe-sim 归档均已上传。P4 待项目负责人签收。
