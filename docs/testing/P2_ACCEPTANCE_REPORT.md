# P2 Advanced Combat 独立验收记录

范围为项目总纲 V1.0 第十章 P2。项目负责人已接受 P1 并授权进入 P2。由独立 QA 在独立工作树逐项验证。下表为实施前设计，实际观测和结论将在集成后填写。

|总纲项|黑盒检测与组合边界|
|---|---|
|Dash|输入令角色跨越正确位移和帧数；与动作锁、受击、边界组合。|
|Hitstun / Blockstun|相同攻击在受击和格挡时分别观察扣血、动作锁、下一次可行动帧。|
|Hitstop|冲击帧后计时冻结与恢复；多人同帧命中不会随攻击枚举顺序叠加错误。|
|Cancel Window|对开窗前、首帧、末帧、关窗后输入同一动作，检查条件与能量。|
|连招|连续受击递增，脱离硬直后重置；多攻击者按冻结的归属规则计数。|
|多段攻击|独立有效窗各可命中；同一窗内对同一目标只命中一次，对不同目标分别记账。|
|投技|范围、格挡与免疫组合；多人同帧争夺和优先级遵循 ADR。|
|投射物|生成、移动、命中、未命中到期；敌我识别与生命期边界；重复输入不重复生成。|
|Armor / Invulnerability|分别比对血量、硬直、事件；验证有效时机、击数和绕过条件。|
|Super|能量不足与恰好足够，成功动作仅扣一次；验证普通攻击与取消分支。|
|2V2|两个友方和两个敌方的攻击同帧重叠，检查队伍过滤与多个独立目标。|
|四人自由混战|同时提交所有输入，按照各自 ID 检查多人命中及 KO。|
|同帧统一结算|反转合法的初始枚举及输入顺序，逐帧比较 ID 对应的状态与事件；同帧致命攻击相互结算。|
|P1 / P0 回归|P1 独立用例与模拟器、ABI 生命周期、真实 Godot 场景、Schema 验证。|

等待冻结契约后落实测试输入、预期、复现命令和运行证据。项目负责人拥有阶段签收权。

## 独立实测 · 2026-11-26

QA 环境为 macOS 27.3 ARM64、Rust 1.100.0、Godot 4.7.2 stable。黑盒测试仅通过公开 Arena、Duel 接口提交输入及读取结果。独立对照使用 P1 已签收基线 38bb8ab。

|项目|实测|
|---|---|
|冲刺|速度 5、持续 3 帧，X 依次为 5、10、15，随后正常向左走到 13。|
|受击/格挡与 hitstop|同帧停顿 1 与 3 取最大值 3。三个冻结 tick 内忽略目标的冲刺请求。之后分别观察 4 帧 Hitstun 或 3 帧 Blockstun，再次攻击在规定帧解锁。HP 46（命中）或 60（格挡）。|
|取消窗口与超必杀|年龄 2 至 3 的窗口，年龄 1/4 拒绝，2/3 接受。空挥及能量 9 无法转入消耗 10 的招式；能量 10 时只扣一次，并发出对应 Cancelled。|
|多段攻击与连招|两个窗口分别命中两个敌人。窗口内重复帧没有重复伤害；目标各剩 46 HP，攻击者连击数 2，目标脱离硬直后回到 0。|
|投技、护甲及无敌|投技绕过防御和护甲，无敌时未扣血；互投同时进入 Hitstun。前两帧无敌结束后第三帧消耗 1 护甲，维持动作但扣 7 HP，第四帧硬直。两名来源同帧攻击，较小 ID 消耗唯一护甲。致命一击在有护甲时仍进入 KO。|
|投射物|发射后原点判定，次帧移动 5。施放者已回到自由态时仍分别命中两名敌人，4 帧寿命结束后移除。延迟命中不会授权取消另一招。越界时重复 step 两次都保留原有帧号、双方状态、投射物和先前事件。|
|2V2 / FFA|同一四人重叠场景中，2V2 有 8 个有向命中、FFA 有 12 个。反转阵容和逐帧输入顺序，两种模式的状态及事件均相同。四个低生命角色同帧出招时登记 12 次命中与 4 个 KO。|
|模拟器|从本机 release 压缩包解出程序、duel.frames 和 arena.frames 后执行。2V2 第 22 帧 HP 为 150/150/111/143，FFA 为 143/118/102/143；均可观察能量变化和投射物。|
|P1 / P0|P1 的 26 帧 CLI 输出与旧版 golden 逐字节一致。P0 ABI 的 100,000 次生命周期与并发 CTest 均通过；重新链接的扩展在真实 Godot 进程输出 OFE_P0_PASS。原格式与 move profile 共 14 项校验测试通过，角色 manifest 与 profile 直接校验通过。|

### 兼容性缺陷记录

在 P1 接口上单独模拟 KO 后穿过已倒下对手：第 1 帧 KO，第 6 帧走至 X=2，第 7 帧中立，对手留在 X=1。QA 编译 38bb8ab 的历史核心并实测旧版 survivor_facing=-1；P2 初稿观察为 +1。此行为改动只出现在没有存活敌人之后，常规 P1 演示脚本未覆盖这一情况。已加入回归用例 legacy_duel_retains_facing_after_winner_walks_past_ko_opponent。

### 本机构建与复现

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo nextest run --locked --workspace
cargo bench --locked -p ofe-sim --bench arena -- --warm-up-time 1 --measurement-time 2 --sample-size 20
cargo build --locked --release -p ofe-sim
python3 scripts/release/package_sim.py build/qa-sim.tar.gz
cmake --build build/native
ctest --test-dir build/native --output-on-failure
cmake --install build/native --prefix integrations/godot/runtime
python3 scripts/test/headless.py /Applications/Godot.app/Contents/MacOS/Godot
python -m unittest discover -s tests/formats -v
```

## 最终本机复核

**独立技术验收通过。** 在 a29197e 执行 rustfmt、Clippy、cargo test、cargo-nextest，结果 56 通过、0 跳过，其中包含 16 项 P2 独立黑盒用例。修正提交 542b1f7 优先选择存活敌方，在所有敌人均已 KO 时仍保留位置比较。第 7 帧朝向用例实测为 -1，兼容性缺陷已关闭。

独立重建的 P0 CTest 两项通过，Godot 实际加载扩展并运行成功，14 个 Schema 测试通过，release 包中的两种多人模式执行正常。[原始运行摘录](evidence/p2-qa.txt)

阶段状态 TESTING，待项目负责人决定是否签收。托管 CI 与本机验证分别留证。
