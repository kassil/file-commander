// Copyright (c) 2025 Kevin Kassil
// Terminal file manager inspired by Norton Commander

extern crate ncurses;

use ncurses::*;
use std::fs;
use std::io;
//mod modal_dialog;
mod viewer;

struct DirView {
    window: WINDOW, // ncurses window
    selected: usize, // Selected row (absolute index)
    scroll_offset: usize, // First visible entry index
    dirents: io::Result<Vec<DirListItem>>, // Directory entries
    path: std::path::PathBuf, // Path of the directory being viewed
    dirty: bool, // Needs redraw
}

enum DirListItem {
    ParentDir(std::path::PathBuf),      // Represents ".."
    Entry(fs::DirEntry),                // Actual filesystem entry
}

impl DirView {
    // Change to a new directory and load its contents
    fn load(&mut self, current_path: &std::path::Path) {
        self.path = current_path.to_path_buf();
        self.reload();
    }

    // Update the directory listing from the filesystem
    fn reload(&mut self) {
        let mut elts = Vec::new();
        // Add the parent entry first (unless we're at the root)
        if let Some(parent) = self.path.parent() {
            elts.push(DirListItem::ParentDir(parent.to_path_buf()));
        }

        let foo = read_directory_contents(&self.path);
        match foo {
            Ok(entries) => {
                // Add real directory entries
                // for entry in entries.drain(..) {
                //     elts.push(DirListItem::Entry(entry));
                // }
                elts.extend(entries.into_iter().map(DirListItem::Entry));
                self.dirents = Ok(elts);
            }
            Err(e) => {
                // Store the error
                self.dirents = Err(e);
            }
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
        wattron(window, COLOR_PAIR(1));
        keypad(window, true);
        scrollok(window, true);
        wbkgd(window, COLOR_PAIR(1));
        let mut dirview = DirView {
            window,
            selected: 0,
            scroll_offset: 0,
            dirents: Ok(Vec::new()), // Placeholder, will be loaded
            path: path.to_path_buf(),
            dirty: true,
        };
        dirview.load(path); // Load directory contents before returning
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
            Ok(elements) => {
                let view_height = (win_height - 2) as usize; // Adjust for borders
                // Display directory entries with scrolling
                for (i, entry) in elements
                    .iter()
                    .enumerate()
                    .skip(self.scroll_offset)       // Top of page
                    .take(view_height)     // As many as fit in the window
                {
                    match entry {
                        DirListItem::ParentDir(_) => {
                            let file_name_str = "[..]".to_string();
                            if i == self.selected {
                                wattron(self.window, A_REVERSE);
                            }
                            // Highlight directories in a different color
                            wattron(self.window, COLOR_PAIR(2));
                            mvwaddstr(self.window, (i + 1 - self.scroll_offset) as i32, 1, &file_name_str);
                            wattron(self.window, COLOR_PAIR(1)); // Reset to default color
                            if i == self.selected {
                                wattroff(self.window, A_REVERSE);
                            }
                        }
                        DirListItem::Entry(entry) => {
                            let file_name_str = display_name(entry);
                            if i == self.selected {
                                wattron(self.window, A_REVERSE);
                            }
                            // Highlight directories in a different color
                            if is_openable_dir(entry) {
                                wattron(self.window, COLOR_PAIR(2));
                            }
                            mvwaddstr(self.window, (i + 1 - self.scroll_offset) as i32, 1, &file_name_str);
                            if i == self.selected {
                                wattroff(self.window, A_REVERSE);
                            }
                            wattron(self.window, COLOR_PAIR(1)); // Reset to default color
                        }
                    }
                }
                waddstr(w_debug, &format!("Draw {}:{}\n", self.scroll_offset, self.scroll_offset + view_height));
            }
            Err(e) => {
                // Highlight directories in a different color
                let file_name_str = "[..]".to_string();
                wattron(self.window, A_REVERSE);
                wattron(self.window, COLOR_PAIR(2));
                mvwaddstr(self.window, 1, 1, &file_name_str);
                wattron(self.window, COLOR_PAIR(1)); // Reset to default color
                wattroff(self.window, A_REVERSE);
                mvwaddstr(self.window, 2, 1, &format!("Read error: {}", e));
            }
        }
        mvwaddstr(self.window, win_height - 1, 2, "Use arrow keys to move, 'q' to quit.");
        wrefresh(self.window);
        self.dirty = false;
    }
}

