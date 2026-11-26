# Open Fighting Engine（OFE）
# 项目总任务书 V1.0

**项目名称：** Open Fighting Engine  
**项目简称：** OFE  
**项目类型：** 开源 2D 双线格斗引擎及示范游戏  
**任务性质：** 完整工程建设任务  
**项目负责人：** 用户  
**技术负责人：** Codex 主控 Agent  
**开发模式：** 多 Agent 协作 + 独立测试 + 分阶段验收  
**当前启动阶段：** P0 Foundation & Technical Proof  
**项目状态：** NOT_STARTED

---

# 第一章：项目目标

## 1.1 项目定位

Open Fighting Engine（OFE）是一个独立、纯开源、可扩展的 2D 双线格斗游戏引擎。

项目不是简单制作一款格斗游戏，而是先开发一套可以独立运行、可以扩展、可以被第三方开发者使用的格斗引擎，再使用这套引擎开发一款完整的示范游戏。

核心目标：

1. 建立独立的格斗战斗模拟核心。
2. 建立双线战斗坐标与攻击判定系统。
3. 支持现代格斗游戏常见的帧数、取消、连招、防御、受击等机制。
4. 支持最多四名角色同时战斗。
5. 支持角色数据、招式数据和动画资源独立配置。
6. 支持战斗确定性、输入记录、状态快照和回放。
7. 提供 Godot 运行时集成。
8. 提供角色 SDK、动画 SDK 和项目格式。
9. 提供一款完整可玩的离线示范游戏。
10. 最终实现 Windows、Linux、macOS 三平台构建和发行。

OFE 必须是一个真正的引擎项目，而不是将某款游戏的代码包装成引擎。

## 1.2 首期冻结范围

以下内容属于项目 V1 首期已冻结决策。

| 项目 | 冻结方案 |
|---|---|
| 目标平台 | Windows / Linux / macOS Desktop |
| 游戏类型 | 2D 双线格斗 |
| 画面形式 | 纯 2D Sprite / 手绘风格 |
| 战斗模式 | 离线 |
| 项目性质 | 纯开源 |
| 开发顺序 | 先引擎，后示范游戏 |
| 战斗人数 | 最多四名角色 |
| 游戏框架 | Godot |
| 战斗核心 | Rust |
| Godot 桥接 | C++ GDExtension + Rust C ABI |
| 编辑器 | 首期不开发 |
| 联网对战 | 首期不开发 |
| MUGEN 兼容 | 首期不开发完整兼容运行时 |

未经项目负责人批准，不得擅自改变以上冻结决策。

## 1.3 首期明确不做的内容

首期禁止投入以下开发：

- Character Creator 人物编辑器。
- Game Studio 可视化游戏编辑器。
- 网络对战。
- Rollback 网络同步实现。
- 账号、匹配、服务器。
- 在线角色市场。
- 在线社区。
- AI 角色生成。
- 完整 MUGEN / Ikemen GO 兼容运行时。
- WASM 扩展系统。
- 移动端。
- 3D 战斗和 3D 渲染。

但架构必须为未来扩展保留合理接口。

特别注意：

**预留扩展接口不等于提前开发完整功能。**

---

# 第二章：产品与战斗设计

## 2.1 双线战斗系统

OFE 采用 X / Z / H 逻辑坐标体系。

| 坐标 | 含义 |
|---|---|
| X | 横向位置 |
| Z | 战斗纵深，决定前线与后线 |
| H | 跳跃高度 |

示例：

- 前线 Z = 0。
- 后线 Z = 1000。
- X 负责左右移动。
- Z 负责纵深移动。
- H 负责跳跃高度。

以上是内部逻辑单位示例，必须在项目中统一定义单位、精度和坐标方向。

画面投影：

```text
ScreenX = X
ScreenY = GroundY - α * Z - H
```

其中 α 为纵深投影系数，必须配置化。

注意：

逻辑上存在 X、Z、H 三个维度，不代表使用 3D 渲染。

首期必须采用：

- 2D 场景。
- 2D Sprite。
- 2D 舞台。
- 2D 特效。
- 固定镜头。
- 不跟随角色。
- 不动态缩放镜头。

## 2.2 战斗人数

引擎必须支持：

1. 1V1。
2. 2V2。
3. 四人自由混战。

每个战斗实体必须具有：

- 唯一 Entity ID。
- Team ID。
- 战斗状态。
- 位置。
- 生命值。
- 能量值。
- 当前动作。
- 当前攻击实例。
- 当前受击状态。

不得将战斗逻辑写死为两个角色。

## 2.3 双线攻击规则

攻击不能简单使用：

```rust
attacker.lane == defender.lane
```

作为唯一命中判断。

每个招式必须拥有独立的 Lane Rule。

首期定义：

```text
SAME_LANE
CROSS_LANE
BOTH_LANES
DEPTH_RANGE
```

规则说明：

- SAME_LANE：只攻击相同战斗线。
- CROSS_LANE：允许攻击不同战斗线。
- BOTH_LANES：攻击覆盖两条战斗线。
- DEPTH_RANGE：根据纵深距离判断是否命中。

未来允许扩展更多规则，但不得破坏现有格式兼容性。

## 2.4 切线机制

首期提供基础切线机制。

调试初始参数：

| 阶段 | 帧数 |
|---|---:|
| 起始动作 | 3 |
| 纵深移动 | 6 |
| 结束动作 | 3 |
| 冷却 | 20 |

以上仅为初始测试参数，不代表任何原作的精确数据。

切线必须具备：

