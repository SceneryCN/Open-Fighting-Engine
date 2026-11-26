# 启动 P4 双人对战

从 GitHub Actions 下载适合系统的 ofe-runtime 归档。解开 tar.gz，保留同目录的动态库、数据文件及所有许可声明，启动 OFE。角色和贴图随包提供。[四目标发布验证](https://github.com/SceneryCN/Open-Fighting-Engine/actions/runs/45802105150)

从源码运行需 Godot 4.7.2、固定版 Rust 工具链和支持 C++17 的 CMake 编译环境。在仓库根目录执行以下命令。Windows 的多配置构建同样使用 Release。

```sh
cmake -S . -B build/native -DCMAKE_BUILD_TYPE=Release
cmake --build build/native --config Release --parallel 4
ctest --test-dir build/native -C Release --output-on-failure
cmake --install build/native --config Release --prefix integrations/godot/runtime
godot --headless --path integrations/godot/runtime --editor --import
godot --path integrations/godot/runtime res://match.tscn
```

以下键位适用于当前源码的双人对战场景；历史 P4 包可查看画面底部的当版操作提示。

|动作|Amber · P1|Indigo · P2|
|---|---|---|
|左右移动|A / D|← / →|
|跳跃|W|↑|
|切换战斗线|S|↓|
|普攻|J 或 O|F|
|格挡|K|G|
|冲刺|L|H|

R 关闭当前对局并重新创建实例。双方的位置、朝向、生命、能量、状态及命中事件来自 Rust。Godot 根据 OFA 帧裁剪图集并按原点显示；命中和格挡分别触发不同音效。两条地面标线对应近线和远线。

## 替换角色

运行时允许分别指定两份本地 OFC 目录。任意一方省略参数会使用对应的内置角色。目录需要符合 P3 的对战绑定要求，包含一个攻击矩形和一个受击矩形。启动前可以使用 P3 作者工具校验目录。命令中两条目录应替换成实际路径。

```sh
godot --path integrations/godot/runtime res://match.tscn -- --p1-dir="/absolute/path/first" --p2-dir="/absolute/path/second"
```

内置 amber.ofc、indigo.ofc 是标准 ZIP 角色包，载入时解到独立临时目录，离树后清理。文档路径按 P3 约定解析；运行时把原始 PNG 编码字节从 Rust 传给 Godot，每个实例按图像 ID 缓存已解码纹理。对战状态名和 OFA ID 忽略大小写匹配，缺省使用 idle 或首个片段。JSON 快照、输入位和调用方缓冲区约定见 [ADR-007](../architecture/ADR-007.md)。

## 内置素材来源

场景的几何舞台由 GDScript 绘制，命中与格挡使用程序生成的短波形。角色数据派生自仓库的 Amber 与 Indigo CC0 夹具。两个角色包内含各自的图集、动画和许可声明。图集通过内置 ImageGen 为本工程生成，按格切分和缩放后封装。素材按各包内的 CC0 声明提供。

生成提示词：

> A precisely gridded 2 by 2 sprite atlas for a side-view 2D fighting game, transparent background, no lettering, no labels, no shadows, no weapons. Four equally sized square cells on a square canvas. Both characters face RIGHT in every cell. Upper row one original copper-orange jacket boxer with black hair and dark trousers, left cell compact idle boxing stance, right cell same fighter reaching forward for a straight punch. Lower row one original blue-indigo jacket boxer with white cropped hair and dark trousers, left cell idle boxing stance, right cell same fighter reaching for straight punch. Clean bold retro arcade pixel art silhouettes with bright cream highlights and dark ink outlines. Entire bodies including feet within each cell with generous transparent padding, aligned feet near bottom of each cell, identical scale.
