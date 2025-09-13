// View the contents of a file as text.
// Navigate with arrow keys and Page Up/Down.
// Press Esc to close the window.

use ncurses::*;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

fn find_prev_line_start(w_debug: WINDOW, reader: &mut BufReader<File>, file_pos: u64) -> std::io::Result<u64> {
    if file_pos == 0 {
        // Already at start of file
        waddstr(w_debug, &format!("find_prev_line_start already at beginning\n"));
        return Ok(0);
    }

    // Choose how far back to step (at most 4096 or to start of file)
    // -1 to avoid re-reading current line's newline
    let backstep = 4096.min((file_pos - 1) as usize) as u64;
    let seek_pos = file_pos - backstep - 1;
    reader.seek(SeekFrom::Start(seek_pos))?;

    let mut buf = vec![0u8; backstep as usize];
    let n = reader.read(&mut buf)?;
    let slice = &buf[..n];

    // search backward through slice
    if let Some(rel_idx) = slice.iter().rposition(|&b| b == b'\n') {
        // newline found — line starts right after it
        waddstr(w_debug, &format!("find_prev_line_start found {} + {} = {}\n", seek_pos, rel_idx, seek_pos + rel_idx as u64 + 1));
        Ok(seek_pos + rel_idx as u64 + 1 as u64)
    } else {
        // no newline — in middle of first line, or we didn't go back far enough
        waddstr(w_debug, &format!("find_prev_line_start no newline found before {}!\n", file_pos));
        Ok(0)
    }
}

fn calc_extents() -> (i32, i32, i32, i32) {

    let scr_rows = getmaxy(stdscr());
    let scr_cols = getmaxx(stdscr());
    let height = scr_rows;
    let width = scr_cols - scr_cols/2;
    let startrow = 0;
    let startcol = scr_cols/2;
    (height, width, startrow, startcol)
}

fn resize(w_debug: WINDOW, superwindow: WINDOW, window: WINDOW, file_path: &Path) {
    let (height, width, startrow, startcol) = calc_extents();

    //werase(w_debug);
    wresize(w_debug, height, width);

    wresize(superwindow, height, width);
    mvwin(superwindow, startrow, startcol);

    wresize(window, height - 2, width - 2);
    mvwin(window, startrow + 1, startcol + 1);

    // Redraw border and title
    box_(superwindow, 0, 0);
    mvwaddstr(superwindow, 0, 2, &format!(" {} ", file_path.display()));
    mvwaddstr(superwindow, height - 1, 2, "Up/Down to scroll, Esc or 'q' to close");
    wrefresh(superwindow);
}

fn rtrim(line: &mut String) {
    // Remove trailing newline if present
    if line.ends_with('\n') {
        line.pop(); // remove '\n'
        if line.ends_with('\r') {
            line.pop(); // remove '\r' for Windows CRLF
        }
    }
}

pub fn view_file_modal(w_debug: WINDOW, file_path: &Path) {

    let file = match File::open(file_path) {

        Ok(f) => f,
        Err(e) => {
            waddstr(
                w_debug,
                &format!("Error opening file {}: {}\n", file_path.display(), e),
            );
            return;
        }
    };

    let mut reader = BufReader::new(file);

    let (height, width, startrow, startcol) = calc_extents();
    let superwindow = newwin(height, width, startrow, startcol);
    let window = newwin(height-2, width-2, startrow+1, startcol+1);
    scrollok(window, true);
    keypad(window, true);

    // Box around window
    box_(superwindow, 0, 0);
    // Title with filename
    mvwaddstr(superwindow, 0, 2, &format!(" {} ", file_path.display()));
    // Instructions
    mvwaddstr(superwindow, height-1, 2, "Up/Down to scroll, Esc or 'q' to close");
    wrefresh(superwindow);

    // The file position of each visible line
    let mut line_offsets: VecDeque<u64> = VecDeque::new();

    // Load and display the visible portion
    let mut temp_pos = 0;
    for i in 0 .. getmaxy(window) {
        line_offsets.push_back(temp_pos);
        let mut line = String::new();
        if let Ok(n_bytes) = reader.read_line(&mut line) {
            if n_bytes == 0 {
                break; // EOF
            }
            temp_pos += n_bytes as u64;  // mark where the next line will begin
            // Remove trailing newline
            rtrim(&mut line);
            // Draw the line
            mvwaddnstr(window, i as i32, 0, &line, getmaxx(window));

            // record starting position
        }
    }
    line_offsets.push_back(temp_pos);
    wrefresh(window);
    waddstr(w_debug, &format!("OPEN N:{} offsets:", line_offsets.len()));
    for i in &line_offsets {
        waddstr(w_debug, &format!(" {}", i));
    }
    waddstr(w_debug, "\n");

    loop {
        wrefresh(w_debug); // Draw debug window below dialog

        match wgetch(window) {
            KEY_DOWN => {
                // Rust note: copy the element, otherwise we'd hold an immut reference to the list.
                let bot_file_pos = *line_offsets.back().unwrap();
                // Read a line
                reader.seek(SeekFrom::Start(bot_file_pos));
                let mut line = String::new();
                let line_n_bytes = reader.read_line(&mut line).unwrap();
                if line_n_bytes == 0 {
                    // EOF: cannot scroll down
                    beep();
                }
                else {
                    // Remove the old top row
                    line_offsets.pop_front();
                    // Add the new bottom row
                    line_offsets.push_back(bot_file_pos + line_n_bytes as u64);

                    // Remove trailing newline if present
                    if line.ends_with('\n') {
                        line.pop(); // remove '\n'
                        if line.ends_with('\r') {
                            line.pop(); // remove '\r' for Windows CRLF
                        }
                    }

                    // Draw the bottom row
                    wscrl(window, 1);
                    mvwaddnstr(window, getmaxy(window) - 1, 0, &line, getmaxx(window));
                    wrefresh(window);

                    waddstr(w_debug, &format!("KDOWN top:{} bot:{} n:{}\n", line_offsets.front().unwrap(), line_offsets.back().unwrap(), line_offsets.len()));
                }
            }

            KEY_UP => {
                // Find the line before the top one
                // if line_offsets.front() and ...
                if *line_offsets.front().unwrap() > 0 && let Ok(new_pos) = find_prev_line_start(w_debug, &mut reader, *line_offsets.front().unwrap()) {

                    // Advance bottom row
                    line_offsets.pop_back();
                    line_offsets.push_front(new_pos);

                    waddstr(w_debug, &format!("KUP top:{} bot:{} N:{}\n",
                        *line_offsets.front().unwrap(), *line_offsets.back().unwrap(), line_offsets.len()));
                    reader.seek(SeekFrom::Start(new_pos));
                    // Read one new line at top
                    let mut line = String::new();
                    if let Ok(_line_n_bytes) = reader.read_line(&mut line) {

                        // Remove trailing newline if present
                        if line.ends_with('\n') {
                            line.pop(); // remove '\n'
                            if line.ends_with('\r') {
                                line.pop(); // remove '\r' for Windows CRLF
                            }
                        }

                        wscrl(window, -1);
                        mvwaddnstr(window, 0, 0, &line, getmaxx(window));
                        wrefresh(window);
                    }
                }
            }

            // Handle terminal resize
            KEY_RESIZE => {
                resize(w_debug, superwindow, window, file_path);
            }

            // Escape or 'q' to quit
            113 | 27 => {
                break;
            }
            _ => {}
        }
    }
    delwin(window);
    delwin(superwindow);
}