- 当前状态。
- 切线方向。
- 起始阶段。
- 移动阶段。
- 结束阶段。
- 冷却时间。
- 是否允许被打断。
- 切线过程中的攻击判定规则。

这些参数必须配置化。

---

# 第三章：总体技术架构

## 3.1 技术选型

核心技术：

| 层级 | 技术 |
|---|---|
| 战斗核心 | Rust |
| 工程管理 | Cargo Workspace |
| 游戏框架 | Godot 4.7.x 稳定版本 |
| Godot 扩展 | C++ GDExtension |
| 跨语言接口 | Rust C ABI |
| 构建系统 | CMake + Cargo |
| 数据格式 | JSON + JSON Schema |
| 自动化测试 | Rust Test + cargo-nextest |
| 性能测试 | Criterion |
| 静态检查 | rustfmt + Clippy |
| CI | GitHub Actions |
| 版本控制 | Git |

Rust 工具链必须锁定经过验证的 Stable 版本，初始基线为 Rust 1.100.0。

必须使用：

```text
rust-toolchain.toml
Cargo.lock
```

禁止默认依赖 Nightly。

Godot 必须锁定稳定版本，不允许未经验证直接使用 Beta、RC 或开发版本。

具体版本和依赖必须记录在 ADR 和依赖清单中。

## 3.2 总体架构

必须采用以下分层：

```text
                  Demo Game
                      |
                      v
                 Game Runtime
                      |
          +-----------+-----------+
          |                       |
          v                       v
   Character Runtime       Animation Runtime
          |                       |
          +-----------+-----------+
                      |
                      v
                Combat Core
                      |
                      v
                Combat Events
                      |
                      v
                Snapshot / Replay


                  Godot
                    |
                    v
             Godot Integration
                    |
                    v
             C++ GDExtension
                    |
                    v
               Rust C ABI
                    |
                    v
               Rust Core
```

以上为逻辑依赖关系，实际模块依赖必须通过 ADR 和依赖图明确。

## 3.3 Combat Core

Combat Core 是唯一权威战斗模拟系统。

必须独立为 Rust Crate。

不得依赖：

- Godot 类型。
- Godot 场景树。
- Godot 物理系统。
- Godot AnimationPlayer。
- Godot CharacterBody2D。
- Godot 信号系统。

Combat Core 负责：

1. 固定帧模拟。
2. 角色状态机。
3. X / Z / H 坐标。
4. 移动与跳跃。
5. 切线。
6. 碰撞。
7. 攻击判定。
8. 防御判定。
9. 伤害结算。
10. 击退。
11. Hitstun。
12. Blockstun。
13. Hitstop。
14. 连招。
15. 取消窗口。
16. 投技。
17. 投射物。
18. 四人战斗。
19. 战斗事件。
20. 确定性模拟。
21. 状态快照。
22. 状态哈希。
23. 输入记录。
24. 回放。

Godot 只负责表现、输入采集和引擎接口调用。

**任何战斗规则只能存在一份权威实现。**

## 3.4 固定帧系统

战斗模拟必须固定为 60Hz。

必须实现：

- 固定帧推进接口。
- 渲染帧与模拟帧解耦。
- 输入帧编号。
- 帧计数。
- 帧状态。
- 帧事件。
- 模拟时间控制。
- 慢动作调试支持。

禁止直接使用渲染帧率作为战斗逻辑时间。

战斗逻辑不能因为不同电脑的 FPS 不同而产生不同结果。

## 3.5 状态机

至少定义以下状态：

```text
Idle
Walk
Dash
Jump
Fall
Attack
Guard
Hitstun
Blockstun
Knockdown
Getup
LaneSwitch
Grab
Super
KO
Win
Lose
```

状态必须具有明确的进入条件、退出条件和合法转换规则。

状态机必须可以通过测试验证。

## 3.6 攻击数据结构

每个招式必须支持：

```text
Startup
Active
Recovery
Damage
Hitstun
Blockstun
Hitstop
Pushback
Cancel Window
Invulnerability
Armor
Energy Cost
Lane Rule
```

还需要考虑：

- 攻击实例 ID。
- 多段攻击。
- 命中目标记录。
- 攻击优先级。
- 攻击属性。
- 防御属性。
- 投射物生命周期。
- 受击效果。
- 取消条件。
- 连招计数。

招式数据必须由 OFC 配置提供。

禁止把示范角色的招式硬编码到 Combat Core。

## 3.7 碰撞系统

必须区分：

- Hitbox：攻击判定框。
- Hurtbox：受击判定框。
- Pushbox：角色推挤框。

首期使用整数或定点数 AABB 碰撞。

禁止让浮点数误差破坏确定性。

四人战斗必须采用统一的帧结算机制：

1. 收集当前帧所有攻击判定。
2. 生成 Hit Event。
3. 统一计算命中关系。
4. 统一处理同帧伤害。
5. 统一应用状态变化。
6. 生成下一帧状态。

不能因为遍历角色的顺序不同，导致同一帧的战斗结果不同。

同一攻击实例默认对同一目标只命中一次。

多段攻击必须通过不同有效窗口或不同攻击实例实现。

---

# 第四章：确定性与回放

## 4.1 首期必须具备的能力

虽然首期完全离线，但必须从底层支持确定性。

必须设计：

- 固定帧模拟。
- 可复现 PRNG。
- 输入记录。
- 完整状态快照。
- 状态哈希。
- 快照恢复。
- 回放。
- 不一致帧定位。

## 4.2 确定性要求

相同初始状态、相同随机种子、相同输入序列，必须得到相同模拟结果。

