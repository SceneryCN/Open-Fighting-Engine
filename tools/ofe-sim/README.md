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

## P2 四人对战

每行依次输入 P1、P2、P3、P4 四个动作。2v2 由 P1 与 P2 对阵 P3 与 P4；ffa 为四人自由混战。默认输入源为键盘，也可重放同一个文本脚本。运行时打印每名角色的队号、生命、能量、状态、位置、朝向、招式索引、连击数、剩余 hitstop 和冲刺状态，同时列出场上投射物及帧事件。

```sh
cargo run --locked -p ofe-sim -- --mode 2v2
cargo run --locked -p ofe-sim -- --mode ffa --script tools/ofe-sim/fixtures/arena.frames
cargo run --locked -p ofe-sim -- --mode 2v2 --script tools/ofe-sim/fixtures/arena.frames
```

|四人输入|动作|
|---|---|
|L、R、J、S、G、-|移动、跳跃、切线、防御、中立，含义与双人控制相同|
|D|冲刺，可与 L 或 R 组合指定方向|
|A 或 0|普通打击 jab|
|1|多段 flurry，可在命中后取消至招式 4|
|2|投技 grab|
|3|投射物 bolt|
|4|消耗 20 能量的超必杀 nova|
|5|带两次护甲的 brace|
|6|启动期间有两帧无敌的 slip|

例如输入 `1 - - G` 发起多段攻击，P4 保持防御。后续每行继续分别指定四名角色的动作，输入 `4 - - G` 可在首段已命中且取消窗口开放时追击。每个槽位同一帧最多请求一个招式。多个事件在同一个 frame 标题下出现时，由核心统一结算。

脚本第 1 帧，2v2 中 P1 命中 P3，FFA 中 P1 同时命中 P2 与 P3。第 2 帧经过击停，第 3 帧 P1 取消至超必杀，能量从 40 降至 20，连续命中数达到 2。第 9 帧可观察投射物实例。

四个槽位共用原创新建的 OFC 1.0 技术角色 arena_probe。manifest 的 moves 顺序定义数字键索引；OFC 各招式文件提供帧数、伤害、格挡硬直、击停、护甲、无敌、能量消耗及取消窗口。配套 move_profile.json 按 OFC 招式 ID 标注窗口、取消目标、投射物速度和寿命。arena_scene.json 选择本地 manifest 与 profile，并提供队号、出生点、移动参数和初始能量。以相同输入比较两个模式时，双方距离相同，队友保护与最近敌方选择可能改变命中对象。

```sh
python tools/ofe-validator/ofe.py validate tools/ofe-sim/fixtures/arena/manifest.json
python tools/ofe-validator/ofe.py validate tools/ofe-sim/fixtures/arena/move_profile.json
cargo test --locked -p ofe-sim
```

## 外部角色

在仓库根执行 `cargo run --locked -p ofe-sim -- --character tools/ofe-sim/fixtures/arena`，可用该目录中的角色创建镜像对战。每行两列输入分别控制双方，A 使用 manifest.moves 中的首个动作，@N 选择从零开始的招式索引（例如 `@3 -` 施放夹具的投射物）。输入脚本可通过 `--script FILE` 提供。位置与移动参数来自默认对战场景，初始能量受角色的 max_energy 限制。

角色目录在启动时读取。可修改属性或招式后重跑同一可执行文件。现有 Arena 配置接收一个共享攻击框与一个受击框，外部对战入口在几何不明确时报告错误。角色层仍可读取包含其他框组、多个几何框或仅用于展示的角色包。带取消窗口的动作需要根目录中的 move_profile.json 提供目标。
