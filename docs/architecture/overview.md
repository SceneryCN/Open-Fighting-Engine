# P0 架构与数据流

## 模块依赖

箭头从调用方指向被调用方。P0 构建、宿主和校验工具可以分别验证。

```text
Godot PoC (scene / input / projection)
    └─ C++ GDExtension
          ├─ godot-cpp → Godot extension API
          └─ ofe_v1_* (C ABI)
                  └─ Rust bridge → ofe-combat-core → Rust std

ofe validate → Draft 2020-12 validator + resource checker
                   └─ OFC / OFA / OGP / OFL
```

从 P1 起，拟定的逻辑层级为 Demo Game → Game Runtime → Character Runtime → Combat Core。Animation Runtime 消费表现状态，Resource Runtime 提供索引和资源。Combat Events 与 Snapshot/Replay 由权威帧产生，战斗规则和计时归 Combat Core。该部分是后续阶段的依赖设计。

## 帧数据流

```text
输入设备 → Godot 输入采集 → (next_frame, axis) → C ABI submit
                                                    ↓
                                     Rust 单槽下一帧输入缓冲
                                                    ↓
60 Hz 固定时钟 → C ABI step → 读取输入，未提交时使用中立方向
                                                    ↓
                                   校验并计算新坐标与帧号
                                                    ↓
                                    提交新状态，清空输入槽
                                                    ↓
Godot 投影 ← C++ 类型转换 ← C ABI read ← 已提交的权威状态
```

读取属于观察操作，在同一 tick 内可多次调用。Rust 快照包含权威状态和已接收的下一帧输入，C ABI read 输出渲染所需字段。快照草案用于重建状态并复查哈希。完整输入历史、战斗回放与攻击结算在后续阶段实现。

## 逻辑坐标与投影

P0 坐标为有符号 32 位整数，时间单位为 tick；1 秒等于 60 tick。一个逻辑单位是最小移动量。X 增大向右，Z 增大向后，H 增大向上。前线、后线分别取 Z=0、1000。P0 输入移动每 tick 至多一个 X 单位。Z、H 坐标作为结构化状态字段保留。P0 后的攻击判定仍在整数逻辑空间内。

显示层以固定镜头绘制 2D 对象，投影形式为 `screen_x = origin_x + X` 和 `screen_y = ground_y - depth_projection * Z - H`。屏幕原点平移和纵深系数由表现层设置，取浮点值亦不进入权威哈希。1.0 OGP 的 settings 保存 depth_projection 和 simulation_hz；P0 校验工具验证此配置，项目运行时加载由后续阶段接入。

## 决议与风险

接口签名、模块边界和所有权见 [ADR-001](ADR-001.md)，版本兼容、输出容量、失效句柄与冻结错误码见 [ADR-002](ADR-002.md)。

| 风险 | 验证方法 |
|---|---|
| ABI 端序、对齐或平台 ABI 出现差异 | 同步编译 C 头文件与 Rust 布局断言，各目标实际加载扩展并读取同一测试状态 |
| Godot 版本与绑定生成代码不匹配 | 锁定 Godot 和 godot-cpp，在真实 headless 项目做导入和帧推进 |
| 同帧输入由多个线程竞争导致接受顺序变化 | 宿主串行安排输入；验证重复提交错误及状态原子性 |
| Godot 对象或渲染值进入确定性数据 | 核心 crate 独立测试，对同一状态使用固定字段顺序计算哈希 |
| 资源目录在分发时改变层级或存在失效符号链接 | 在最终目录运行递归校验，错误包含每层 JSON Pointer 与文件名 |
| 媒体文件损坏但仍具有效标头 | P0 仅检查路径和 PNG 签名，后续资源运行时须测试实际解码 |
