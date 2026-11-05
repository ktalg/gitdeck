# gitdeck

A Git client for the terminal, laid out like the JetBrains Git tool windows.

![changes](assets/screenshots/changes.png)

Three panes: changes on the left, history in the middle, commit details on the right.
Keyboard first, mouse optional. It reads and writes through the `git` CLI, so nothing
in your repository behaves differently than it does in your shell.

**English** · [简体中文](README.zh-CN.md)

## What it does

- Stage files, or individual hunks inside a diff, and commit only what you checked
- Commit graph with branches, tags and merge lanes
- Per-file diffs with word-level highlighting and soft wrapping
- Branch list: check out, create, delete, merge
- Push, fetch, and pull with fast-forward, merge or rebase strategy
- Merge and rebase state in the footer, with abort and continue
- Conflict resolution: take ours, take theirs, or mark resolved
- Discard a file's changes, ignore an untracked path

![log](assets/screenshots/log.png)

## Install

```sh
cargo install --path crates/gitdeck-cli --locked
```

Or build and run from a checkout:

```sh
cargo build --release
./target/release/gitdeck ~/some/repo
```

Needs `git` on `PATH` and a terminal of at least 80×24.

```sh
gitdeck --doctor                      # profile, theme and terminal size
gitdeck --mouse-debug /tmp/mouse.log  # raw mouse events, when clicks misbehave
```

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

Open a diff, move to a `@@` header, press `space`, and the hunk is checked. Only
checked hunks are committed. Patches are built from the index bytes and applied to a
temporary index, so CRLF files, tabs and adjacent hunks survive the round trip.

![diff](assets/screenshots/diff.png)

## Not included

- Interactive rebase and a rebase word list
- Cherry-pick, revert, reset
- A three-way merge editor
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

The log is virtualised, and commit details are queried after the selection settles
instead of on every keystroke. A 6,000-commit repository paints in under a second and
scrolls without spawning a process per event.

## Development

```sh
just verify        # fmt, clippy, unit, suite, PTY end-to-end, screenshot gate
just clean-verify  # clone the commit into a scratch directory and verify there
just probe steps.txt
```

Tests run in a sandbox: `CARGO_HOME`, `TMPDIR`, the XDG directories and
`GIT_CONFIG_GLOBAL` all point inside the checkout. The end-to-end suite drives the
real binary through a pseudo terminal, with keyboard and mouse.

## Credits

The tool-window layout and the keyboard flow come from JetBrains' Git windows. The
graph renderer, the diff renderer and the hunk staging are written for the terminal.

## License

Apache-2.0 or MIT, at your option.