测试必须覆盖：

1. 单角色移动。
2. 双角色攻击。
3. 连招。
4. 双线切换。
5. 四人战斗。
6. 同帧互殴。
7. 快照恢复。
8. 输入回放。

状态哈希必须排除不参与战斗逻辑的表现数据。

不得将 Godot 对象地址、随机内存布局、渲染数据纳入权威状态哈希。

## 4.3 Rollback 预留

首期不开发网络 Rollback。

但是快照和输入系统必须具有足够清晰的边界，使未来能够实现：

```text
Input History
      |
      v
Snapshot Restore
      |
      v
Fixed Frame Simulation
      |
      v
State Hash
```

---

# 第五章：数据格式规范

OFE 必须定义自己的原生数据标准。

所有格式都必须有明确版本字段、Schema 和兼容策略。

## 5.1 OFC：角色包格式

OFC 是 OFE 原生角色包。

采用：

- 开发时目录格式。
- 发布时 `.ofc` ZIP 格式。

至少包含：

```text
character/
  manifest.json
  character.json
  attributes.json
  moves/
  animations/
  hitboxes/
  sounds/
  assets/
  licenses/
```

必须支持：

- 角色信息。
- 作者。
- 许可证。
- 角色属性。
- 招式定义。
- 动画引用。
- 碰撞框。
- 音效。
- 资源清单。
- 依赖关系。
- 格式版本。

## 5.2 OFA：动画格式

OFA 是动画中间格式。

必须支持：

- PNG 序列帧。
- Sprite Sheet。
- 帧索引。
- 帧时长。
- 循环。
- 原点。
- 动画事件。
- 资源引用。
- 缺失资源处理。

动画表现不得决定战斗核心状态。

## 5.3 OGP：游戏项目格式

OGP 表示完整游戏项目。

必须包含：

- 游戏信息。
- 角色列表。
- 舞台。
- UI 资源。
- 音频。
- 字体。
- 主题。
- 游戏模式。
- 项目设置。
- 依赖资源。

## 5.4 OFL：界面布局格式

OFL 负责描述游戏 UI 布局。

至少定义：

- 主菜单。
- 角色选择。
- VS 页面。
- 战斗 HUD。
- 暂停页面。
- 胜负页面。

首期只开发 Schema 和运行时接口，不开发可视化编辑器。

## 5.5 Schema 规范

统一采用 JSON Schema Draft 2020-12。

必须实现：

- Schema 校验。
- 格式版本检查。
- 必填字段检查。
- 类型检查。
- 枚举检查。
- 资源引用检查。
- 错误定位。
- 兼容性检查。

必须提供合法和非法样例。

不能只判断 JSON 能否解析。

## 5.6 CLI 工具

计划提供：

```bash
ofe validate
ofe inspect
ofe package
ofe replay
```

各命令职责：

- validate：校验 OFC、OFA、OGP、OFL。
- inspect：查看资源和项目元信息。
- package：打包角色或游戏项目。
- replay：读取和检查战斗回放。

CLI 必须具有明确错误码和可读错误信息。

---

# 第六章：运行时架构

## 6.1 Character Runtime

负责：

- OFC 解析。
- 角色属性加载。
- 招式数据加载。
- 角色实例创建。
- 角色资源引用。
- 角色数据校验。

## 6.2 Animation Runtime

负责：

- OFA 解析。
- 动画帧管理。
- Sprite Sheet 管理。
- 动画播放状态。
- 动画循环。
- 动画事件。
- 资源引用。

动画系统只能表现战斗状态，不得反向决定权威战斗状态。

## 6.3 Resource Runtime

负责：

- 资源加载。
- 资源索引。
- 缓存。
- 释放。
- 依赖管理。
- 缺失资源处理。
- 资源路径安全检查。

## 6.4 Godot Integration

负责：

- GDExtension。
- Rust API 桥接。
- 生命周期管理。
- 角色渲染。
- 动画显示。
- 舞台显示。
- HUD。
- 音频。
- 输入采集。
- 场景管理。

## 6.5 Game Runtime

负责：

- 游戏模式。
- 回合系统。
- 队伍管理。
- 胜负判定。
- 角色选择。
- 暂停。
- 游戏设置。

Game Runtime 不得重复实现 Combat Core 的攻击、伤害和碰撞规则。

---

# 第七章：Rust 与 Godot 桥接规范

## 7.1 桥接方案

采用：

```text
Godot
  |
  v
C++ GDExtension
  |
  v
Versioned C ABI
  |
  v
Rust Combat Core
```

C++ GDExtension 负责：

- Godot 类型适配。
- Godot 生命周期。
- 调用 Rust API。
- 输入转换。
- 状态转换。
- Godot 侧资源管理。

Rust 负责：

- 战斗模拟。
- 战斗状态。
- 战斗事件。
- 快照。
- 确定性。

## 7.2 C ABI 规范

禁止直接暴露 Rust 内部结构体的原始内存布局。

必须使用：

- 不透明句柄。
- 版本化 C 结构体。
- 显式字段类型。
- 显式缓冲区管理。
- 明确的内存所有权。
- 明确的释放函数。
- 明确的错误码。

必须考虑：

- ABI 版本。
- 结构体大小。
- 空指针。
- 越界。
- 无效句柄。
- 重复释放。
- 创建失败。
- 调用失败。
- 跨线程调用约束。

不得让 C++ 和 Rust 同时拥有不明确的内存释放责任。

## 7.3 P0 桥接 PoC 验收

