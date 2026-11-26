# P0 至 P2 复核 · 2026-11-26

|阶段|复现与修复|验证|
|---|---|---|
|P0 桥接|同一 proof 节点移出场景树后重新挂入，会访问已释放的 Rust 战斗句柄。退出时释放引用并请求下次入树重新初始化。|原始失败脚本在新工作树安装后复测成功，重新入树后 status=0、frame=2。常规桌面 CI 加入连续三次重入的回归。默认 CMake 干净检出安装、首次编辑器导入和 256 次句柄生命周期测试通过。Godot 4.7.2 首次导入仍需使用仓库的安装步骤预注册扩展。|
|P1 数据|校验器接受整数值 60.0，Rust 的整数反序列化拒绝读取。P1/P2 共享精确整数解码；校验器保留十进制精度。|从角色数据读取 60.0 并跑完整场对局，附有指数、边界与近整数回归。|
|P2 多人结算|复用逐帧暂存及接触缓冲区，减少重复查询。|独立 QA 核对七组确定性轨迹，包括结算溢出与重试。优化后 72,600 次计时均在 16.667 ms 内。[逐轮压力测试和原始数据](P2_FRAME_BUDGET.md)|

## 真实窗口

macOS ARM64、Apple M1 Max、Godot 4.7.2、Compatibility 渲染器。测量 P0 技术场景的 frame_post_draw 回调间隔，预热 120 帧，采集 600 个间隔。9.999332 秒内推进 600 个物理 tick，平均 60.004 FPS。P50 为 16.663 ms，P95 为 17.751 ms，P99 为 19.266 ms，最大为 21.644 ms，超过 25 ms 有 0 次。该样本覆盖桥接技术场景；完整对战画面的帧时间需在 P4 实测。

```sh
godot --path integrations/godot/runtime --max-fps 60 --script "$PWD/scripts/test/render_pacing.gd"
```

[复核版本及四目标 CI](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45792503484)
