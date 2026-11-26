# P6 确定性与回放 · 独立 QA

在 Apple M1 Max、macOS 27.3 ARM64、Godot 4.7.2 stable 上，分别测试源项目、独立构建的 Release 导出程序和 Release 模拟器。最终签收由项目负责人决定。[数据记录](evidence/p6-replay.json)

## 战斗与首个分歧

|测试|实际观测|
|---|---|
|单角色移动与双人攻击|原有单实体 600 帧测试从待处理输入快照恢复，每一帧哈希一致。Godot 训练局记录一人出拳命中另一人后行走，重新运行程序，从磁盘回放至同一位置并保留真实伤害。|
|连招、双线与四人战|四席各自成队，在第 1 帧出现同帧相互命中与飞行道具，第 26 帧观察到真实取消事件和二连击。第 40 帧触发切线，经过深度中间位置，随后第四位角色跳跃和移动。|
|快照恢复|在上述 100 帧过程的 8 个时点，将完整快照写入 JSON 并读回，恢复新实例及现有实例。每个恢复起点到末帧逐帧核对哈希、选手、事件和投射物。每次故意跳过一帧提交，原状态保持，可继续按正确帧提交。|
|错误快照|逐项修改快照版本、时钟数组、朝向、投射物来源槽位及战斗规则，五次恢复均拒绝。原实例哈希及下一次推进结果与对照实例一致。|
|PRNG 与跨进程哈希|固定 SplitMix64 样本通过，种子不同的初态哈希不同。最大 u64 种子的四人标准脚本终帧为 22，哈希为 `78b3a389c0be7054`。QA 使用独立 Python 编码重新计算了初始 FNV-1a，与 Rust 的 `b0fff557443b6004` 一致。四个固定帧哈希进入桌面 CI 回归。|
|首个分歧|修改第 3 帧的取消指令，同时改坏第 7 帧哈希，CLI 返回非零并报告第 3 帧及双方哈希。另将 100 帧脚本中的第 80 帧行走方向反转，首错为 80，预期 `767fe20bbf5e9a48`、实算 `b8c6d56ee601330c`。|
|真实录放|编辑器和 macOS Release 包分别用 F8/F10 录制并保存，CLI 在独立进程校验，然后重新启动游戏用 F11 读取。同次实测两者各 25 帧，末帧哈希均为 `37639bfe3db9acfd`。改坏磁盘录像第 7 帧后，错误提示定位到该帧，保留加载前的训练状态；F9 仍能完整回放上一份有效录像。|
|边界与回归|训练录满 36,000 帧后停止，存盘并重新加载，回放结束的完整展示快照与录制末帧一致。P0、P4、P5 检查仍通过，3 项 CTest 通过。|

独立检查推动了损坏录像的修复。初版读盘只检查结构和起始快照便替换内存录像。现有流程在后台完整验证后再替换，编辑器与导出程序均通过上述第 7 帧损坏用例。

## 性能

Release 原生测试使用四人混战规则，循环 6,000 帧。时间为实测墙钟微秒。

|操作|中位数|P99|最大|
|---|---:|---:|---:|
|战斗步进|0.125|0.625|6.042|
|捕获快照并编码 JSON|14.125|19.750|390.583|
|解码 JSON 并恢复实例|41.709|55.541|238.166|
|回放一步并计算哈希|2.125|3.875|17.541|

采样中最大的序列化快照为 11,926 字节。独立的分配器计数用例在四人、一个活动投射物时测得每个快照持有 5,488 字节（结构本体及其申请的堆字节），连续创建、丢弃 1,000 次均回到基线。这组数字只覆盖指定场景。

M1 Max 真实图形窗口限速 60 FPS，每轮先预热 120 帧，再采集 600 个绘制间隔。训练开启碰撞框、音乐，并分别录制和回放。录制平均 60.021 FPS，绘制 P99 为 21.930 ms，最长 24.197 ms。回放平均 60.002 FPS，绘制 P99 为 20.474 ms，最长 31.487 ms。两轮物理处理峰值分别为 3.090 和 3.064 ms。其他平台的图形帧率仍待对应硬件实测。

10,898,446 字节的满长录像在真实窗口后台核验读入，经 5 个绘制回调完成，耗时 83.712 ms；保存耗时 52.929 ms，期间收到 3 个绘制回调。提交读写操作分别耗时 0.123 / 0.045 ms。文件操作期间战斗暂停。

## 复现

在项目根目录运行。GODOT 指向本机 Godot 4.7.2，OFE_APP 指向同一版本代码导出的程序。

```sh
cargo test --locked -p ofe-sim --test p6_determinism
cargo test --locked --release -p ofe-sim --test p6_determinism -- --ignored --nocapture
cargo test --locked --release -p ofe-sim --test p6_snapshot_memory -- --ignored --nocapture --test-threads=1
cargo build --locked --release -p ofe-sim
python3 scripts/test/headless.py "$GODOT" --simulator target/release/ofe-sim
python3 scripts/test/headless.py "$OFE_APP" --exported --simulator target/release/ofe-sim
"$GODOT" --path integrations/godot/runtime --max-fps 60 --script "$PWD/scripts/test/p5_pacing.gd" -- --record
"$GODOT" --path integrations/godot/runtime --max-fps 60 --script "$PWD/scripts/test/p5_pacing.gd" -- --replay
```