必须完成真实可运行的 PoC：

1. Godot 能够加载 GDExtension。
2. Godot 能够调用 Rust 创建战斗实例。
3. Rust 返回结构化状态。
4. Godot 能够提交输入。
5. Rust 能够推进至少一帧。
6. Godot 能够读取推进后的状态。
7. Godot 能够安全销毁实例。
8. 错误处理可验证。
9. 重复创建和销毁没有已知内存泄漏。
10. 有自动化测试或可重复验证脚本。

不能用伪代码、模拟返回值或纯文档代替 PoC。

---

# 第八章：示范游戏

## 8.1 产品目标

完成一款真正可玩的离线 2D 双线格斗游戏。

该游戏必须通过 OFE 引擎公开 API 运行。

不能为了赶进度，把示范游戏的战斗逻辑写进引擎核心。

## 8.2 游戏功能

至少包括：

- 主菜单。
- 角色选择。
- VS 页面。
- 战斗 HUD。
- 暂停。
- 胜负页面。
- 训练模式。
- 音效。
- 背景音乐。
- 游戏设置。

## 8.3 示例角色

角色 A：平衡型。

至少拥有：

- 普通攻击。
- 冲刺。
- 升龙类技能。
- 超必杀。

角色 B：速度型或远程型。

必须具有不同的：

- 攻击距离。
- 攻击节奏。
- 连招路线。
- 特殊技能。
- 超必杀。

角色的所有招式必须通过 OFC 配置。

## 8.4 训练模式

训练场必须显示：

- 生命值。
- 能量。
- 当前帧。
- 当前状态。
- 当前招式。
- Startup。
- Active。
- Recovery。
- Hitstun。
- Blockstun。
- 连招数。
- 伤害。
- 坐标。

必须提供：

- Hitbox 显示。
- Hurtbox 显示。
- Pushbox 显示。
- 假人站立。
- 假人防御。
- 假人受击。
- 无限生命。
- 重置位置。
- 慢动作。
- 单帧推进。
- 输入记录。
- 回放。

## 8.5 游戏模式

必须支持：

- 训练模式。
- 1V1。
- 2V2。
- 四人自由混战。

全部离线。

---

# 第九章：项目目录

推荐采用 Monorepo。

```text
open-fighting-engine/
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  README.md
  LICENSE

  engine/
    combat-core/
    combat-events/
    combat-input/
    combat-snapshot/
    character-runtime/
    animation-runtime/
    resource-runtime/

  integrations/
    godot/
      extension/
      bridge/
      runtime/

  formats/
    ofc/
    ofa/
    ofl/
    ogp/

  sdk/
    character-sdk/
    animation-sdk/
    game-sdk/

  game/
    demo-game/
      project.godot
      scenes/
      scripts/
      assets/
      characters/
      stages/
      ui/

  tools/
    ofe-cli/
    ofe-validator/
    ofe-package/
    ofe-replay/

  tests/
    determinism/
    combat/
    integration/
    fixtures/

  docs/
    architecture/
    combat/
    formats/
    sdk/
    development/
    testing/
    release/

  samples/
    characters/
    animations/
    stages/
    projects/

  scripts/
    build/
    test/
    release/

  .github/
    workflows/
```

目录允许在 P0 阶段通过 ADR 进行合理调整，但必须保证：

- 模块职责清晰。
- 引擎和游戏分离。
- Rust Core 和 Godot 分离。
- 格式规范独立。
- 测试独立。
- SDK 独立。
- 构建流程统一。

---

# 第十章：开发阶段

项目分为 P0 至 P8。

每个阶段必须独立验收。

## P0：Foundation & Technical Proof

目标：建立工程基础，验证架构和技术路线。

任务：

1. 架构设计。
2. Monorepo 初始化。
3. Rust 工具链固定。
4. Godot 工程初始化。
5. C++ GDExtension 初始化。
6. Rust C ABI。
7. Rust/Godot 桥接 PoC。
8. OFC Schema。
9. OFA Schema。
10. OGP Schema。
11. OFL Schema。
12. Combat Core 骨架。
13. Entity ID。
14. 固定帧接口。
15. 坐标类型。
16. 输入结构。
17. 事件结构。
18. Snapshot 接口草案。
19. 测试框架。
20. 跨平台 CI。
21. 开源许可证初审。
22. P0 交付报告。

P0 不开发完整战斗系统。

P0 不开发完整 Demo Game。

P0 不开发编辑器。

**P0 未经项目负责人验收，不得进入 P1。**

## P1：Combat Core MVP

目标：完成最小可玩的双线战斗核心。

必须支持：

- 固定帧。
- 两名角色。
- X/Z/H 坐标。
- 移动。
- 跳跃。
- 切线。
- 普通攻击。
- Hitbox / Hurtbox。
- 伤害。
- 防御。
- 受击。
- 击退。
- 基础状态机。
- 基础战斗事件。

交付：

- 可独立运行的 Rust Combat Core。
- 自动化测试。
- 基础战斗模拟程序。
- 接口文档。
- 性能基准。

## P2：Advanced Combat

目标：完成现代格斗机制。

支持：

- Dash。
- Hitstun。
- Blockstun。
- Hitstop。
- Cancel Window。
- 连招。
- 多段攻击。
- 投技。
- 投射物。
- Armor。
- Invulnerability。
- Super。
- 2V2。
- 四人自由混战。
- 同帧统一结算。

交付完整战斗测试矩阵。

## P3：Character & Animation Runtime

目标：让角色和动画数据真正脱离硬编码。

支持：

