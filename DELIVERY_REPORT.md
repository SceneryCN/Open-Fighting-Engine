# P1 Combat Core MVP 交付记录 · 2026-11-26

P0 已由负责人签收。P1 交付独立运行的 Rust 双人核心及终端模拟器。阶段状态 TESTING，项目负责人负责最终签收。[P0 交付记录](docs/p0/DELIVERY_REPORT.md) · [P1 实施方案](docs/p1/PLAN.md)

## 运行与复核

```sh
cargo run --locked -p ofe-sim -- --script tools/ofe-sim/fixtures/duel.frames
cargo run --locked -p ofe-sim
```

前一条命令重现 26 帧对局。第 2 帧发生相向命中，双方生命从 60/75 变为 48/60。第 8 帧 Amber 的攻击被格挡。第 26 帧 Amber 位于后线，Indigo 位于前线，Indigo 的双线攻击命中，结果为 36/60。交互模式每行接收两名玩家本帧的输入。[操作说明](tools/ofe-sim/README.md)

|交付项|实现与证据|
|---|---|
|固定帧双人核心|Duel 接收相邻帧号和双方输入；统一结算后公开状态与事件。错误帧保留前帧结果。|
|移动、跳跃、切线|整数 X/Z/H，地面与空中移动，分阶段切线、冷却与可中断配置。|
|普攻与防御|按招式帧窗、双线规则、朝向和整数 X/H 矩形检测。有效窗去重，同帧伤害、格挡、受击、击退和 KO 有测试。|
|数据与 API|两份 OFC 1.0 样例提供生命、普通攻击和判定框；场景提供出生位置和移动参数。[接口决议](docs/architecture/ADR-004.md)|
|自动化与基准|测试覆盖多帧确定性、同时 KO、坐标溢出与错误原子性。实测双向命中约 29 ns/帧。[性能数据](docs/p1/PERFORMANCE.md) · [独立 QA](docs/testing/P1_ACCEPTANCE_REPORT.md)|
|构建|四个桌面目标的 CI 分别测试核心，构建、运行并打包 ofe-sim，另跑 Godot ABI v1 回归。[托管构建](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45785808701)|

## 已知限制与风险

模拟器随程序编入两个样例角色，采用逐帧终端操作。P1 的碰撞与基准覆盖每人一个攻击框与一个受击框。完整角色包加载、Godot 对战表现和四人模式各有后续里程碑。对外使用 Rust 双人接口；C ABI v1 继续提供既有的单实体功能。P2 须另获项目负责人授权。
