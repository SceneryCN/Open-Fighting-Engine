# P7 第三方作者与项目启动 · 独立 QA

在 Apple M1 Max、macOS 27.3 ARM64、Godot 4.7.2 stable 上，外部 Rust 作者可生成角色，并从游戏的选人界面进入真实对战。独立导出的 macOS 应用与作者工具包分别归档，再解到仓库以外执行。最终签收由项目负责人决定。[机器记录](evidence/p7-author.json)

|检验|实测结果|
|---|---|
|独立制作|仅复制 SDK、示例作者工程与固定工具链配置到临时目录。Cargo 通过路径依赖编译两个 MIT SDK，生成的 10 个角色文件与发布样例一致。比较时统一文本换行，PNG 逐字节核对。|
|作者修改|直接编辑示例 Rust 源的名称、ID、生命上限、招式伤害和帧锚点，分别生成 QA Rook 与 QA Echo。作者生成物保持为开放的 OFC/OFA 文件。|
|项目归档|以示例 Amber 项目为基础，改标题、模式和纵深系数。工具成功校验并检查项目目录、入口文档、OGP、OFC 及角色目录。打包后删除源项目与 Rook 目录，使用 OGP、Rook OFC 与 Echo 目录混合启动。|
|菜单与真实对战|默认菜单显示 QA Open Dojo，唯一战斗入口为 Local duel。选人顺序为 Amber、QA Rook、QA Echo。将两位追加角色分别选入 P1、P2，确认 VS 页显示作者名称并点击 Fight。原生初始生命为 137 和 113。可见精灵使用外部贴图，帧锚点为 (9,17)，渲染偏移为 (−9,−17)。纵深系数为 0.067。玩家输入触发原生命中，伤害 23，P2 生命及 HUD 同时降至 90。|
|失败处理|同 ID 的目录与 OFC、缺失属性文件、损坏 OGP、未知 OFC 版本、角色依赖未满足、项目依赖未满足，六种启动均返回非零。每次检查子程序未启动且暂存目录为空。成功启动后检查项目解包目录、OFC 解包目录及描述文件已清除，用户自有目录仍可读取。|
|历史回归|Rust 工作区常规测试 85 项通过，另以 Release 单独执行 2 项快照性能与内存测试并通过。CTest 3 项、格式测试 16 项、P3 包测试 8 项通过。P0、P4、P5、P6 均在编辑器与独立导出执行。两边各自录制的 25 帧均由独立模拟器验证，末帧哈希为 37639bfe3db9acfd。|

## 加载与帧率

从启动 CLI 到它验证项目、准备数据并进入子进程，编辑器链路的一次测量为 429.2 ms，仓库外发布包为 456.6 ms。此处包括启动 Python 与读取归档的耗时。自动化分别在两个进程中实际跑到首次命中后退出，总耗时为 4.065 秒及 2.464 秒。

独立 Release 导出以 60 FPS 限速打开真实图形窗口。两位第三方角色完成首次命中后预热 120 帧，随后在防御切换期间采集 600 个绘制间隔，同期原生推进 600 帧。平均为 **59.983 FPS**，P95 为 **19.805 ms**，P99 为 **20.986 ms**，最长 **23.728 ms**，超过 25 ms 的帧为 **0**。单帧有超过 16.67 ms 的情形。该记录覆盖这次双角色运行，其他设备和显示平台的图形表现仍待实测。

## 复现

先安装作者工具的 Python 依赖，构建并安装 Godot 桥接，再导入游戏项目。GODOT 指向编辑器程序，OFE_APP 指向独立导出。--kit 可指向解压后的作者工具包。

```sh
python -m pip install --require-hashes -r tools/ofe-validator/requirements.txt
python scripts/test/headless.py "$GODOT" --simulator target/release/ofe-sim
python scripts/test/p7_author.py "$GODOT"
python scripts/test/headless.py "$OFE_APP" --exported --simulator target/release/ofe-sim
python scripts/test/p7_author.py "$OFE_APP" --exported --kit "$AUTHOR_KIT"
python scripts/test/p7_author.py "$OFE_APP" --exported --pacing
```