- OFC 加载。
- OFA 加载。
- 角色属性解析。
- 招式数据解析。
- 动画帧。
- Sprite Sheet。
- 动画事件。
- 资源管理。
- 角色资源校验。
- 角色打包。

## P4：Godot Runtime

目标：实现完整的 Godot 运行时集成。

支持：

- GDExtension。
- Rust 战斗实例管理。
- 输入采集。
- 战斗状态读取。
- 角色渲染。
- 动画显示。
- 舞台。
- HUD。
- 音频。
- 游戏场景管理。

## P5：Demo Game

目标：完成可玩的示范游戏。

至少两名角色、一张原创舞台，具备角色选择、VS、战斗、训练、暂停、胜负和音频系统。

## P6：Determinism & Replay

目标：完成确定性验证与回放。

支持：

- 输入记录。
- 可复现 PRNG。
- 快照。
- 恢复。
- 状态哈希。
- 回放。
- 不一致帧定位。
- 自动化回归测试。

## P7：SDK & Mod Foundation

目标：建立第三方扩展基础。

支持：

- 角色 SDK。
- 动画 SDK。
- 数据格式文档。
- 角色示例。
- Mod 基础加载。
- CLI 工具。
- 示例项目。

不开发可视化编辑器。

## P8：Cross-platform Release

目标：正式跨平台发行。

必须支持：

- Windows x64。
- Linux x64。
- macOS ARM64。
- macOS x64。

完成：

- Release 构建。
- Godot 导出。
- 自动化测试。
- 依赖许可证检查。
- 发布文档。
- 安装与运行指南。
- 示例游戏发行包。

---

# 第十一章：P0 专项执行任务

P0 是整个项目最重要的架构验证阶段。

必须拆成以下工作包。

## P0-01：Architecture

负责人：Architecture Agent。

交付：

- 总体架构图。
- 模块依赖图。
- 数据流图。
- 战斗帧流程图。
- Rust/Godot 桥接设计。
- C ABI 规范。
- 坐标系统规范。
- 数据格式规范。
- ADR。
- 风险清单。

验收重点：

- Combat Core 不依赖 Godot。
- 战斗逻辑唯一。
- 模块边界清楚。
- 接口具有版本管理。
- 技术选型有明确依据。

## P0-02：Monorepo

负责人：Build & DevOps Agent。

交付：

- Cargo Workspace。
- rust-toolchain.toml。
- Cargo.lock。
- Godot 项目。
- CMake。
- godot-cpp。
- 统一目录。
- Git 规范。
- 基础 CI。

## P0-03：Rust/Godot Bridge PoC

负责人：Godot Integration Agent。

交付：

- C++ GDExtension。
- Rust C ABI。
- 实例创建。
- 实例销毁。
- 输入提交。
- 固定帧推进。
- 结构化状态返回。
- 错误处理。
- 内存管理测试。
- 最小可运行 Godot Demo。

这是 P0 的核心门槛。

## P0-04：Data Schema

负责人：Architecture Agent + Animation & Asset Agent。

交付：

- OFC Schema。
- OFA Schema。
- OGP Schema。
- OFL Schema。
- 合法数据样例。
- 非法数据样例。
- 版本字段。
- Schema 校验工具。

## P0-05：Combat Core Skeleton

负责人：Combat Core Agent。

只完成接口和基础骨架：

- Battle Instance。
- Entity ID。
- 固定帧接口。
- 坐标类型。
- 输入结构。
- 事件结构。
- Snapshot 接口草案。
- 基础错误类型。

不得在 P0 阶段直接扩展成完整格斗系统。

## P0-06：Testing Framework

负责人：QA Agent。

交付：

- Rust 单元测试框架。
- 集成测试框架。
- 桥接测试。
- Godot Headless 检查。
- 状态哈希测试结构。
- 回归测试结构。
- 测试报告格式。

## P0-07：Cross-platform Build

负责人：Build & DevOps Agent。

必须建立：

- Windows x64 构建。
- Linux x64 构建。
- macOS ARM64 构建。
- macOS x64 构建。
- Godot Headless 验证。
- Godot 导出验证方案。
- GitHub Actions。

不能仅仅配置 YAML 文件。

必须有实际构建结果。

## P0-08：Documentation & Delivery

负责人：Documentation Agent + Tech Lead。

交付：

- README。
- 架构文档。
- 环境配置指南。
- Agent 职责说明。
- 依赖清单。
- 许可证初审。
- P0 风险清单。
- P0 验收报告。
- DELIVERY_REPORT.md。

---

# 第十二章：Codex 多 Agent 协作组织

Codex 主控 Agent 必须承担 Tech Lead 职责。

不得将所有任务混在一个 Agent 中无序执行。

建议组织如下：

## Agent 00：Tech Lead

唯一主控。

职责：

- 总体架构决策。
- 任务拆分。
- 公共接口冻结。
- ADR 审核。
- 分支管理。
- Agent 调度。
- 冲突解决。
- PR 审查。
- 集成。
- 里程碑管理。
- 最终验收。

主控不得只负责分发任务。

必须对最终交付负责。

## Agent 01：Architecture

职责：

- 架构设计。
- 模块依赖。
- ADR。
- C ABI。
- 数据格式。
- 版本策略。
- 技术风险。

## Agent 02：Combat Core

职责：

- Rust 战斗核心。
- 坐标。
- 固定帧。
- 状态机。
- 碰撞。
- 攻击。
- 伤害。
- 双线规则。
- 四人战斗。
- 确定性。

## Agent 03：Animation & Asset

