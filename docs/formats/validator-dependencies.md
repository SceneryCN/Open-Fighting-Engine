# P0 格式工具依赖初审

以下为 Python 3.12.14 安装环境的包元数据与许可证文件审查。依赖用于开发与 CI 校验，游戏包无需携带。requirements.txt 锁定各版本及可接受分发件的 SHA-256。uv 0.12.14 用于生成锁文件，常规安装可使用 pip。

| 包及精确版本 | 用途 | 声明许可证 | 上游 |
|---|---|---|---|
| jsonschema 4.25.1 | Draft 2020-12 校验 | MIT | [仓库](https://github.com/python-jsonschema/jsonschema) |
| jsonschema-specifications 2025.9.1 | 注册标准元 Schema | MIT | [仓库](https://github.com/python-jsonschema/jsonschema-specifications) |
| referencing 0.37.0 | 本地 Schema 引用解析 | MIT | [仓库](https://github.com/python-jsonschema/referencing) |
| attrs 26.1.0 | 上游依赖的数据类支持 | MIT | [仓库](https://github.com/python-attrs/attrs) |
| rpds-py 2026.9.1 | 上游依赖的持久数据结构 | MIT | [仓库](https://github.com/crate-py/rpds) |
| typing-extensions 4.16.0 | Python <3.13 的类型支持 | PSF-2.0 | [仓库](https://github.com/python/typing_extensions) |

上述许可证均允许再分发。分发 MIT 包须保留版权与许可声明；PSF-2.0 须保留许可、版权与适用的修改摘要，并遵守商标限制。检查打包版本随附的 dist-info/licenses；当前仓库仅提供下载锁，未复制第三方 wheel。[MIT 许可条件](https://opensource.org/license/mit) · [Python 许可条件](https://docs.python.org/3/license.html)

fixtures 中的图像由本项目生成，角色数据、布局、舞台和主题为测试输入，授权记录位于角色 licenses/notice.txt。图像只有一个像素。
