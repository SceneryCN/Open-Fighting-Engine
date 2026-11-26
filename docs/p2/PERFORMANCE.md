# P2 多人结算基准 · 2026-11-26

macOS 27.3 ARM64，提交 993e269，Rust 1.100.0、Criterion 0.5.1、release。预热 1 秒，测量 2 秒，每项 25 组样本，表中为估计值与 95% 置信区间。四人 FFA 的压力场景让所有攻击框与受击框重叠，采样前断言一次推进确实产生 12 次有方向的命中。

|场景|每帧估计|区间|
|---|---:|---:|
|四人待机|215.29 ns|213.49–219.74 ns|
|四人同帧 12 次命中|1.1027 µs|1.0910–1.1119 µs|
|双人兼容入口待机|176.98 ns|173.59–183.88 ns|
|双人兼容入口同帧互击|418.52 ns|415.18–422.71 ns|

P1 基线的双人互击约 29 ns。P2 每帧暂存实体与投射物、汇集并排序接触，再统一提交，双人兼容入口在本机也有相应耗时增长。即使是 12 次命中的样例也处于 60 Hz 单帧 16.67 ms 的预算内。这些结果仅覆盖列出的负载；大量长寿命投射物会增加每帧复制和碰撞成本，尚无针对任意并发投射物数量的性能保证。

```sh
cargo bench --locked -p ofe-sim --bench arena -- --sample-size 25 --warm-up-time 1 --measurement-time 2
cargo bench --locked -p ofe-sim --bench duel -- --sample-size 25 --warm-up-time 1 --measurement-time 2
```

测量范围为 Rust 核心与只读观测接口，配置解析、对局构造、Godot 与终端输出均在计时区外。[原始测量](../testing/evidence/p2-criterion.txt)