职责：

- OFA。
- 动画运行时。
- Sprite Sheet。
- PNG 序列帧。
- 动画事件。
- 资源加载。
- 资源管理。

## Agent 04：Godot Integration

职责：

- GDExtension。
- C++ 桥接。
- Rust C ABI 集成。
- Godot 场景。
- 角色渲染。
- 动画表现。
- 音频。
- HUD。

## Agent 05：Game Logic

职责：

- 游戏模式。
- 角色选择。
- 回合。
- 队伍。
- 胜负。
- Demo Game。

## Agent 06：QA

独立质量负责人。

职责：

- 测试方案。
- 测试用例。
- 自动化测试。
- 回归测试。
- 独立审查。
- 验收报告。

QA 有权拒绝不合格交付。

QA 不得只复述开发 Agent 的测试结果。

## Agent 07：Build & DevOps

职责：

- Cargo Workspace。
- CMake。
- Godot 构建。
- GitHub Actions。
- 多平台构建。
- 依赖管理。
- 发布流程。

## Agent 08：Documentation

职责：

- README。
- 架构文档。
- API 文档。
- 数据格式文档。
- SDK 文档。
- 开发指南。
- 构建指南。
- 发布文档。

## 12.1 Agent 协作规则

必须遵守：

1. Tech Lead 是唯一架构决策者。
2. 每个 Agent 有固定模块职责。
3. 不允许擅自修改公共数据结构。
4. 不允许重复实现战斗逻辑。
5. 公共 API 修改必须经过 ADR。
6. 每个 Agent 必须提供测试。
7. 所有代码通过统一 CI。
8. 主控负责最终集成。
9. QA 独立复核。
10. Agent 数量不能超过可管理的并行任务数量。
11. 不允许多个 Agent 同时修改相同核心文件。
12. 使用独立 Git 分支或 Worktree。

如果 Codex 当前环境不支持真正的子 Agent 或独立 Worktree，必须如实说明，并采用可行的串行任务调度方案，不得伪造 Agent 执行记录。

---

# 第十三章：Git 与代码规范

## 13.1 分支策略

建议：

```text
main
  |
  +-- feature/architecture
  +-- feature/combat-core
  +-- feature/godot-bridge
  +-- feature/animation-runtime
  +-- feature/build-ci
  +-- feature/documentation
```

禁止多个 Agent 直接在 main 上同时开发。

## 13.2 提交规范

采用 Conventional Commits。

例如：

```text
feat(combat): add fixed frame simulation
fix(bridge): handle invalid battle handle
test(combat): add lane collision tests
docs(architecture): add combat core dependency graph
build(ci): add macos arm64 pipeline
```

## 13.3 代码质量

Rust 必须通过：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

适用的集成测试必须执行 cargo-nextest。

C++ 必须配置格式检查和编译警告。

JSON 必须通过 Schema 校验。

所有公共 API 必须具有文档。

禁止：

- 大量 TODO 代替实现。
- 空函数伪装完成。
- 硬编码招式数据。
- 忽略错误。
- 无边界的 unsafe。
- 未说明的内存所有权。
- 未测试的公共接口。
- 仅为通过测试而绕过真实功能。

允许必要的 TODO，但必须标注责任人、原因和计划阶段。

---

# 第十四章：性能与质量标准

所有性能目标必须通过基准测试验证。

## 14.1 战斗性能

首期以最多四名战斗角色作为基本规模。

必须测试：

- 每帧模拟耗时。
- 碰撞检测耗时。
- 攻击判定耗时。
- 状态更新耗时。
- 事件生成耗时。
- Snapshot 耗时。
- 回放耗时。

固定帧目标为 60Hz。

模拟逻辑必须尽可能控制在每帧 16.67ms 预算以内。

但不能为了达到目标而牺牲确定性和正确性。

## 14.2 内存与资源

必须检查：

- 战斗实例创建和销毁。
- 资源重复加载。
- 资源缓存。
- 资源释放。
- 快照内存占用。
- 长时间训练模式运行。

不能仅凭一次运行宣称没有内存泄漏。

## 14.3 测试要求

每个核心模块必须有：

- 单元测试。
- 边界测试。
- 错误测试。
- 回归测试。

核心战斗必须有：

- 固定帧测试。
- 坐标测试。
- 碰撞测试。
- 双线测试。
- 同帧结算测试。
- 四人战斗测试。
- 确定性测试。

桥接必须有：

- 创建测试。
- 销毁测试。
- 重复创建销毁测试。
- 无效句柄测试。
- 空指针测试。
- 错误返回测试。

---

# 第十五章：开源许可证与知识产权

OFE 必须是纯开源项目。

初步许可证建议：

| 模块 | 初步许可证 |
|---|---|
| Combat Core | MPL-2.0 |
| Rust SDK | MIT |
| Godot Integration | MPL-2.0 |
| OFC/OFA/OGP/OFL 格式规范 | CC0-1.0 |
| 示例角色代码 | MIT |
| 示例素材 | CC0 或 CC-BY，前提是授权明确 |

以上为建议方案，P0 必须完成许可证兼容性审查并最终冻结。

每个依赖必须记录：

- 名称。
- 版本。
- 来源。
- 许可证。
- 使用模块。
- 是否允许再分发。
- 是否存在额外限制。

禁止：

- 未经许可复制第三方代码。
- 未经授权使用游戏素材。
- 将开源误认为可以随意再分发。
- 将第三方角色素材直接打包进官方发行版而不审查许可证。

未来 MUGEN 兼容必须独立设计。

