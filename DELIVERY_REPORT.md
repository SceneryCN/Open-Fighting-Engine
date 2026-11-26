# P2 Advanced Combat 交付报告 · 2026-11-26

P1 由项目负责人签收后，P2 在 Rust 核心实现统一的 2–4 人 Arena。四人队伍对抗和自由混战均可独立运行。P2 状态 TESTING，最终签收由项目负责人决定。[实施方案](docs/p2/PLAN.md) · [P1 归档](docs/p1/DELIVERY_REPORT.md)

## 复现一场对局

```sh
cargo run --locked -p ofe-sim -- --mode 2v2 --script tools/ofe-sim/fixtures/arena.frames
cargo run --locked -p ofe-sim -- --mode ffa --script tools/ofe-sim/fixtures/arena.frames
```

两种模式共用同一份 22 帧输入。2v2 第 1 帧，P1 的首段命中 P3；FFA 中 P2 同时成为合法目标。2v2 第 3 帧，P1 命中后取消至招式 4，能量 40→20，连击数为 2。省略脚本参数即可手动输入。[控制与招式表](tools/ofe-sim/README.md)

## 接口和实现

|范围|可观测结果|
|---|---|
|动作、受击与防御|冲刺有方向和持续帧。受击硬直、格挡硬直与命中停顿分别计时。|
|招式链|有效窗口分别对每个目标去重；已命中的动作可以在配置窗口转入指定动作；连击按攻击者与目标跟踪。|
|特殊攻击与资源|即时投技绕过格挡与护甲；无敌会拒绝接触。护甲按命中次数消耗。超必杀开始时按配置扣能量。投射物离手后独立移动、命中并到期。|
|多人结算|输入按 ID 匹配，友方免伤。先采集同帧接触，再依稳定顺序处理护甲及累计伤害、击退、硬直；同帧 KO 仍保留已产生的攻击。|
|兼容与样例|双人 Duel 通过同一 Arena 执行。原 26 帧 CLI 输出有逐字节回归。OFC 1.0 配合版本化的招式补充文件，未修改旧角色包结构。|

[公共 Rust 契约与时序](docs/architecture/ADR-005.md) · [完整战斗测试矩阵](docs/testing/P2_ACCEPTANCE_REPORT.md)

## 验证、风险及待签收项

- 工作区 53 项 Rust 测试通过，包括 13 项 P2 独立黑盒测试。14 项格式测试通过。P0/P1 的 ABI、Godot 技术样例和双人接口保留独立回归证据。
- 本机从归档解包后分别运行双人、2v2 与 FFA，双人输出与黄金样本一致。四个平台执行同一套模拟器打包、自检及 Godot 验证。[集成提交托管运行](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45788357812)
- 四人同时产生 12 次命中的实测为 1.10 µs/帧。每帧暂存与动态接触集合增加了双人路径成本；长寿命投射物的高并发需按实际项目负载量测。[基准与复现命令](docs/p2/PERFORMANCE.md)
- 本阶段交付终端核心验证与确定性输入脚本。投技采用接触时立即结算的规则。图形演出、通用外部角色包加载、正式游戏回合与胜负画面属于后续阶段。P3 等待项目负责人签收 P2 并授权启动。
