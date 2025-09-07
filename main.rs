extern crate ncurses;

use ncurses::*;
use std::fs;
use std::io;

struct DirView {
    window: WINDOW, // ncurses window
    selected: usize, // Selected row (absolute index)
    scroll_offset: usize, // First visible entry index
    dirents: io::Result<Vec<fs::DirEntry>>, // Directory entries
    path: std::path::PathBuf, // Path
    dirty: bool, // Needs redraw
}

impl DirView {
    // Update the directory listing from the filesystem
    fn load(&mut self) {
        self.dirents = read_directory_contents(self.path.to_str().unwrap());
        if let Ok(ref mut entries) = self.dirents {
            entries.sort_by_key(|e| e.file_name());
        }
        self.selected = 0;
        self.scroll_offset = 0;
        self.dirty = true;
    }
    // Create a new DirView instance
    fn new(win_height: i32, win_width: i32, win_starty: i32, win_startx: i32, path: &std::path::Path) -> io::Result<Self> {
        // Throw if win_height or win_width is less than 3
        if win_height < 3 || win_width < 3 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Dirview height and width must be at least 3"));
        }
        let window = newwin(win_height, win_width, win_starty, win_startx);
        keypad(window, true);
        scrollok(window, true);
        let mut dirview = DirView {
            window,
            selected: 0,
            scroll_offset: 0,
            dirents: Ok(Vec::new()), // Placeholder, will be loaded
            path: path.to_path_buf(),
            dirty: true,
        };
        dirview.load(); // Load directory contents before returning
        Ok(dirview)
    }

    // Resize the DirView window
    fn resize(&mut self, new_height: i32, new_width: i32, new_starty: i32, new_startx: i32) {
        // Resize the window
        wresize(self.window, new_height, new_width);
        mvwin(self.window, new_starty, new_startx);
        self.dirty = true;
    }

    // Draw the DirView contents if dirty
    fn draw(&mut self, w_debug: WINDOW) {
        // Drawing logic
        if self.dirty == false {
            return;
        }

        werase(self.window);
        box_(self.window, 0, 0);
        // Display path at the top
        let rc = mvwaddstr(self.window, 0, 2, self.path.to_str().unwrap());
        if let Err(rc) = rc {
            panic!("mvwaddstr path: {} error: {}", self.path.display(), rc);
        }
        if let Ok(rc) = rc {
            if rc == ERR {
                panic!("mvwaddstr returned ERR for path: {}", self.path.display());
            }
        }

        let win_height = getmaxy(self.window);
        match &self.dirents {
            Ok(dirents) => {
                let list_height = win_height - 2; // Adjust for borders

                // Display directory entries with scrolling
                waddstr(w_debug, &format!("Sc{} Sel{} ", self.scroll_offset, self.selected));
                for (i, entry) in dirents.iter()
                    .enumerate()
                    .skip(self.scroll_offset)       // Top of page
                    .take(list_height as usize)     // As many as fit in the window
                {
                    waddstr(w_debug, &format!(" {}", i));
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();
                    if i == self.selected {
                        wattron(self.window, A_REVERSE);
                    }
                    mvwaddstr(self.window, (i + 1 - self.scroll_offset) as i32, 1, &file_name_str);
                    if i == self.selected {
                        wattroff(self.window, A_REVERSE);
                    }
                }
                waddstr(w_debug, "\n"); // Newline in debug window
            }
            Err(e) => {
                mvwaddstr(self.window, 1, 1, &format!("Read error: {}", e));
                wrefresh(self.window);
                return;
            }
        }
        mvwaddstr(self.window, win_height - 1, 2, "Use arrow keys to move, 'q' to quit.");
        wrefresh(self.window);
        self.dirty = false;
    }
}

fn main() {
    initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    //start_color();
    //init_pair(1, COLOR_CYAN, COLOR_BLACK);
    //init_pair(2, COLOR_YELLOW, COLOR_BLACK);

    let w_debug = newwin(getmaxy(stdscr()), getmaxx(stdscr())/2, 0, 0);
    if w_debug.is_null() {
        endwin();
        eprintln!("Create debug window failed");
        std::process::exit(1);
    }
    keypad(w_debug, true);
    scrollok(w_debug, true);
    waddstr(w_debug, "Debug Window\n");

    let cwd = std::env::current_dir().expect("Failed to get current directory");

    let (init_win_height, init_win_width, init_win_starty, init_win_startx);
    {
        // Get terminal size
        let max_y = getmaxy(stdscr());
        let max_x = getmaxx(stdscr());
        init_win_starty = 0;
        init_win_startx = max_x / 2;
        init_win_height = max_y;
        init_win_width = max_x - init_win_startx;
    }
    let mut dirview = DirView::new(init_win_height, init_win_width, init_win_starty, init_win_startx, &cwd)
        .expect("Failed to initialize DirView");

    loop {
        // Draw if dirty
        dirview.draw(w_debug);
        wrefresh(w_debug);

        // Handle input
        let ch = wgetch(dirview.window);
        match ch {
            KEY_UP => {
                if let Ok(ref _dirents) = dirview.dirents {
                    if dirview.selected > 0 {
                        // Move cursor up to previous entry
                        dirview.selected -= 1;
                        if dirview.selected < dirview.scroll_offset {
                            // Scroll up
                            dirview.scroll_offset -= 1;
                        }
                        waddstr(w_debug, &format!("KUP Sel{} Scr{}\n", dirview.selected, dirview.scroll_offset));
                        dirview.dirty = true;
                    } else {
                        // Bell on attempt to move above first entry
                        beep();
                    }
                }
                else {
                    beep();
                }
            }
            KEY_DOWN => {
                if let Ok(ref dirents) = dirview.dirents {
                    if dirview.selected + 1 < dirents.len() {
                        let list_height = getmaxy(dirview.window) - 2; // Adjust for borders
                        // Move cursor down to next entry
                        dirview.selected += 1;
                        if dirview.selected >= dirview.scroll_offset + list_height as usize {
                            // Scroll down
                            dirview.scroll_offset += 1;
                        }
                        waddstr(w_debug, &format!("KDOWN Sel{} Scr{} LD{} N{}\n", dirview.selected, dirview.scroll_offset, list_height, dirents.len()));
                        dirview.dirty = true;
                    }
                    else {
                        // Bell on attempt to move below last entry
                        beep();
                    }
                }
            }
            113 | 27 => {
                break;
            }
            KEY_RESIZE => {
                // Resize dirview
                // Get terminal size
                let max_y = getmaxy(stdscr());
                let max_x = getmaxx(stdscr());
                let win_starty = 0;
                let win_startx = max_x / 2;
                let win_height = max_y;
                let win_width = max_x - win_startx;
                dirview.resize(win_height, win_width, win_starty, win_startx);
                // Resize debug window
                wresize(w_debug, max_y, max_x/2);
                mvwin(w_debug, 0, 0);
            }
            _ => {}
        }
    }

    delwin(dirview.window);
    delwin(w_debug);
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