OFC 是 OFE 原生角色的唯一标准。

未来可建立独立 MUGEN 兼容运行时，但不得让 MUGEN 语义污染 Combat Core。

---

# 第十六章：项目管理与验收机制

## 16.1 任务状态

所有任务使用以下状态：

```text
NOT_STARTED
IN_PROGRESS
BLOCKED
IMPLEMENTED
TESTING
ACCEPTED
```

含义：

- NOT_STARTED：未开始。
- IN_PROGRESS：开发中。
- BLOCKED：被阻塞。
- IMPLEMENTED：代码已实现。
- TESTING：测试中。
- ACCEPTED：已通过独立验收。

**IMPLEMENTED 不代表 ACCEPTED。**

## 16.2 每个阶段的强制交付物

每个阶段都必须提供：

1. 实施方案。
2. 完整源代码。
3. 自动化测试。
4. 测试结果。
5. 构建结果。
6. 性能报告。
7. 技术文档。
8. 已知问题。
9. 风险清单。
10. DELIVERY_REPORT.md。

缺少关键交付物，不得标记为 ACCEPTED。

## 16.3 验收权限

开发 Agent 不能自行宣布自己完成验收。

QA 必须独立测试。

Tech Lead 必须完成集成审查。

最终由项目负责人确认是否通过。

## 16.4 进度汇报

每个工作日或每个重要里程碑必须更新：

```text
PROJECT_STATUS.md
```

至少记录：

- 当前阶段。
- 当前任务。
- 负责人。
- 任务状态。
- 已完成内容。
- 未完成内容。
- 阻塞原因。
- 风险。
- 下一步。

禁止只用代码行数、文件数量或 Agent 完成数量作为进度指标。

项目进度以通过验收的真实功能为准。

---

# 第十七章：P0 验收清单

P0 必须逐项验收。

## A. 架构验收

- [ ] Combat Core 不依赖 Godot 类型。
- [ ] 战斗规则只有一套权威实现。
- [ ] Rust Core 和 Godot 表现层职责清晰。
- [ ] 核心模块依赖关系明确。
- [ ] 技术选型具有 ADR。
- [ ] 坐标系统有明确规范。
- [ ] 固定帧模拟接口已定义。
- [ ] C ABI 具有版本管理。
- [ ] 内存所有权和生命周期有明确说明。

## B. 桥接验收

- [ ] Godot 能加载 GDExtension。
- [ ] Godot 能调用 Rust 创建实例。
- [ ] Rust 能返回结构化状态。
- [ ] Godot 能提交输入。
- [ ] Rust 能推进至少一帧。
- [ ] Godot 能读取更新后的状态。
- [ ] Godot 能安全销毁实例。
- [ ] 错误处理可验证。
- [ ] 无效句柄有保护。
- [ ] 重复创建和销毁测试通过。
- [ ] 没有已知内存泄漏问题。

## C. 数据格式验收

- [ ] OFC Schema 已建立。
- [ ] OFC 合法数据校验通过。
- [ ] OFC 非法数据能定位错误。
- [ ] OFA Schema 已建立。
- [ ] OFA 动画帧和资源引用结构明确。
- [ ] OGP 基本结构已建立。
- [ ] OFL Schema 已建立。
- [ ] 所有格式具有版本字段。
- [ ] 提供合法样例。
- [ ] 提供非法样例。

## D. Combat Core 骨架验收

- [ ] Battle Instance 已定义。
- [ ] Entity ID 已定义。
- [ ] 固定帧接口已定义。
- [ ] 坐标类型已定义。
- [ ] 输入结构已定义。
- [ ] 事件结构已定义。
- [ ] Snapshot 接口草案已定义。
- [ ] 基础错误类型已定义。
- [ ] 有基础自动化测试。

## E. 构建与测试验收

- [ ] Rust 单元测试通过。
- [ ] 桥接集成测试通过。
- [ ] Godot Headless 检查通过。
- [ ] Windows x64 构建成功。
- [ ] Linux x64 构建成功。
- [ ] macOS ARM64 构建成功。
- [ ] macOS x64 构建成功。
- [ ] CI 自动执行。
- [ ] 构建结果有记录。
- [ ] 失败原因有记录。

## F. 开源交付验收

- [ ] 依赖许可证清单已建立。
- [ ] 官方素材授权策略已建立。
- [ ] README 已完成。
- [ ] 架构文档已完成。
- [ ] 开发环境指南已完成。
- [ ] P0 风险清单已完成。
- [ ] DELIVERY_REPORT.md 已完成。
- [ ] QA 独立验收报告已完成。

## 17.1 P0 一票否决项

以下任意一项不通过，P0 就不能通过：

1. Godot 无法调用 Rust。
2. Rust Core 依赖 Godot 类型。
3. Godot 和 Rust 存在两套战斗判定逻辑。
4. 没有固定帧模拟接口。
5. 没有数据 Schema。
6. 没有自动化测试。
7. 没有可执行的跨平台构建方案。
8. C ABI 内存所有权不明确。
9. 核心工程无法在干净环境中复现构建。

P0 未通过，不得进入 P1。

---

# 第十八章：Codex 启动执行指令

收到本任务书后，立即启动项目。

严格按照以下顺序执行：

## 第一步：环境检查

检查当前工作环境：

- 操作系统。
- CPU 架构。
- Rust 版本。
- Cargo。
- CMake。
- C++ 编译器。
- Godot。
- godot-cpp。
- Git。
- 可用构建工具。
- 当前仓库状态。
- 当前目录结构。

