use alloc::string::String;
use core::fmt;
use crate::vga_buffer::{BUFFER_HEIGHT, BUFFER_WIDTH, ColorCode, CURSOR, EMPTY, ScreenChar, Writer};
use crate::vga_buffer::*;

impl Writer {
    /// Writes an ASCII byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    pub fn write_byte(&mut self, byte: u8) {
        let color_code = self.color_code;
        self.clean_cursor_current_position();
        match byte {
            b'\n' => {
                self.new_line()
            }
            byte => {
                if self.column_position >= BUFFER_WIDTH { self.new_line(); }
                self.shift_char_right();
                self.write_relative_sc(0, ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.move_right();
            }
        }
        self.update_cursor();
    }

    /// Writes the given ASCII string to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character. Does **not**
    /// support strings with non-ASCII characters, since they can't be printed in the VGA text
    /// mode.
    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                b'\t' => {
                    self.tab();
                }
                0x08 => { // backspace
                    self.backspace();
                }
                0x1b => { // Esc
                    self.clear_all();
                    self.column_position = 0;
                    self.row_position = 0;
                    self.update_cursor();
                }
                0x0c => { //Control-L
                    self.clear_all();
                    self.column_position = 0;
                    self.row_position = 0;
                    self.update_cursor();
                    self.write_string("MarOS:\n");
                }
                0x03 => {//Control-C
                    self.copy_line(self.row_position);
                }
                0x16 => {//Control-v
                    self.paste_line(self.row_position);
                }
                0x7f => {//canc
                    self.canc();
                }
                // not part of printable ASCII range
                _ => self.write_byte(byte),
            }
        }
    }

    /// Shifts all lines one line up and clears the last row.
    fn shift_lines_up(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn shift_char_right(&mut self) {
        let mut current = self.read_relative_sc(0);
        let mut i = self.column_position;
        while i < BUFFER_WIDTH - 1 && current != EMPTY {
            let next = self.buffer.chars[self.row_position][i + 1].read();
            self.buffer.chars[self.row_position][i + 1].write(current);
            current = next;
            i += 1;
        }
        if i == BUFFER_WIDTH - 1 {
            self._shift_char_right_rec(self.row_position + 1, current);
        }
    }
    fn _shift_char_right_rec(&mut self, row: usize, current: ScreenChar) {
        let mut current = current;
        let mut i = 0;
        while i < BUFFER_WIDTH && current != EMPTY {
            let next = self.buffer.chars[row][i].read();
            self.buffer.chars[row][i].write(current);
            current = next;
            i += 1;
        }
        if i == BUFFER_WIDTH {
            self._shift_char_right_rec(row + 1, current);
        }
    }

    /// Clears a row by overwriting it with blank characters.
    fn clear_row(&mut self, row: usize) {
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(EMPTY);
        }
    }

    pub fn clear_all(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
    }

    ///moves down the cursor if possible, otherwise shifts lines up
    fn new_line(&mut self) {
        let mut tmp = String::new();
        for i in self.column_position..BUFFER_WIDTH - 1 {
            let sc = self.buffer.chars[self.row_position][i].read();
            if sc == EMPTY { break; }
            self.buffer.chars[self.row_position][i].write(EMPTY);
            tmp.push(sc.ascii_character as char);
        }
        if self.row_position == BUFFER_HEIGHT - 1 {
            self.shift_lines_up();
        } else {
            self.row_position += 1;
            self.column_position = 0;
        }
        self.write_string(tmp.as_str());
        self.clean_cursor_current_position();
        self.column_position = 0;
    }

    fn read_relative_sc(&mut self, shift: i32) -> ScreenChar {
        let (row, col) = self.get_relative_position(shift);
        self.buffer.chars[row][col].read()
    }

    fn write_relative_sc(&mut self, shift: i32, sc: ScreenChar) {
        let (row, col) = self.get_relative_position(shift);
        self.buffer.chars[row][col].write(ScreenChar {
            ascii_character: sc.ascii_character,
            color_code: sc.color_code,
        });
    }
    /// Takes away cursor color scheme if present on the current screenchar
    fn clean_cursor_current_position(&mut self) {
        let current_sc = self.read_relative_sc(0);
        if current_sc.color_code == CURSOR.color_code { // Cursor visibility has to adapt
            self.update_color_code(self.color_code.0)
        }
    }

    fn get_relative_position(&mut self, shift: i32) -> (usize, usize) {
        if shift < 0 {
            if self.column_position != 0 { self.column_position -= 1 } else {
                self.row_position = if self.row_position != 0 { self.row_position - 1 } else { 0 };
                self.column_position = BUFFER_WIDTH - 1;
                while self.read_relative_sc(0).ascii_character == EMPTY.ascii_character && self.column_position != 0 {
                    self.column_position -= 1;
                }
                if !(self.read_relative_sc(0) == EMPTY) && self.column_position != BUFFER_WIDTH - 1 { self.column_position += 1 };
            }
            return self.get_relative_position(shift + 1);
        }
        if shift > 0 {
            if self.column_position == BUFFER_WIDTH - 1 || self.read_relative_sc(0) == EMPTY {
                self.column_position = 0;
                self.row_position = if self.row_position != BUFFER_HEIGHT - 1
                { self.row_position + 1 } else { self.row_position }
            } else {
                if self.read_relative_sc(0) != EMPTY { self.column_position += 1; }
            }
            return self.get_relative_position(shift - 1);
        }

        (self.row_position, self.column_position)
    }
    fn set_relative_position(&mut self, shift: i32) {
        (self.row_position, self.column_position) = self.get_relative_position(shift);
    }

    pub(crate) fn move_left(&mut self) {
        self.clean_cursor_current_position();
        self.set_relative_position(-1);
        self.update_cursor()
    }
    pub(crate) fn move_right(&mut self) {
        self.clean_cursor_current_position();
        self.set_relative_position(1);
        self.update_cursor();
    }
    pub(crate) fn move_down(&mut self) {
        self.clean_cursor_current_position();
        if self.row_position == BUFFER_HEIGHT - 1 {
            self.update_cursor();
            return;
        }
        self.row_position += 1;
        while self.read_relative_sc(0) == EMPTY && self.column_position > 0 {
            self.column_position -= 1;
        }
        self.update_cursor()
    }
    pub(crate) fn move_up(&mut self) {
        self.clean_cursor_current_position();
        if self.row_position == 0 {
            self.update_cursor();
            return;
        }
        self.row_position -= 1;
        while self.read_relative_sc(0) == EMPTY && self.column_position > 0 {
            self.column_position -= 1;
        }
        self.update_cursor()
    }
    fn update_char(&mut self, ascii_character: u8) {
        let sc = self.buffer.chars[self.row_position][self.column_position].read();
        self.buffer.chars[self.row_position][self.column_position].write(ScreenChar {
            ascii_character,
            color_code: sc.color_code,
        })
    }
    fn update_color_code(&mut self, color_code: u8) {
        let sc = self.buffer.chars[self.row_position][self.column_position].read();
        self.buffer.chars[self.row_position][self.column_position].write(ScreenChar {
            ascii_character: sc.ascii_character,
            color_code: ColorCode::new_from(color_code),
        })
    }
    fn update_cursor(&mut self) {
        self.update_color_code(CURSOR.color_code.0);
    }

    fn copy_line(&mut self, row: usize) {
        let mut tmp = String::new();
        for i in 0..BUFFER_WIDTH {
            let sc = self.buffer.chars[row][i].read();
            if sc == EMPTY { break; }
            let ch = sc.ascii_character as char;
            tmp.push(ch)
        }
        self.clipboard = tmp;
    }
    fn paste_line(&mut self, row: usize) {
        self.clean_cursor_current_position();
        self.clear_row(row);
        self.column_position = 0;
        let sentence = self.clipboard.clone();
        self.write_string(sentence.chars().as_str());
        self.update_cursor();
    }
    fn tab(&mut self) {
        if self.column_position == 0 && self.read_relative_sc(0).ascii_character == 0x0 {
            for _ in 0..4 {
                self.write_byte(b' ');
            }
            return;
        }
        let mut current = self.read_relative_sc(0).ascii_character;
        while current != b' ' && current != 0x0 {
            self.move_right();
            current = self.read_relative_sc(0).ascii_character;
        }
        self.move_right()
    }
    fn backspace(&mut self) {
        self.clean_cursor_current_position();
        let mut tmp_str = String::new();
        let mut going_up: bool = false;
        if self.column_position == 0 && self.row_position != 0 {
            going_up = true;
            for i in 0..BUFFER_WIDTH {
                let sc = self.buffer.chars[self.row_position][i].read();
                if sc == EMPTY { break; }
                tmp_str.push(sc.ascii_character as char);
            }
        }
        self.move_left();
        if !going_up {
            for i in self.column_position..BUFFER_WIDTH - 1 {
                let nc = self.buffer.chars[self.row_position][i + 1].read().ascii_character;
                self.buffer.chars[self.row_position][i].write(ScreenChar {
                    ascii_character: nc,
                    color_code: self.color_code,
                })
            }
        } else {
            self.clear_row(self.row_position + 1);
            let (prev_col, prev_row) = (self.column_position, self.row_position);
            for ch in tmp_str.chars() {
                self.write_byte(ch as u8)
            }
            self.clean_cursor_current_position();
            (self.column_position, self.row_position) = (prev_col, prev_row );
        }
        self.buffer.chars[self.row_position][BUFFER_WIDTH - 1].write(EMPTY);
        self.update_cursor();
    }
    fn canc(&mut self) {
        self.clean_cursor_current_position();
        self.write_relative_sc(0, EMPTY);
        for i in self.column_position..BUFFER_WIDTH - 1 {
            let nc = self.buffer.chars[self.row_position][i + 1].read().ascii_character;
            self.buffer.chars[self.row_position][i].write(ScreenChar {
                ascii_character: nc,
                color_code: self.color_code,
            })
        }
        self.buffer.chars[self.row_position][BUFFER_WIDTH - 1].write(EMPTY);
        self.update_cursor();
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}
