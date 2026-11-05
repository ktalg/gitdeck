# gitdeck

一个终端里的 Git 客户端，布局沿用 JetBrains Git 工具窗。

![changes](assets/screenshots/changes.png)

三栏：左边变更，中间历史，右边提交详情。键盘为主，鼠标为辅。读写都走 `git` 命令，
仓库行为与你在命令行里看到的完全一致。

[English](README.md) · **简体中文**

## 功能

- 按文件暂存，或在 diff 里按 hunk 暂存，只提交勾选的内容
- 提交图，分支、tag、合并线
- 单文件 diff，词级高亮与软折行
- 分支列表：切换、新建、删除、合并
- Push、Fetch，Pull 支持 fast-forward / merge / rebase 三种策略
- 页脚显示 merge 与 rebase 状态，可中止或继续
- 冲突处理：采用我方、采用对方、标记已解决
- 丢弃文件改动，忽略未跟踪文件

![log](assets/screenshots/log.png)

## 安装

```sh
cargo install --path crates/gitdeck-cli --locked
```

或者从源码构建：

```sh
cargo build --release
./target/release/gitdeck ~/some/repo
```

需要 `PATH` 上有 `git`，终端不小于 80×24。

```sh
gitdeck --doctor                      # 能力档位、主题、终端尺寸
gitdeck --mouse-debug /tmp/mouse.log  # 记录原始鼠标事件，排查点击问题
```

## 按键

| 按键 | 作用 |
|---|---|
| `tab` / `shift+tab` | 在 Changes、Log、Details 之间切换焦点 |
| `j` `k` `↑` `↓` | 移动选中项，或滚动已打开的 diff |
| `space` | 切换复选框，或光标所在的 hunk |
| `enter` | 打开文件 diff，或切换到选中分支 |
| `i` | 编辑提交消息 |
| `ctrl+enter` | 提交勾选的文件或 hunk |
| `alt+a` `alt+s` | Amend、sign-off |
| `alt+m` `alt+n` `alt+shift+d` | 合并、新建、删除分支 |
| `alt+o` `alt+t` `alt+r` | 采用我方、采用对方、标记已解决 |
| `alt+d` `alt+i` | 丢弃改动、忽略未跟踪文件 |
| `alt+x` `alt+u` | 中止或继续 merge / rebase |
| `p` `f` `ctrl+b` | Pull、Fetch、分支过滤 |
| `alt+w` `alt+z` | 词级高亮、软折行 |
| `/` `?` | 过滤日志、查看全部按键 |
| `ctrl+q` | 退出 |

`?` 与 `gitdeck --help` 输出自同一份按键表。

## 部分提交

打开 diff，移到某个 `@@` 行，按 `space`，这个 hunk 就被勾上。只有勾选的 hunk 会进入提交。
补丁取自索引字节，写入临时索引，CRLF、Tab 和相邻 hunk 都能原样保留。

![diff](assets/screenshots/diff.png)

## 未实现

- 交互式 rebase 与 rebase 词表
- cherry-pick、revert、reset
- 三方合并编辑器
- stash
- 并排 diff、行级暂存、hunk 拆分
- blame、文件历史、多仓库、tag 管理、子模块

## 终端

| 环境 | 档位 |
|---|---|
| `COLORTERM=truecolor`、iTerm2、WezTerm、ghostty、VS Code、Terminal.app | `rich` |
| `TERM=*256color`、`screen` | `standard` |
| `NO_COLOR`、`TERM=dumb` / `vt100` / `ansi` / `linux`、输出被管道接管 | `basic` |

`basic` 只输出 ASCII。可以用 `GITDECK_CAPS` 或 `--caps` 覆盖。

## 性能

日志采用虚拟滚动，提交详情在选中项稳定后才查询，不随每次按键发起进程。6,000 提交的
仓库首屏不到一秒，滚动不会为每个事件 fork 一个 `git`。

## 开发

```sh
just verify        # fmt、clippy、单测、套件、真终端 e2e、截图门禁
just clean-verify  # 克隆当前提交到临时目录，在那里完整跑一遍
just probe steps.txt
```

测试在沙箱里运行：`CARGO_HOME`、`TMPDIR`、XDG 目录和 `GIT_CONFIG_GLOBAL` 都指向仓库内。
e2e 通过伪终端驱动真实二进制，键盘和鼠标都覆盖。

## 致谢

工具窗布局和键盘操作流来自 JetBrains 的 Git 窗口。提交图、diff 渲染和 hunk 暂存是
针对终端重写的。

## 许可证

Apache-2.0 或 MIT，任选其一。
