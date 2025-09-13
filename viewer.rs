// View the contents of a file as text.
// Navigate with arrow keys and Page Up/Down.
// Press Esc to close the window.

use ncurses::*;
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

    let mut reader = BufReader::new(file);

    let (height, width, startrow, startcol) = calc_extents();
    let superwindow = newwin(height, width, 0, width);
    let window = newwin(height-2, width-2, 1, width+1);
    scrollok(window, true);

    // Box around window
    box_(superwindow, 0, 0);
    // Title with filename
    mvwaddstr(superwindow, 0, 2, &format!(" {} ", file_path.display()));
    // Instructions
    mvwaddstr(superwindow, height-1, 2, "Up/Down to scroll, Esc or 'q' to close");
    wrefresh(superwindow);

    let mut top_file_pos: u64 = 0; // where the top of screen begins
    let mut bot_file_pos: u64 = 0; // after the bottom of screen
    keypad(window, true);

    for i in 0 .. getmaxy(window) {
        let mut line = String::new();
        if let Ok(n_bytes) = reader.read_line(&mut line) {

            if n_bytes == 0 {
                break; // EOF
            }
            bot_file_pos += n_bytes as u64;
            rtrim(&mut line);
            mvwaddnstr(window, i as i32, 0, &line, getmaxx(window));
        }
    }
    wrefresh(window);
    waddstr(w_debug, &format!("OPEN top:{} bot:{}\n", top_file_pos, bot_file_pos));

    loop {
        wrefresh(w_debug); // Draw debug window below dialog

        match wgetch(window) {
            KEY_DOWN => {
                // Advance top row
                // TODO Maybe keep the positions of the visible lines?
                reader.seek(SeekFrom::Start(top_file_pos)).unwrap();
                top_file_pos += reader.skip_until(b'\n').unwrap() as u64;

                // Read a line
                reader.seek(SeekFrom::Start(bot_file_pos)).unwrap();
                let mut line = String::new();
                let result = reader.read_line(&mut line).unwrap();
                //bot_file_pos = reader.stream_position().unwrap() as u64;
                bot_file_pos += result as u64;

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

                waddstr(w_debug, &format!("KDOWN top:{} bot:{}\n", top_file_pos, bot_file_pos));
            }

            KEY_UP => {
                // Find the line before the top one
                if top_file_pos > 0 && let Ok(new_pos) = find_prev_line_start(w_debug, &mut reader, top_file_pos) {

                    waddstr(w_debug, &format!("KUP line1_start:{} line1_len:{} line2_start:{} lineN_start:{}\n", new_pos, top_file_pos - new_pos, top_file_pos, bot_file_pos));
                    top_file_pos = new_pos;
                    reader.seek(SeekFrom::Start(top_file_pos));
                    // Read one new line at top
                    let mut line = String::new();
                    if let Ok(n) = reader.read_line(&mut line) {

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

                    // Find the line before the bottom one
                    if let Ok(new_pos) = find_prev_line_start(w_debug, &mut reader, bot_file_pos) {
                        bot_file_pos = new_pos;
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
