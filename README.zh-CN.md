# gitdeck

**住在终端里的 JetBrains 风格 Git 客户端。**

三栏布局、提交图、hunk 级提交、分支、远程、冲突解决——你手指已经记住的那套操作流，
不必离开命令行。

[![ci](https://github.com/ktalg/gitdeck/actions/workflows/ci.yml/badge.svg)](https://github.com/ktalg/gitdeck/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#许可证)
![rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)

[English](README.md) · **简体中文**

![changes](assets/screenshots/changes.png)

## 为什么做它

`git status` 能告诉你**什么**变了，但不会告诉你某一行来自哪个提交、哪些 hunk 该一起提交、
以及一次 merge 为什么停在半路。

`gitdeck` 把熟悉的三块工具窗——**Changes / Log / Details**——摆在你面前，并且让手不离键盘：

- 按**文件**或按 **hunk** 暂存，只提交你真正勾选的内容
- 直接看**提交图**，而不是 `git log --graph --oneline`
- 看得见 `In N branches` 和 committer，而不只是 author
- `alt+o` / `alt+t` / `alt+r` 解决冲突，然后提交
- 用 `MERGING` / `REBASING` 提示当前中断状态，`alt+x` 中止前会先问一次

![log](assets/screenshots/log.png)

## 安装

```sh
cargo install --path crates/gitdeck-cli --locked   # 从源码安装
# 或者
cargo build --release && ./target/release/gitdeck /path/to/repo
```

要求：`git` 在 `PATH` 上（gitdeck 通过 CLI 读写，不自己重新实现 Git）；终端不小于 80×24。

```sh
cd ~/some/repo && gitdeck
gitdeck --doctor                       # 探测到的能力档位、主题、终端尺寸
gitdeck --mouse-debug /tmp/mouse.log   # 点击异常时记录原始鼠标事件
```

## 60 秒上手

| 按键 | 作用 |
|---|---|
| `tab` / `shift+tab` | 在 Changes → Log → Details 之间切换焦点 |
| `j` `k` `↑` `↓` | 移动选中项；diff 页签激活时滚动 diff |
| `space` | 切换复选框；在 diff 里切换光标所在 **hunk** |
| `enter` | 打开文件 diff；checkout 选中的分支 |
| `i` | 编辑提交消息（任意面板可用） |
| `ctrl+enter` | 提交勾选的文件或 hunk |
| `alt+a` `alt+s` | amend、sign-off |
| `alt+m` `alt+n` `alt+shift+d` | 合并、新建、删除分支 |
| `alt+o` `alt+t` `alt+r` | 采用我方 / 采用对方 / 标记已解决 |
| `alt+d` `alt+i` | 丢弃文件改动、忽略未跟踪文件 |
| `alt+x` `alt+u` | 中止 / 继续中断的 merge 或 rebase |
| `p` `f` `ctrl+b` | pull、fetch、分支过滤 |
| `alt+w` `alt+z` | 词级高亮、软折行 |
| `/` `?` | 过滤日志、查看全部键位 |
| `ctrl+q` | 退出（有未提交草稿时会先确认） |

在应用里按 `?` 看到的键位表与 `gitdeck --help` 出自同一份注册表，不会漂移。

## 从四十行里挑四行提交

这正是它存在的理由：`git add -p` 是一场对话，而有时你只想要一张清单。
打开 diff，`j`/`k` 移到某个 `@@`，按 `space`，看到 `[x]` 出现——然后只提交这一块。

![diff](assets/screenshots/diff.png)

gitdeck 用**索引里的原始字节**拼补丁，并通过**临时索引**提交，所以 CRLF 文件、Tab、混杂
hunk 都能原样往返，而你没勾选的文件不会混进这次提交。

## 它不做什么

1.0 明确不做（如实列出，而不是让用户去试）：

- 交互式 rebase 与 rebase 词表
- cherry-pick / revert / reset 菜单
- 三方合并编辑器（冲突按"整边"解决，不能逐行编辑）
- stash / Shelf
- 并排 diff、行级暂存、hunk 拆分
- blame、文件历史、多仓库、tag 管理、子模块

早期版本里出现过的 Shelf、正则过滤、多页签等入口，因为"点了没反应"，已经被**移除**而不是留在那里。

## 终端兼容性

| 环境 | 档位 |
|---|---|
| `COLORTERM=truecolor`、iTerm2、WezTerm、ghostty、VS Code、Terminal.app | `rich` |
| `TERM=*256color`、`screen` | `standard` |
| `NO_COLOR`、`TERM=dumb`/`vt100`/`ansi`/`linux`、输出被管道接管 | `basic`（纯 ASCII） |

可用 `GITDECK_CAPS=rich|standard|basic` 或 `--caps` 覆盖。`basic` 档保证不输出任何非 ASCII 字形。

## 性能

Log 采用虚拟滚动，提交详情的查询会**等选中项稳定后**才执行，所以在 10k 提交的仓库里滚动
不会每按一次键就 fork 一个 `git` 进程。6,000 提交的仓库首屏在 1 秒内出现。

## 开发

```sh
just verify        # guard、fmt、clippy、单测、套件、真终端 e2e、截图门禁
just clean-verify  # 把当前提交克隆到临时目录，在那里完整验证一遍
just probe steps.txt
```

测试跑在沙箱环境里（`CARGO_HOME`、`TMPDIR`、XDG 目录与 `GIT_CONFIG_GLOBAL` 全部重定向到仓库内），
e2e 套件通过伪终端驱动真实二进制——键盘**和**鼠标都覆盖。

## 致谢

工具窗布局和键盘优先的操作流，厚着脸皮借用了 JetBrains 在 IDE 里做 Git 的那一套——
毕竟肌肉记忆也是一种功能。提交图绘制、diff 渲染与 hunk 暂存是自己写的，是对着终端而不是对着别人的代码。

想要整个 IDE，就用整个 IDE；只想在编辑器开着、终端更快的时候处理 Git——这个工具是给你的。

## 许可证

双许可，任选其一：[Apache-2.0](LICENSE-APACHE) 或 [MIT](LICENSE-MIT)。
