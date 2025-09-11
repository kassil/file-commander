// View the contents of a file as text.
// Navigate with arrow keys and Page Up/Down.
// Press Esc to close the window.

use std::io::BufReader;
use std::ptr;
//use std::fs::File;
use std::io::BufRead;
//use std::path::Path;
use ncurses::*;

struct FileViewer {
    window: WINDOW,
    path: std::path::PathBuf,
    //file: std::fs::File,
    //reader: std::io::BufReader<std::fs::File>,
    // Load entire file contents
    lines: Vec<String>,
    // File line number at the top of window
    line_idx: usize,
    // File offset at the top of window
    //offset: usize,
    dirty: bool,
}

impl FileViewer {
    fn new(path: &std::path::Path, reader: std::io::BufReader<std::fs::File> /* file: &std::fs::File*/) -> Self {

        let mut viewer = Self {
            window: ptr::null_mut(), // Placeholder, will be set below
            path: path.to_path_buf(),
            //reader: reader, //BufReader::new(file),
            lines: reader.lines().filter_map(Result::ok).collect(),
            line_idx: 0,
            //offset: 0,
            dirty: true,
        };

        let (width, height, startx, starty) = viewer.calc_extents();
        viewer.window = newwin(height, width, starty, startx);
        keypad(viewer.window, true);

        viewer
    }

    fn draw(&mut self, /*w_debug: WINDOW*/) {

        if self.dirty {

            let n_cols = getmaxx(self.window) - 4;
            let n_rows = getmaxy(self.window) - 2;
            // Erase previous content
            werase(self.window);
            // Draw border and title
            box_(self.window, 0, 0);
            mvwaddnstr(self.window, 0, 2, &self.path.to_string_lossy(), n_cols);
            // Render lines
            // for (i, option) in self.prompt.iter().enumerate() {
            //     // Truncate long lines to fit window width
            //     mvwaddnstr(self.window, (i + 1) as i32, 2, option, n_cols);
            // }

            // Draw current "window" of lines
            for (i, line) in self.lines.iter().skip(self.line_idx).take(n_rows as usize).enumerate() {
                // truncate line if it's longer than window width
                // let display_line = if line.len() > width as usize {
                //     &line[..(width as usize)]
                // } else {
                //     line
                // };
                mvwaddnstr(self.window, 1+i as i32, 1, line, n_cols);
            }
            wrefresh(self.window);
            self.dirty = false;
        }
    }

    fn calc_extents(&self) -> (i32, i32, i32, i32) { 

        // TODO If screen is less than 6 cols or 3 rows, panic
        let screen_cols = getmaxx(stdscr());
        let width = screen_cols;
        let height = getmaxy(stdscr());
        let starty = 0;
        let startx = 0;
        (width, height, startx, starty)
    }

    fn resize(&mut self) {

        let (width, height, startx, starty) = self.calc_extents();
        wresize(self.window, height, width);
        mvwin(self.window, starty, startx);
        self.dirty = true;
    }

    fn handle_input(&mut self, ch: i32) -> Option<usize> {

        match ch {
            KEY_UP => {
                if self.line_idx > 0 {
                    self.line_idx -= 1;
                    self.dirty = true;
                }
            }
            KEY_DOWN => {
                let n_rows = (getmaxy(self.window) - 2) as usize;
                if self.line_idx + n_rows < self.lines.len() {
                    // Seek to next line
                    self.line_idx += 1;
                    self.dirty = true;
                }
            }
            // KEY_PAGE_UP => {
            //     self.
            // }

            27 => return Some(0), // ESC key to close

            KEY_RESIZE => self.resize(),

            _ => {}
        }
        None
    }
}

impl Drop for FileViewer {

    fn drop(&mut self) {

        // werase(self.window);
        // wrefresh(self.window); // Redraw cleared window to erase from screen
        delwin(self.window); // Clean up the window
    }
}

pub fn view_file_modal(w_debug: WINDOW, file_path: &std::path::Path) {

    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    //let path = Path::new(file_path);
    let file = match File::open(&file_path) {
        
        Ok(f) => f,
        Err(e) => {
            waddstr(w_debug, &format!("Error opening file {}: {}\n", file_path.display(), e));
            return;
        }
    };

    let reader = BufReader::new(file);
    //let lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();

    let mut dialog = FileViewer::new(file_path, reader);

    loop {
        //wrefresh(w_debug);  // Draw debug window below dialog
        dialog.draw(); // Redraw after handling input
        // Handle input
        let ch = wgetch(dialog.window);
        if ch == KEY_RESIZE {
            // // Handle terminal resize
            // clear();
            // refresh();
        }
        if let Some(selected) = dialog.handle_input(ch) {
            if selected == 0 {
                // Close option selected
                //waddstr(w_debug, "File viewer closed\n");
                break;
            }
        }
        //waddstr(w_debug, "Viewing file...\n");
    }
}

