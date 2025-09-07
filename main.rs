extern crate ncurses;

use ncurses::*;
use std::fs;
use std::io;

struct DirView {
    window: WINDOW, // ncurses window
    selected: i32, // Selected row (absolute index)
    scroll_offset: i32, // First visible entry index
    dirents: Vec<fs::DirEntry>, // Directory entries
    path: std::path::PathBuf, // Path
}

impl DirView {
    fn new(win_height: i32, win_width: i32, win_starty: i32, win_startx: i32, path: &std::path::Path) -> io::Result<Self> {
        // Throw if win_height or win_width is less than 3
        if win_height < 3 || win_width < 3 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Window height and width must be at least 3"));
        }
        let window = newwin(win_height, win_width, win_starty, win_startx);
        keypad(window, true);
        scrollok(window, true);
        let dirents = read_directory_contents(path.to_str().unwrap())?;
        Ok(DirView {
            window,
            selected: 0,
            scroll_offset: 0,
            dirents,
            path: path.to_path_buf(),
        })
    }
    fn draw(&self) {
        // Drawing logic moved to main loop for better control
        werase(self.window);
        box_(self.window, 0, 0);
        mvwaddstr(self.window, 0, 2, self.path.to_str().unwrap());

        let win_height = getmaxy(self.window);
        let list_height = win_height - 2; // Adjust for borders

         // Display directory entries with scrolling
        for (i, entry) in self.dirents.iter().enumerate()
            .skip(self.scroll_offset as usize)
            .take(list_height as usize)  // As many as fit in the window
        {
            if i >= (win_height - 2) as usize {
                break;
            }
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if i as i32 == self.selected {
                wattron(self.window, A_REVERSE);
            }
            mvwaddstr(self.window, (i + 1) as i32, 1, &file_name_str);
            if i as i32 == self.selected {
                wattroff(self.window, A_REVERSE);
            }
        }
        mvwaddstr(self.window, win_height - 1, 2, "Use arrow keys to move, 'q' to quit.");
        wrefresh(self.window);
    }
}

fn main() {
    initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    //start_color();
    //init_pair(1, COLOR_CYAN, COLOR_BLACK);
    //init_pair(2, COLOR_YELLOW, COLOR_BLACK);

    let cwd = std::env::current_dir().expect("Failed to get current directory");

    let (win_height, win_width, win_starty, win_startx);
    {
        // Get terminal size
        let max_y = getmaxy(stdscr());
        let max_x = getmaxx(stdscr());
        win_height = max_y;
        win_width = max_x / 2;
        win_starty = 0;
        win_startx = max_x / 2;
    }
    let mut dirview = DirView::new(win_height, win_width, win_starty, win_startx, &cwd)
        .expect("Failed to initialize DirView");

    loop {
        DirView::draw(&dirview);

        let ch = wgetch(dirview.window);
        match ch {
            KEY_UP => {
                if dirview.selected > 0 {
                    // Move cursor up to previous entry
                    dirview.selected -= 1;
                    if dirview.selected < dirview.scroll_offset {
                        // Scroll up
                        dirview.scroll_offset -= 1;
                    }
                } else {
                    // Bell on attempt to move above first entry
                    beep();
                }
            }
            KEY_DOWN => {
                let list_height = win_height - 2; // Adjust for borders
                if dirview.selected + 1  < dirview.dirents.len() as i32 {
                    // Move cursor down to next entry
                    dirview.selected += 1;
                    if dirview.selected >= dirview.scroll_offset + list_height {
                        // Scroll down
                        dirview.scroll_offset += 1;
                    }
                }
                else {
                    // Bell on attempt to move below last entry
                    beep();
                }
            }
            113 | 27 => {
                break;
            }
            //KEY_RESIZE => {
            //    // Handle resize: recalculate sizes, recreate windows, etc.
            _ => {}
        }
    }

    delwin(dirview.window);
    endwin();
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
