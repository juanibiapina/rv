# rv

Code review TUI. Two-panel interface: file tree and side-by-side diffs for working tree changes.

## Install

```
cargo install --path .
```

## Usage

Run `rv` in any git repository:

```
rv
```

Shows unstaged working tree changes. Select a file to see its diff.

## Keybindings

### File list

| Key | Action |
|-----|--------|
| `j` / `Down` | Next file |
| `k` / `Up` | Previous file |
| `g` / `Home` | First file |
| `G` / `End` | Last file |
| `J` / `K` | Scroll diff down / up |
| `PgDn` / `PgUp` | Page down / up |
| `Enter` | Expand/collapse directory, or open diff |
| `Tab` | Switch to diff panel |
| `q` / `Esc` | Quit |

### Diff panel

| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `g` / `Home` | Top |
| `G` / `End` | Bottom |
| `PgDn` / `PgUp` | Page down / up |
| `Tab` / `Esc` | Back to file list |
| `q` | Quit |
