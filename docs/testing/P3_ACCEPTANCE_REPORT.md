# P3 独立 QA 验收 · 2026-11-26

**本机技术复核通过，待项目负责人签收。** QA 在独立 Git 工作树检出根清单与锁文件提交 48677fc，并加入独立 consumer 用例（af99116）。在 macOS ARM64、Rust 1.100.0、Python 3.12.14 上重新构建 release 模拟器，从源码树以外的临时工作目录调用真实程序。

|总纲项目|独立观测|
|---|---|
|OFC 加载、属性、招式|独立构造两个招式的外部包。127 HP 的角色受到 13 伤害后剩 114。保持同一可执行文件，将外部数据改为 200 HP、19 伤害后剩 181。反转 manifest 的招式顺序后普通攻击伤害为 7，剩 193。旧归档保持旧结果。|
|OFA 帧、Sprite Sheet、动画事件|同一个 6×4 PNG，按 3×2 网格解析，索引 4/2/3 对应左上角 (2,2)/(4,0)/(0,2)，每个裁剪区域 2×2。序列帧使用整图。时长 2/1/3 的显示序列为 0,0,1,2,2,2；循环重入重发事件，非循环结束后持帧且无事件。修改表现时长、图集帧和动画事件后，战斗输出逐字节相同。|
|资源管理|相同文件由两个 owner 引用时共享同一 Arc。替换磁盘内容后缓存仍返回旧值；clear 后重读新值，释放 store 后旧 Arc 仍可读。已加载角色不依赖临时目录继续存在。|
|角色资源与配置校验|缺图、越界图集索引、网格不能整除、零时长、不可解析的 profile 关系和越界相对引用均被拒绝。错误的 .ofc 不能取代先前可用的成品。|
|角色打包、检查与运行|同一目录双次打包生成相同字节，归档按路径排序，只含可达数据及许可。目录和归档 inspect 的角色、作者、许可与有序招式/动画列表相同。stored 与 deflated 均通过；直接运行目录、CLI 运行目录及 CLI 运行归档的结果一致。脚本出错时透传退出码、输出并清理临时目录。|
|归档边界|对绝对/越界路径、大小写重复条目、链接、超过 4,096 个条目及单文件超过 64 MiB 的公开命令验证均返回失败。|

## 运行结果

- rustfmt、Clippy（包含所有目标且将警告作为错误）、cargo test 均通过。cargo-nextest 的工作区结果为 **73/73，0 跳过**，包含 5 项完全通过公开 Rust API 运行的 P3 consumer 测试。
- Python 格式测试 **15/15**；独立外部目录与归档验收 **8/8**。Python 进程实际执行本次工作树的 release 可执行文件。
- 本轮检测到集成期间两处测试工程问题：根 workspace/lock 尚未提交时 --locked 无法构建；consumer 初稿引用了未冻结的 getter。根清单与锁已纳入 48677fc，consumer 已改为读取实际公开字段并复测通过。
- 该验收覆盖本机 CPU 上的数据加载与播放时序。真实 Godot 显示属于 P4。四平台托管结果应按本阶段的实际运行记录另行核对。

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
cargo nextest run --locked --workspace
cargo build --locked --release -p ofe-sim
python -m unittest discover -s tests/formats -v
python -m unittest discover -s tests/p3 -v
```

Python 环境按 tools/ofe-validator/requirements.txt 哈希锁安装。CI 可用 OFE_SIM 指定本机 release 程序，否则从 target/release 自动选择平台文件名。测试保留在 tests/p3，独立夹具在运行时生成，测试用 PNG 随仓库提供。
