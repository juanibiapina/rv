use std::io;
use std::time::Duration;

use ansi_to_tui::IntoText;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use rv::app::{Action, App};
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

    if !git::has_delta() {
        return Err(Error::Git(
            "delta is required but not found. Install with: brew install git-delta".into(),
        ));
    }

    let (diff_args, description) = git::compute_diff_args()?;
    let files = git::changed_files(&diff_args)?;

    if files.is_empty() {
        return Err(Error::Git("no changes to review".into()));
    }

    // Set up terminal
    enable_raw_mode().map_err(|e| Error::Git(format!("terminal: {}", e)))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| Error::Git(format!("terminal: {}", e)))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| Error::Git(format!("terminal: {}", e)))?;

    // Run the app (restore terminal on any exit path)
    let result = run_app(&mut terminal, files, diff_args, description);

    // Restore terminal
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    files: Vec<git::FileEntry>,
    diff_args: Vec<String>,
    description: String,
) -> Result<(), Error> {
    let mut app = App::new(files, description);

    // Load diff for the first file immediately
    if app.selected_file().is_some() {
        load_diff_for_selected(&mut app, &diff_args)?;
    }

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

        // Poll for events
        if event::poll(Duration::from_millis(100))
            .map_err(|e| Error::Git(format!("event: {}", e)))?
        {
            if let Event::Key(key) = event::read().map_err(|e| Error::Git(format!("event: {}", e)))? {
                // Only handle key press events, not release
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app.handle_key(key.code) {
                    Action::Quit => break,
                    Action::LoadDiff(_idx) => {
                        load_diff_for_selected(&mut app, &diff_args)?;
                    }
                    Action::Render | Action::None => {}
                }
            }
        }
    }

    Ok(())
}

fn load_diff_for_selected(app: &mut App, diff_args: &[String]) -> Result<(), Error> {
    if let Some(file) = app.selected_file() {
        let path = file.path.clone();
        let raw = git::file_diff_with_delta(diff_args, &path)?;

        let text = raw
            .into_text()
            .unwrap_or_else(|_| ratatui::text::Text::raw(String::from_utf8_lossy(&raw).into_owned()));

        let line_count = text.lines.len();
        app.set_diff_content(text, line_count);
    }
    Ok(())
}
