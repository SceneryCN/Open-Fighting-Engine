# P1 独立 QA 验收

验收范围取自项目总纲 V1.0 第十章「Combat Core MVP」。项目负责人已批准启动 P1。QA 在独立工作树执行，负责人签收单独记录。

## 验证计划

|观察对象|独立检验方法|
|---|---|
|固定帧、双人输入|两名角色同 tick 提交输入，按实体 ID 比对状态，每次成功推进只增加一帧。|
|坐标、移动与跳跃|分开比对 X/Z/H，逐帧观察起跳、空中及落地，检查边界。|
|切线|跟踪纵深方向和阶段边界，复查冷却与切线中的受击。|
|普通攻击、判定框|读取配置中的帧窗与几何，分别构造正面、背面、超距、高度分离与跨线情况。|
|伤害、防御、受击、击退|观察实际生命、位置、锁定状态与事件，验证一次攻击的多个有效帧仅对同目标造成规定次数的伤害。|
|同帧结算和确定性|相互攻击、重新排序输入、重复运行，逐帧比对权威状态和事件。|
|模拟器与性能|通过普通命令执行演示输入；观察有效的帧数、伤害/防御/事件输出，并执行基准。|
|P0 兼容|独立跑核心、格式、ABI 与真实 Godot 项目验证；检查版本化 ABI 和原状态哈希向量。|

最终结果须给出受检提交、命令、观测值、失败与修正记录、剩余风险和负责人签收状态。

## 独立执行结果 · 2026-11-26

**本机技术验收通过，P1 待项目负责人签收。** QA 在 macOS 27.3 ARM64、Rust 1.100.0、Godot 4.7.2 stable 上独立执行。验收的集成基线为 4302f1b；独立工作树 3e27418 在此基础上包含另外两条 QA 边界回归。系统工作负载会影响基准数值。托管 CI 的结果须按实际工作流记录另行核对。

|检查|观测值及结果|
|---|---|
|双人固定帧与整数坐标|两名实体在同一 tick 按各自轴移动 5 单位。第二人坐标溢出时，第一人的移动、双方状态、事件列表和帧号均恢复，使用合法输入可重试。|
|帧号、输入和状态机|重复帧号及非法轴均拒绝；失败帧保留上一次事件及动作计龄。起手后相应 active 首帧命中，连续 3 个 active tick 只结算一次。|
|命中几何|左右两个朝向在边接触时均未命中，向内 1 单位命中；对手走到攻击背面或跳出 H 范围可避开有效帧。|
|防御、伤害与击退|防御产生 Blocked、进入 Blockstun，HP 保持 100；未防御时从 100 变 75、进入 Hitstun，朝左击退 3 单位。|
|同帧结算|双方初始 HP 20，同时攻击造成两方向命中与两个 KO 事件，双方 HP 钳至 0，次帧均不能继续出招。|
|双线|切线到 Z=500 时 SameLane 与 DepthRange(499) 拒绝命中；CrossLane、BothLanes、DepthRange(500) 允许。可中断切线归入后线 Z=1000；不可中断时普通受击仍扣血并完成动作，结束及冷却边界逐帧验证。|
|跳跃与落地|逐帧观察 Jump、Fall、Idle，H 始终非负并最终回到 0。脚本中玩家 2 在第 20 帧落地。|
|确定性|独立运行两次 240 帧输入，每帧对比两个 fighter 和有序事件列表，全部相同。|
|OFC 与模拟器|amber、indigo 均通过独立 Schema 校验。脚本头两帧同帧互击并分别造成 15 与 12 伤害，之后出现格挡、跳跃、切线；第 26 帧出现跨线单向命中，最终 HP 为 36/60。stdin 提交 A G 后 - G 同样出现 Blocked。|
|独立包|本机 release 打包及解包成功，只用归档内的 ofe-sim 和 duel.frames 从新目录运行，终帧 26、HP 36/60，与仓库内执行一致。|
|P0 回归|Rust v1 快照哈希与 ABI 测试通过。重新链接含 P1 核心的 GDExtension，CTest 两项通过；真实 Godot 项目导入及场景自检输出 OFE_P0_PASS。原 12 项格式测试均通过。|
|基准|Criterion，release，预热 1 秒、测量 2 秒、20 组样本。双人空闲 tick 17.751–17.920 ns，同帧双向命中 tick 27.975–28.957 ns。计时包含模拟步骤，互击样例准备移至计时区间外。|

在全部工作区运行 rustfmt、Clippy（警告作错误）、cargo test 和 cargo-nextest。nextest 结果为 33 通过、0 跳过，其中 13 项来自独立黑盒 P1 套件。常规依赖树中 ofe-combat-core 依赖 Rust 标准库。

### 首轮拒绝与复核

QA 在核心 300c9e7 发现切线途中的致命命中令角色以 Z=500 永久停在两线中间。独立测试实际观测 500，按最近线规则期望 1000。核心维护者在 4392288 修正在进入 KO 前结束切线的位置处理；同一用例复测得到 KO 且 Z=1000。首轮 Clippy 报告的一处未使用导入也已修复并通过完整复查。

### 复现命令

在仓库根目录、固定工具链和已准备的 Schema 环境执行：

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-features --all-targets -- -D warnings
cargo test --locked --workspace
cargo nextest run --locked --workspace
cargo bench --locked -p ofe-sim --bench duel -- --warm-up-time 1 --measurement-time 2 --sample-size 20
cargo run --locked -p ofe-sim -- --script tools/ofe-sim/fixtures/duel.frames
python -m unittest discover -s tests/formats -v
python tools/ofe-validator/ofe.py validate tools/ofe-sim/fixtures/amber/manifest.json
python tools/ofe-validator/ofe.py validate tools/ofe-sim/fixtures/indigo/manifest.json
cmake --build build/native --parallel 4
ctest --test-dir build/native --output-on-failure
cmake --install build/native --prefix integrations/godot/runtime
python scripts/test/headless.py /Applications/Godot.app/Contents/MacOS/Godot
```

原始运行摘录与提交对应关系见 [P1 QA 证据](evidence/p1-qa.txt)。本地验证覆盖 macOS ARM64 的标准入口、真实桥接、两个角色的逐帧核心及命令行包。P1 的授权范围为双人 MVP，四人对战、完整 SDK 和图形战斗接入保留在各自阶段。
