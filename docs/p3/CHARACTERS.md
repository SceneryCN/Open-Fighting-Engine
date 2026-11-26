# 从角色目录到第一场对战

从仓库根运行以下命令。Python 3.12 安装格式工具依赖，Rust release 构建提供独立模拟器。Windows 的可执行文件名为 ofe-sim.exe。

```sh
python -m pip install --require-hashes -r tools/ofe-validator/requirements.txt
cargo build --locked --release -p ofe-sim
python tools/ofe-validator/ofe.py validate tools/ofe-sim/fixtures/amber
python tools/ofe-validator/ofe.py package tools/ofe-sim/fixtures/amber amber.ofc
python tools/ofe-validator/ofe.py inspect amber.ofc
python tools/ofe-validator/ofe.py run amber.ofc --simulator target/release/ofe-sim --script tools/ofe-sim/fixtures/duel.frames
```

复制 amber 目录，修改 attributes.json 的 max_health，或修改 moves/tap.json 的 damage，再用同一可执行文件直接运行。输出中的初始生命和命中伤害可用于核对改动。双方都使用传入的角色，输入 A 调用 manifest.moves 中的第一项。交互模式省略脚本参数。

```sh
target/release/ofe-sim --character tools/ofe-sim/fixtures/amber --script tools/ofe-sim/fixtures/duel.frames
```

manifest.json 列出角色信息、属性、动作、判定框、动画、资源、依赖声明和许可。文档中的相对路径从该文档所在目录解析。多段、取消目标和投射物参数由包根可选的 move_profile.json 给出，可参考内置 arena 目录。打包工具按稳定顺序收集引用到的文件。解包校验、运行结束后会清理临时目录。

OFA 支持 PNG 序列帧和按行索引的均分图集。duration 以模拟 tick 计。Player 第一次 tick 发出首帧的进入事件，连续展示同一帧时不会重复发出事件。非循环动画停在末帧，循环动画每次重新进入首帧都会发出相应事件。事件交给表现层。资源层可共享同一文件的字节，释放缓存后再次加载可读取修改后的内容。[运行时接口与归档限制](../architecture/ADR-006.md)
