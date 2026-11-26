# P1 双人模拟基准 · 2026-11-26

macOS 27.3 ARM64，Rust 1.100.0、Criterion 0.5.1、release，提交 4302f1b。每项预热 1 秒，测量 2 秒，30 个样本。使用模拟器同一组 OFC，计时区域为一次公开 Duel::step 和只读结果访问。双向命中用例在采样前断言同帧产生两个 Hit 事件。

|场景|耗时估计|95% 置信区间|
|---|---:|---:|
|两人地面待机|17.901 ns/帧|17.760–18.067 ns|
|双人普攻同帧结算，含判定框、生命、状态与事件|28.767 ns/帧|28.048–29.668 ns|

```sh
cargo bench --locked -p ofe-sim --bench duel -- --warm-up-time 1 --measurement-time 2 --sample-size 30
```

OFC 解析、起手准备和终端输出均在计时区外。该数据覆盖两名角色各一个攻击框和一个受击框的 P1 工作负载；四人对战与渲染有各自的测量范围。[Criterion 原始结果](../testing/evidence/p1-criterion.txt)
