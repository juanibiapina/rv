# rv

Git commit browser TUI. Three-panel interface: commit list, file tree, and delta-rendered diffs.

## Requirements

- [delta](https://github.com/dandavison/delta) for diff rendering

## Install

```
cargo install --path .
```

## Usage

Run `rv` in any git repository:

```
rv
```

Opens a three-panel view showing recent commits. Select a commit to see its changed files, select a file to see its diff.

## Keybindings

### Commit list

| Key | Action |
|-----|--------|
| `j` / `Down` | Next commit |
| `k` / `Up` | Previous commit |
| `g` / `Home` | First commit |
| `G` / `End` | Last commit |
| `Tab` | Switch to file list |
| `q` / `Esc` | Quit |

### File list

| Key | Action |
|-----|--------|
| `j` / `Down` | Next file |
| `k` / `Up` | Previous file |
| `g` / `Home` | First file |
| `G` / `End` | Last file |
| `Enter` | Toggle expand/collapse directory |
| `Tab` | Switch to diff panel |
| `Esc` | Back to commit list |
| `q` | Quit |

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
