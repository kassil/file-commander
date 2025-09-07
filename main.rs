extern crate ncurses;

use ncurses::*;
use std::fs;
use std::io;

struct DirView {
    window: WINDOW, // ncurses window
    row: i32,      // Current row in the directory listing
    dirents: Vec<fs::DirEntry>, // Directory entries
    cwd: std::path::PathBuf, // Path
}

impl DirView {
    fn new(win_height: i32, win_width: i32, win_starty: i32, win_startx: i32, path: &std::path::Path) -> io::Result<Self> {
        let window = newwin(win_height, win_width, win_starty, win_startx);
        keypad(window, true);
        scrollok(window, true);
        let dirents = read_directory_contents(path.to_str().unwrap())?;
        Ok(DirView {
            window,
            row: 0,
            dirents,
            cwd: path.to_path_buf(),
        })
    }
}

fn main() {
    initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    start_color();
    init_pair(1, COLOR_CYAN, COLOR_BLACK);
    init_pair(2, COLOR_YELLOW, COLOR_BLACK);

    let mut max_y = 0;
    let mut max_x = 0;
    getmaxyx(stdscr(), &mut max_y, &mut max_x);

    let win_height = max_y;
    let win_width = max_x / 2;
    let win_starty = 0;
    let win_startx = max_x / 2;

    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let mut dirview = DirView::new(win_height, win_width, win_starty, win_startx, &cwd)
        .expect("Failed to initialize DirView");

    loop {
        werase(dirview.window);
        box_(dirview.window, 0, 0);
        mvwaddstr(dirview.window, 0, 2, "PATH");
        for (i, entry) in dirview.dirents.iter().enumerate() {
            if i >= (win_height - 2) as usize {
                break;
            }
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            mvwaddstr(dirview.window, (i + 1) as i32, 1, &file_name_str);
        }
        mvwaddstr(dirview.window, win_height - 1, 0, "Use arrow keys to move, 'q' to quit.");
        wrefresh(dirview.window);

        let ch = getch();
        match ch {
            KEY_UP => {
                if dirview.row > 0 {
                    dirview.row -= 1;
                }
            }
            KEY_DOWN => {
                if dirview.row < (dirview.dirents.len() as i32 - 1) && dirview.row < (win_height - 3) {
                    dirview.row += 1;
                }
            }
            113 | 27 => {
                break;
            }
            _ => {}
        }
    }

    delwin(dirview.window);
    endwin();
}

// ...existing code...

fn main() {
    initscr();              // Start ncurses mode
    //keypad(stdscr(), true); // Enable arrow keys
    noecho();               // Don't echo input
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    start_color();         // Enable color if terminal supports it
    init_pair(1, COLOR_CYAN, COLOR_BLACK); // Define color pair 1
    init_pair(2, COLOR_YELLOW, COLOR_BLACK); // Define color pair 2

    // Get screen size
    let mut max_y = 0;
    let mut max_x = 0;
    getmaxyx(stdscr(), &mut max_y, &mut max_x);
    // // Cast to i16, truncating if too large
    // let max_y = if max_y > i16::MAX as i32 { i16::MAX } else { max_y as i16 };
    // let max_x = if max_x > i16::MAX as i32 { i16::MAX } else { max_x as i16 };

    // Calculate right half window position and size
    let win_height = max_y;
    let win_width = max_x / 2;
    let win_starty = 0;
    let win_startx = max_x / 2;

    // Open the current working directory
    let mut cwd = std::env::current_dir().expect("Failed to get current directory");

    let dirView = DirView {
        window: stdscr(),
        row: 0,
        dirents: Vec::new(),
        cwd: cwd.clone(),
    };

    // Create a new window for the right half
    let win = newwin(win_height, win_width, win_starty, win_startx);
    keypad(win, true);
    scrollok(win, true);

    dirview.dirents = read_directory_contents(cwd.to_str().unwrap()).expect("Failed to read directory");
    let mut row = 0;

    loop {
        werase(win); // Clear the window
        box_(win, 0, 0); // Redraw the box
        mvwaddstr(win, 0, 2, "PATH");
        for (i, entry) in dirents.iter().enumerate() {
            if i >= (max_y - 2).try_into().unwrap() {
                break; // Bottom of window
            }
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            mvwaddstr(win, (i + 1) as i32, 1, &file_name_str);
        }
        mvwaddstr(win, max_y - 1, 0, "Use arrow keys to move, 'q' to quit."); // Instructions at bottom
        wrefresh(win); // Refresh the window to show changes
        
        // Player input
        let ch = getch();
        match ch {
            KEY_UP => {
                if row > 0 {
                    row -= 1;
                }
            }
            KEY_DOWN => {
                if row < (dirents.len() as i32 - 1) && row < (max_y - 3) {
                    row += 1;
                }
            }
            113 | 27 => { // 'q' or ESC to quit
                break;
            }
            _ => {}
        }
    }

    delwin(win);
    endwin();               // End ncurses mode
}







/// Returns a Vec of DirEntry for the given directory path.
/// Returns an io::Error if the directory can't be read.
fn read_directory_contents(path: &str) -> io::Result<Vec<fs::DirEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        entries.push(entry);
    }
    Ok(entries)
}
