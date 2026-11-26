# P3 角色与动画运行时交付

外部 OFC 角色可从目录加载，或通过作者工具从 .ofc 归档加载。双人模拟使用包中的属性、动作及判定框。OFA 播放器可在无图形环境中按 tick 输出序列帧、图集区域、原点与进入事件。[复现命令](docs/p3/CHARACTERS.md) · [公共接口](docs/architecture/ADR-006.md)

[P0 至 P2 修复与 60 Hz 测试](docs/testing/P0_P2_REVIEW.md) · [P2 交付归档](docs/p2/DELIVERY_REPORT.md)

本机 release 产物从独立目录解包，可在包内运行作者工具、制作 amber.ofc 并完成 26 帧对局。独立 QA 执行 73 项 Rust 测试、15 项格式测试及 8 项外部 CLI 测试。修改外部角色的生命和伤害后，同一可执行文件的命中结果从 114 HP 变为 181 HP；单独修改动画事件和帧时长，两次对战输出逐字节相同。

当前 Arena 的角色绑定需要一个攻击框和一个受击框；角色读取层可保留多个框组。P3 的 PNG 支持读取图像资源、帧索引、原点和图集区域。完整的 Godot 对战画面及其帧时间在 P4 验证。

[独立 QA 记录](docs/testing/P3_ACCEPTANCE_REPORT.md) · [P3 四目标发布与回归](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45794232658)

托管运行 45794232658 已成功。Linux x64、Windows x64、macOS ARM64 和 macOS x64 均通过真实编辑器、导出程序及外部角色发布包回归，共上传四份模拟器和四份 Godot 导出包。当前状态为 TESTING，待负责人签收。
