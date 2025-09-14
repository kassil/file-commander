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
    let height   = scr_rows.max(3);            // clamp minimum height
    let width    = (scr_cols - scr_cols / 2).max(4); // clamp minimum width
    let startrow = 0;
    let startcol = (scr_cols / 2).max(0);      // just to be safe, nonnegative
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

fn expand_rows(window: WINDOW, line_offsets: &mut VecDeque<u64>, reader: &mut BufReader<File>) {

    let n_lines = (1 + getmaxy(window) - line_offsets.len() as i32).max(0) as usize;
    for _ in 0 .. n_lines { //line_offsets.len() .. line_offsets.len() + n_lines {
        let pos = *line_offsets.back().unwrap();
        let mut line = String::new();
        if let Ok(n_bytes) = reader.read_line(&mut line) {
            if n_bytes == 0 {
                break; // EOF
            }
            // Remove trailing newline
            rtrim(&mut line);
            // Draw the line
            mvwaddnstr(window, line_offsets.len() as i32 - 1, 0, &line, getmaxx(window));

            // mark where the next line will begin
            line_offsets.push_back(pos + n_bytes as u64);
        }
    }
}

fn contract_rows(window: WINDOW, line_offsets: &mut VecDeque<u64>) {
    // Discard rows from the bottom if needed
    line_offsets.truncate(1 + getmaxy(window) as usize);
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

fn scroll_down(w_debug: WINDOW, window: WINDOW, line_offsets: &mut VecDeque<u64>, reader: &mut BufReader<File>) {
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

fn scroll_up(w_debug: WINDOW, window: WINDOW, line_offsets: &mut VecDeque<u64>, reader: &mut BufReader<File>) {
    // Find the line before the top one
    // if line_offsets.front() and ...
    if *line_offsets.front().unwrap() > 0 && let Ok(new_pos) = find_prev_line_start(w_debug, reader, *line_offsets.front().unwrap()) {

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
    // There will be one more element representing the next line after the bottom row.
    let mut line_offsets: VecDeque<u64>= VecDeque::from([0]);

    // Load and display the visible portion
    expand_rows(window, &mut line_offsets, &mut reader);
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
                scroll_down(w_debug, window, &mut line_offsets, &mut reader);
            }

            KEY_UP => {
                scroll_up(w_debug, window, &mut line_offsets, &mut reader);
            }

            // Handle terminal resize
            KEY_RESIZE => {
                resize(w_debug, superwindow, window, &file_path);
                expand_rows(window, &mut line_offsets, &mut reader);
                contract_rows(window, &mut line_offsets);
                wrefresh(window);
                waddstr(w_debug, &format!("N:{} H: {}\n", line_offsets.len(), getmaxy(window)));
                wrefresh(w_debug);
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
