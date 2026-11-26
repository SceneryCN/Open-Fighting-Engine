# P1 双人逐帧模拟器

在仓库根目录运行。两名玩家共用一个终端，每行先输入玩家 1 的动作，再输入玩家 2 的动作，用空格分开。回车让 Rust 核心推进一个 60 Hz 模拟帧。终端逐帧显示双方的生命、状态、朝向、X/Z/H 和该帧事件。

```sh
cargo run --locked -p ofe-sim
cargo run --locked -p ofe-sim -- --script tools/ofe-sim/fixtures/duel.frames
```

|输入|作用|
|---|---|
| - |中立|
| L、R |向左、向右|
| J |跳跃|
| S |切换战斗线|
| A |普通攻击|
| G |本帧保持防御|

每名玩家的输入写成一个 token，例如 RJ 表示本帧请求右移与跳跃。动作优先级、可否执行和命中结果由核心决定。空行与 # 注释可放入脚本。EOF 结束会话。每次启动从同一初始状态开始。

脚本第 2 帧双方互击，生命由 60/75 变为 48/60。第 8 帧输出 Blocked，格挡方生命保持 60。第 12 帧玩家 1 切线，玩家 2 跳跃；第 26 帧双方隔线出招，Indigo 的 BOTH_LANES 命中，Amber 的 SAME_LANE 落空，双方生命为 36/60。

例如输入 A G，再输入 - G，让玩家 1 起手并观察玩家 2 格挡的结果。将第二行改为 - - 可观察相同距离下的命中。两名角色在场景中起始相距 4 单位，分别朝向对方。

样例位于 fixtures/amber 与 fixtures/indigo。各自 manifest 引用属性、招式与矩形文件。双方生命、伤害和双线规则取自各自 OFC；fixtures/scene.json 提供出生点、移动、跳跃与切线参数。样例随程序编译，独立运行不依赖当前工作目录。修改样例后重新校验并编译。

```sh
python tools/ofe-validator/ofe.py validate tools/ofe-sim/fixtures/amber/manifest.json
python tools/ofe-validator/ofe.py validate tools/ofe-sim/fixtures/indigo/manifest.json
cargo test --locked -p ofe-sim
```
