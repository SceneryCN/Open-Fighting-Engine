# P8 跨平台候选交付 · 0.1.0-rc.1

本次交付包括 Windows x64、Linux x64、macOS ARM64 和 macOS x64 的游戏与模拟器，以及通用作者工具。各游戏包从 Sunset Viaduct 主菜单进入对战。文件、校验记录与版本对应的提交一起交负责人验收。[安装与运行](docs/p8/INSTALL.md) · [任务书](docs/p8/TASK_V1.md) · [架构决议](docs/architecture/ADR-011.md)

一号位使用 A/D 左右移动、W 跳、S 换线，J 或 O 普攻、K 防御、L 冲刺、U 特殊技、I 超必杀。二号位使用方向键和 F/G/H/R/T，游戏主菜单和暂停菜单均可查看完整操作。[全部席位与手柄操作](docs/p5/PLAY.md)

## 独立验证

编辑器、Release 导出及解包后的应用全部通过 37 个物理键的按下与释放测试。四人同时输入时，各席位命令准确。J、O、U、I 及二号位 F、R、T 在原生战斗中分别触发预期招式。

M1 Max 实窗两轮各采 600 帧，平均 59.512 和 59.993 FPS。直接运行导出程序时最长间隔 70.547 ms，原因尚未确定；仓库外解包运行时最长 23.341 ms。其他平台的图形帧率暂无实机测量。[独立 QA 报告](docs/testing/P8_ACCEPTANCE_REPORT.md) · [实测数据](docs/testing/evidence/p8-release.json)

公开标签与正式发行页将在负责人验收并授权后建立。[P7 交付归档](docs/p7/DELIVERY_REPORT.md)