fn scroll_down(w_debug: WINDOW, dirview: &mut DirView) {
    if let Ok(ref list) = dirview.dirents {
        if dirview.selected + 1 < list.len() {
            let view_height = (getmaxy(dirview.window) - 2) as usize; // Adjust for borders
            // Move cursor down to next entry
            dirview.selected += 1;
            if dirview.selected >= dirview.scroll_offset + view_height {
                // Scroll down
                dirview.scroll_offset += 1;
            }
            waddstr(w_debug, &format!("KDOWN Beg:{} Sel:{} End:{}\n", dirview.scroll_offset, dirview.selected, dirview.scroll_offset + view_height));
            dirview.dirty = true;
        }
        else {
            beep();  // Cannot move below last entry
        }
    }
    else {
        beep();  // No entries, cannot move
    }
}

fn scroll_up(w_debug: WINDOW, dirview: &mut DirView) {
    if let Ok(ref _list) = dirview.dirents {
        if dirview.selected > 0 {
            // Move cursor up to previous entry
            dirview.selected -= 1;
            if dirview.selected < dirview.scroll_offset {
                // Scroll up
                dirview.scroll_offset -= 1;
            }
            let view_height = (getmaxy(dirview.window) - 2) as usize; // Adjust for borders
            waddstr(w_debug, &format!("KUP Beg:{} Sel:{} End:{}\n", dirview.scroll_offset, dirview.selected, dirview.scroll_offset + view_height));
            dirview.dirty = true;
        } else {
            beep();  // Cannot move above first entry
        }
    }
    else {
        beep();  // No entries, cannot move
    }
}

fn main() {
    initscr();
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    start_color();
    init_pair(1, COLOR_WHITE, COLOR_BLUE);      // Regular files
    init_pair(2, COLOR_YELLOW, COLOR_BLUE);     // Directories

    let w_debug = newwin(getmaxy(stdscr()), getmaxx(stdscr())/2, 0, 0);
    if w_debug.is_null() {
        endwin();
        eprintln!("Create debug window failed");
        std::process::exit(1);
    }
    keypad(w_debug, true);
    scrollok(w_debug, true);
    waddstr(w_debug, "Debug Window\n");

    // modal_dialog::hello_modal(w_debug);

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
                scroll_up(w_debug, &mut dirview);
            }
            KEY_DOWN => {
                scroll_down(w_debug, &mut dirview);
            }
            KEY_ENTER | 10 | 13 => {  // Handle different ENTER representations
                if let Ok(ref elements) = dirview.dirents {
                    // Get the selected entry
                    if let Some(selected_item) = elements.get(dirview.selected) {
                        match selected_item {
                            DirListItem::ParentDir(parent) => {
                                let parent_clone = parent.clone();  // Clone the parent path
                                // Navigate to parent directory
                                dirview.load(&parent_clone);
                                waddstr(w_debug, &format!("KENTER: Chdir {}\n", parent_clone.display()));
                                continue;
                            }
                            DirListItem::Entry(entry) => {
                                let path = entry.path();  // Owns the path
                                if path.is_dir() {
                                    // Navigate to sub-directory
                                    dirview.load(&path);
                                    waddstr(w_debug, &format!("KENTER: Chdir {}\n", path.to_path_buf().display()));
                                } else {
                                    // Handle file (open, view, edit, ...)
                                    waddstr(w_debug, &format!("KENTER: Open {}\n", path.to_path_buf().display()));
                                    viewer::view_file_modal(w_debug, &path);
                                    waddstr(w_debug, &format!("KENTER: Close {}\n", path.to_path_buf().display()));
                                    // Redraw now
                                    dirview.dirty = true;
                                }
                            }
                        }
                    }
                    else {
                        waddstr(w_debug, &format!("KENTER: No entry at selected index {}!\n", dirview.selected));
                    }
                }
                else {
                    if let Some(parent) = dirview.path.parent() {
                        //.unwrap_or(std::path::Path::new("/")) {
                        let parent_clone = parent.to_path_buf();  // Clone the parent path
                        // Navigate to parent directory
                        dirview.load(&parent_clone);
                    }
                }
            }
            113 | 27 => {
                // Escape or 'q' to quit
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

/// Read the contents of a directory and return the entries.
/// Returns a Vec of DirEntry for the given directory path.
/// Returns an io::Error if the directory can't be read.
fn read_directory_contents(path: &std::path::Path) -> io::Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

// Check if the target is a directory and can be opened.
// Follows symlinks.
fn is_openable_dir(entry: &fs::DirEntry) -> bool {
    let path = entry.path();

    // Follow symlinks, check if target is a directory
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_dir() => {
            true
            // Try to actually open the directory to confirm it's accessible
            //fs::read_dir(&path).is_ok()
        }
        _ => false,
    }
}

fn display_name(entry: &fs::DirEntry) -> String {
    let file_name_os = entry.file_name();                     // Own the OsString
    let name = file_name_os.to_string_lossy();                // Borrow from that
    if is_openable_dir(entry) {
        format!("[{}]", name)
    } else {
        name.into()
    }
}
