# P0 性能记录 · 2026-11-26
在 macOS 27.3 ARM64 上，以 rustc 1.100.0 和 Criterion 0.5.1 的 release profile 测量单实例、单实体 P0 路径。每项预热 1 秒，采集 20 个样本，测量 2 秒；默认置信度 95%。机器同时参与构建，结果用于建立回归对照。

|路径|估计时间|区间|
|---|---:|---:|
|提交输入、推进一帧、读取移动事件|2.9071 ns|2.8967–2.9193 ns|
|复制快照、计算状态哈希|9.9720 ns|9.4221–10.994 ns|

```sh
cargo bench --locked -p ofe-combat-core --bench frame -- --warm-up-time 1 --measurement-time 2 --sample-size 20
```

该基准只测 Rust 内核，未包含 Godot、FFI、渲染、碰撞或未来四人战斗工作量。四人完整模拟的 16.67 ms 帧预算须在对应阶段验证。原始统计输出见 evidence/criterion-p0.txt。
