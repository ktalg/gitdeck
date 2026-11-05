# gitdeck

**A Git client for the terminal, with the panes you already know.**

[![ci](https://github.com/ktalg/gitdeck/actions/workflows/ci.yml/badge.svg)](https://github.com/ktalg/gitdeck/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

![gitdeck](assets/demo.gif)

`git status` tells you what changed. It does not tell you which commit a line came from,
which hunks belong in the same commit, or why a merge stopped halfway.

gitdeck puts changes, history and commit details side by side in your terminal, driven by
the keyboard. It shells out to `git`, so your repository behaves exactly as it does in a
shell, and nothing new ends up in it.

**English** · [简体中文](README.zh-CN.md)

## Install

Prebuilt binaries are on the [releases page](https://github.com/ktalg/gitdeck/releases/latest):

| Platform | Asset |
|---|---|
| macOS, Apple Silicon | `gitdeck-0.0.1-macos-arm64` |
| macOS, Intel | `gitdeck-0.0.1-macos-x86_64` |
| Linux, x86-64 (static) | `gitdeck-0.0.1-linux-x86_64` |
| Windows, x86-64 | `gitdeck-0.0.1-windows-x86_64.exe` |

```sh
chmod +x gitdeck-0.0.1-macos-arm64
./gitdeck-0.0.1-macos-arm64 ~/some/repo
```

From source:

```sh
cargo install --path crates/gitdeck-cli --locked
```

Or build it yourself:

```sh
cargo build --release
./target/release/gitdeck ~/some/repo
```

Needs `git` on `PATH` and a terminal of at least 80×24.

```sh
gitdeck --doctor                      # profile, theme and terminal size
gitdeck --mouse-debug /tmp/mouse.log  # raw mouse events, when clicks misbehave
```

## What you get

- **Stage by hunk.** Open a diff, sit on a `@@` header, press `space`. Only what you check
  goes into the commit.
- **A graph, not a wall of text.** Branches, tags and merge lanes, with 10k commits
  scrolling smoothly.
- **Changes that explain themselves.** `MM`, ` D`, `A ` — index and worktree state in two
  columns, next to conflict markers when a merge goes wrong.
- **Branches you can act on.** Check out, create, delete, merge, in the same list.
- **Remotes without a cheatsheet.** Push, fetch, and pull as fast-forward, merge or rebase.
- **Interrupted states that stay visible.** `MERGING`, `REBASING`, abort and continue, with a
  confirmation before anything destructive.
- **Diffs worth reading.** Word-level highlight, soft wrap, dual line numbers, horizontal
  scroll that stops where the line ends.

![changes](assets/screenshots/changes.png)

## Keys

| Key | Action |
|---|---|
| `tab` / `shift+tab` | Move focus between Changes, Log and Details |
| `j` `k` `↑` `↓` | Move the selection, or scroll an open diff |
| `space` | Toggle a checkbox, or the hunk under the cursor |
| `enter` | Open a file diff, or check out the selected branch |
| `i` | Edit the commit message |
| `ctrl+enter` | Commit the checked files or hunks |
| `alt+a` `alt+s` | Amend, sign-off |
| `alt+m` `alt+n` `alt+shift+d` | Merge, create, delete a branch |
| `alt+o` `alt+t` `alt+r` | Take ours, take theirs, mark resolved |
| `alt+d` `alt+i` | Discard changes, ignore an untracked file |
| `alt+x` `alt+u` | Abort or continue a merge or rebase |
| `p` `f` `ctrl+b` | Pull, fetch, branch filter |
| `alt+w` `alt+z` | Word-level highlight, soft wrap |
| `/` `?` | Filter the log, show all keys |
| `ctrl+q` | Quit |

`?` and `gitdeck --help` print from the same binding table.

## Partial commits

![diff](assets/screenshots/diff.png)

`git add -p` is a conversation. Sometimes you want a checklist.

Patches are built from the index bytes and applied to a temporary index, so CRLF files,
tabs and adjacent hunks survive, and a file you did not check cannot sneak into the commit.

## Not in this version

- Interactive rebase, rebase word list
- Cherry-pick, revert, reset
- Three-way merge editor
- Stash
- Side-by-side diff, line-level staging, hunk splitting
- Blame, file history, multiple repositories, tags, submodules

## Terminals

| Environment | Profile |
|---|---|
| `COLORTERM=truecolor`, iTerm2, WezTerm, ghostty, VS Code, Terminal.app | `rich` |
| `TERM=*256color`, `screen` | `standard` |
| `NO_COLOR`, `TERM=dumb` / `vt100` / `ansi` / `linux`, piped output | `basic` |

`basic` renders in ASCII only. Override with `GITDECK_CAPS` or `--caps`.

## Performance

The log is virtualised, and commit details are queried once the selection settles rather
than on every keystroke. A 6,000-commit repository paints in under a second.

## Development

```sh
just verify        # fmt, clippy, unit, suite, PTY end-to-end, screenshot gate
just clean-verify  # clone the commit into a scratch directory and verify there
just probe steps.txt
```

Tests run in a sandbox: `CARGO_HOME`, `TMPDIR`, the XDG directories and
`GIT_CONFIG_GLOBAL` all point inside the checkout. The end-to-end suite drives the real
binary through a pseudo terminal, with keyboard and mouse.

## Credits

The pane layout and the keyboard flow are borrowed from JetBrains' Git windows — muscle
memory is a feature. The graph, the diff renderer and the hunk staging are written for
the terminal, against the terminal.

## License

Apache-2.0 or MIT, at your option.
