# P0 桥接验证

C 头文件位于 `integrations/godot/bridge/include/ofe_v1.h`。ABI v1 的整数错误码、输出结构和所有权约定见 ADR-001、ADR-002。C++ 对象 OFEBattle 通过 open、submit、step、read_state、close 管理一场模拟；释放最后一个 RefCounted 引用也会回收未关闭的实例。Godot Variant 的整数为有符号 64 位，超过其范围的帧编号读取返回容量错误。

## 独立于 Godot 的 C 调用

以下命令适用于 macOS ARM64 上的本机 Apple Clang。从仓库根目录执行。

```sh
cargo build --release -p ofe-bridge --locked
cc -std=c11 -Wall -Wextra -Werror -Iintegrations/godot/bridge/include \
  tests/integration/abi_smoke.c target/release/libofe_bridge.a -o target/abi_smoke
target/abi_smoke
leaks --atExit -- target/abi_smoke
cargo test --locked -p ofe-bridge
```

C 程序以发布版 Rust 静态库执行 100,000 次创建、两帧推进、状态复制、销毁与旧句柄访问。它还检查空指针、版本与容量、写入边界、失败后输出和状态保留。成功打印 OFE_ABI_PASS。Rust 桥接单测另外覆盖同进程多线程实例隔离、旧句柄、缩短的输出缓冲区。2026-11-26 本机运行两项 Rust 桥接单测通过，C 程序通过。leaks 报告 0 个泄漏、0 字节。系统对受限进程的可读内存报告有安全限制。此结果对应所测 C 生命周期路径。

## Godot 运行与自检

构建并安装匹配架构的扩展至 `integrations/godot/runtime/bin/` 后，通过项目场景运行。左右方向键提交下一帧水平输入，画面显示来自 Rust 的实体位置。每次物理 tick 提交一帧，绘制单独进行。命令行使用 `--headless --path integrations/godot/runtime -- --p0-test` 启用 256 次从 Godot 发起的完整调用循环，覆盖显式关闭及 RefCounted 自动销毁。成功标识为 OFE_P0_PASS，失败报告原因并返回 1。首次从源代码检出需要执行构建指南中的导入步骤。headless 自检只用于技术样例。