如果已经存在项目仓库，先检查现状。

禁止未经检查直接覆盖现有工程。

如果存在用户已有代码，必须先识别并保护。

## 第二步：项目初始化

建立 Monorepo。

建立 Cargo Workspace。

锁定 Rust 工具链。

建立 Godot 工程。

建立 C++ GDExtension 工程。

建立基础 CI。

建立文档目录。

建立任务状态文件。

## 第三步：架构冻结

先完成：

- 总体架构。
- 模块依赖图。
- 数据流图。
- 战斗帧流程图。
- C ABI 设计。
- 坐标系统。
- 数据格式。
- ADR。

在公共接口冻结之前，不允许多个 Agent 并行开发相互依赖的核心接口。

## 第四步：启动子 Agent

按以下顺序：

第一批：

- Tech Lead。
- Architecture。

第二批：

- Godot Integration：桥接 PoC。
- Build & DevOps：构建与 CI。
- Combat Core：基础骨架。
- QA：测试框架。

第三批：

- QA 独立验证。
- Documentation 整理交付物。
- Tech Lead 集成。

每个 Agent 必须有明确输入、输出和验收标准。

## 第五步：完成 P0

必须完成全部 P0-01 至 P0-08。

必须在真实环境中构建、运行、测试。

不得用静态代码检查代替运行验证。

不得用文档代替真实 PoC。

不得用模拟测试结果代替真实测试结果。

## 第六步：P0 独立验收

QA 必须根据第十七章逐项验收。

对未通过的项目：

- 标记 BLOCKED 或 TESTING。
- 记录具体失败原因。
- 给出修复方案。
- 修复后重新测试。

## 第七步：提交交付报告

最终必须提交：

```text
docs/
  architecture/
  development/
  testing/
  release/

PROJECT_STATUS.md
DELIVERY_REPORT.md
P0_ACCEPTANCE_REPORT.md
DEPENDENCIES.md
LICENSE_AUDIT.md
```

报告必须区分：

- 已实现。
- 已测试。
- 已通过。
- 未完成。
- 存在风险。
- 被阻塞。
- 需要项目负责人决策。

禁止将未验证的能力描述为已经完成。

---

# 第十九章：项目负责人决策机制

当遇到以下问题时，Codex 不得擅自做出重大改变：

- 改变技术栈。
- 改变战斗坐标体系。
- 改变核心模块边界。
- 改变 Rust/Godot 桥接方案。
- 改变数据格式。
- 改变许可证。
- 改变首期功能范围。
- 引入大型第三方依赖。
- 放弃某个平台。
- 延后核心验收标准。

必须向项目负责人提交决策报告。

格式：

```text
决策事项：

问题背景：

推荐方案：

备选方案：

不推荐方案及原因：

技术影响：

开发成本：

维护成本：

风险：

是否影响现有架构：

是否阻塞当前阶段：

需要项目负责人确认的内容：
```

普通实现细节由 Tech Lead 自主决策，不要把所有技术细节都推给项目负责人。

---

# 第二十章：最终交付标准

OFE 最终不能只是一个 Rust 库，也不能只是一个 Godot 游戏。

最终必须交付：

1. 独立 Rust Combat Core。
2. 完整双线格斗模拟。
3. 四人战斗支持。
4. 角色运行时。
5. 动画运行时。
6. 资源管理系统。
7. Godot GDExtension。
8. Rust/Godot 稳定桥接接口。
9. OFC/OFA/OGP/OFL 格式。
10. 角色 SDK。
11. 动画 SDK。
12. CLI 工具。
13. 确定性系统。
14. 输入记录和回放。
15. 完整示范游戏。
16. 训练模式。
17. 三平台构建。
18. 自动化测试。
19. 完整技术文档。
20. 开源发行材料。

最终交付必须满足：

- 可以在干净环境构建。
- 可以实际运行。
- 可以进行双线战斗。
- 可以进行四人战斗。
- 可以通过 OFC 加载角色。
- 可以通过 OFA 加载动画。
- 可以运行示范游戏。
- 可以记录和回放战斗。
- 可以进行自动化测试。
- 可以在目标平台构建。
- 第三方开发者能够根据文档接入引擎。

---

# 最后：对 Codex 的最高优先级要求

你不是本项目的代码生成器。

你是本项目的技术负责人、工程执行者和最终交付负责人。

你必须主动发现架构问题、技术风险、接口冲突和不合理需求。

不要为了迎合项目负责人而默认所有设计都正确。

如果发现设计存在问题，必须提出专业意见，并给出更合理的方案。

但未经批准，不得擅自改变已冻结的项目范围和核心技术决策。

执行过程中必须遵循以下原则：

**第一，架构正确优先于开发速度。**

**第二，真实功能优先于代码数量。**

**第三，可测试性优先于表面完成度。**

**第四，模块边界优先于临时便利。**

**第五，独立验收优先于开发者自我宣布完成。**

**第六，项目负责人拥有最终产品决策权。**

现在正式启动 Open Fighting Engine。

**当前只允许执行 P0：Foundation & Technical Proof。**

先检查环境、检查仓库、建立架构、冻结接口，再开始实施。

在 P0 验收通过之前，不得擅自进入 P1，不得提前开发完整战斗核心、示范游戏、编辑器、网络系统或 MUGEN 兼容运行时。

最终目标不是生成一个看起来完整的项目，而是交付一个真正可运行、可验证、可扩展、可跨平台发行的开源 2D 双线格斗引擎。

**项目负责人最终验收通过，才代表阶段真正完成。**