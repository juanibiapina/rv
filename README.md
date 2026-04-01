# rv

Git diff viewer TUI. Split-pane interface with a file list on the left and delta-rendered diffs on the right.

## Requirements

- [delta](https://github.com/dandavison/delta) for diff rendering

## Install

```
cargo install --path .
```

## Usage

Run `rv` in any git repository with changes:

```
rv
```

If the working tree is dirty, it shows uncommitted changes (staged + unstaged). If clean, it diffs against the auto-detected base branch.

## Keybindings

### File list

| Key | Action |
|-----|--------|
| `j` / `Down` | Next file |
| `k` / `Up` | Previous file |
| `g` / `Home` | First file |
| `G` / `End` | Last file |
| `Enter` | Toggle expand/collapse directory |
| `Tab` | Switch to diff panel |
| `q` / `Esc` | Quit |

### Diff panel

| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `g` / `Home` | Top |
| `G` / `End` | Bottom |
| `PgUp` | Page up |
| `PgDn` | Page down |
| `Tab` / `Esc` | Back to file list |
