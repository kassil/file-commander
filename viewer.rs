// View the contents of a file as text.
// Navigate with arrow keys and Page Up/Down.
// Press Esc to close the window.

use ncurses::*;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Skip exactly one line from the current position.
/// Returns how many bytes were skipped.
fn skip_line(file: &mut File) -> io::Result<u64> {
    let mut buf = [0u8; 1];
    let mut skipped = 0;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break; // EOF
        }
        skipped += 1;
        if buf[0] == b'\n' {
            break; // end of line
        }
    }
    Ok(skipped)
}

/// Read up to `max_lines` starting from current position.
fn read_lines_from(file: &mut File, max_lines: usize, width: usize) -> io::Result<Vec<String>> {
    let mut reader = BufReader::new(file.try_clone()?);
    let mut lines = Vec::new();
    let mut line = String::new();
    while lines.len() < max_lines {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break; // EOF
        }
        // truncate if longer than screen width
        if line.len() > width {
            line.truncate(width);
        }
        lines.push(line.trim_end_matches('\n').to_string());
    }
    Ok(lines)
}

fn find_prev_line_start(w_debug: WINDOW, file: &mut File, file_pos: u64) -> std::io::Result<u64> {
    if file_pos == 0 {
        // Already at start of file
        waddstr(w_debug, &format!("find_prev_line_start already at beginning\n"));
        return Ok(0);
    }

    // Choose how far back to step (at most 4096 or to start of file)
    // -1 to avoid re-reading current line's newline
    let backstep = 4096.min((file_pos - 1) as usize) as u64;
    let seek_pos = file_pos - backstep - 1; 
    file.seek(SeekFrom::Start(seek_pos))?;

    let mut buf = vec![0u8; backstep as usize];
    let n = file.read(&mut buf)?;
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

pub fn view_file_modal(w_debug: WINDOW, file_path: &Path) {
    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            waddstr(
                w_debug,
                &format!("Error opening file {}: {}\n", file_path.display(), e),
            );
            return;
        }
    };

    let mut height = 0;
    let mut width = 0;
    getmaxyx(w_debug, &mut height, &mut width);

    let superwindow = newwin(height, width, 0, width);
    let window = newwin(height-2, width-2, 1, width+1);

    // Box around window
    box_(superwindow, 0, 0);
    // Title with filename
    mvwaddstr(superwindow, 0, 2, &format!(" {} ", file_path.display()));
    // Instructions
    mvwaddstr(superwindow, height-1, 2, "Up/Down to scroll, Esc or 'q' to close");
    wrefresh(superwindow);

    let mut file_pos: u64 = 0; // where the top of screen begins
    keypad(window, true);

    loop {
        wrefresh(w_debug); // Draw debug window below dialog
        werase(window);

        // Position file and read visible window
        file.seek(SeekFrom::Start(file_pos)).unwrap();
        let lines = read_lines_from(&mut file, height as usize, width as usize).unwrap();

        for (i, line) in lines.iter().enumerate() {
            mvwaddstr(window, i as i32, 0, line);
        }
        wrefresh(window);

        match wgetch(window) {
            KEY_DOWN => {
                // TODO Stop before document scrolls off screen!
                file.seek(SeekFrom::Start(file_pos)).unwrap();
                if let Ok(n) = skip_line(&mut file) {
                    if n > 0 {
                        waddstr(w_debug, &format!("KDOWN {} + {} = {} \n", file_pos, n, file_pos + n));
                        file_pos += n;
                    }
                }
            }
            KEY_UP => {
                if let new_pos = find_prev_line_start(w_debug, &mut file, file_pos).unwrap() {
                    waddstr(w_debug, &format!("KUP {} - {} = {}\n", file_pos, file_pos - new_pos, new_pos));
                    file_pos = new_pos;
                }
            }

            // Handle terminal resize
            KEY_RESIZE => {
                let scr_rows = getmaxy(stdscr());
                let scr_cols = getmaxx(stdscr());
                //werase(w_debug);
                wresize(w_debug, scr_rows, scr_cols/2);

                wresize(superwindow, scr_rows, scr_cols - scr_cols/2);
                mvwin(superwindow, 0, scr_cols/2);

                wresize(window, scr_rows-2, scr_cols - scr_cols/2 - 2);
                mvwin(window, 1, scr_cols/2 + 1);

                // Redraw border and title
                box_(superwindow, 0, 0);
                mvwaddstr(superwindow, 0, 2, &format!(" {} ", file_path.display()));
                mvwaddstr(superwindow, scr_rows-1, 2, "Up/Down to scroll, Esc or 'q' to close");
                wrefresh(superwindow);
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
