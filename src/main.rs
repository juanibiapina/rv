use std::collections::HashMap;
use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use rv::app::{Action, App};
use rv::diff;
use rv::error::Error;
use rv::git;

fn main() {
    if let Err(e) = run() {
        eprintln!("rv: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    // Precondition checks
    if !git::is_git_repo() {
        return Err(Error::Git("not a git repository".into()));
    }

    let diff_args = git::worktree_diff_args();
    let files = git::changed_files(&diff_args)?;

    let raw_diffs = git::all_file_diffs(&diff_args)?;
    let diffs: HashMap<String, _> = raw_diffs
        .into_iter()
        .map(|(path, raw)| (path, diff::parse_side_by_side(&raw)))
        .collect();

    // Set up terminal
    enable_raw_mode().map_err(|e| Error::Git(format!("terminal: {}", e)))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| Error::Git(format!("terminal: {}", e)))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| Error::Git(format!("terminal: {}", e)))?;

    // Run the app (restore terminal on any exit path)
    let result = run_app(&mut terminal, files, diffs);

    // Restore terminal
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    files: Vec<git::FileEntry>,
    diffs: HashMap<String, rv::diff::SideBySideDiff>,
) -> Result<(), Error> {
    let mut app = App::new(files, diffs);
    app.load_initial_diff();

    loop {
        // Update visible rows based on terminal size
        let size = terminal.size().unwrap_or_default();
        // Subtract 3 for borders + status bar
        let visible = if size.height > 3 {
            (size.height - 3) as usize
        } else {
            1
        };
        app.file_scroll.visible_rows = visible;
        app.diff_scroll.visible_rows = visible;

        terminal
            .draw(|frame| rv::ui::render_status_bar(frame, &app))
            .map_err(|e| Error::Git(format!("render: {}", e)))?;

        // Wait for at least one event, then drain all pending events before rendering
        if event::poll(Duration::from_millis(100))
            .map_err(|e| Error::Git(format!("event: {}", e)))?
        {
            let mut quit = false;
            loop {
                if let Event::Key(key) =
                    event::read().map_err(|e| Error::Git(format!("event: {}", e)))?
                {
                    if key.kind == KeyEventKind::Press {
                        if app.handle_key_event(key) == Action::Quit {
                            quit = true;
                            break;
                        }
                    }
                }
                if !event::poll(Duration::ZERO)
                    .map_err(|e| Error::Git(format!("event: {}", e)))?
                {
                    break;
                }
            }
            if quit {
                break;
            }
        }
    }

    Ok(())
}

