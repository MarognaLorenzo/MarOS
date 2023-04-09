use alloc::string::String;
use core::fmt;
use core::ops::{Deref, DerefMut};
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::interrupts::without_interrupts;
use crate::vga_buffer::Color::{Black, Blue, Brown, Cyan, DarkGray, Green, LightBlue, LightCyan, LightGray, LightGreen, LightRed, Magenta, Pink, Red, White, Yellow};

lazy_static! {
    /// A global `Writer` instance that can be used for printing to the VGA text buffer.
    ///
    /// Used by the `print!` and `println!` macros.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        row_position: 0,
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        clipboard: String::new()
    });
}

const CURSOR: ScreenChar = ScreenChar { ascii_character: 0, color_code: ColorCode::new(Black, LightCyan) };
const EMPTY: ScreenChar = ScreenChar { ascii_character: 0, color_code: ColorCode::new(White, Black) };

/// The standard color palette in VGA text mode.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

impl Color {
    pub fn from(n: u8) -> Color {
        match n {
            0 => Black,
            1 => Blue,
            2 => Green,
            3 => Cyan,
            4 => Red,
            5 => Magenta,
            6 => Brown,
            7 => LightGray,
            8 => DarkGray,
            9 => LightBlue,
            10 => LightGreen,
            11 => LightCyan,
            12 => LightRed,
            13 => Pink,
            14 => Yellow,
            15 => White,
            _ => Brown
        }
    }
}

/// A combination of a foreground and a background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a new `ColorCode` with the given foreground and background colors.
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
    pub fn new_from(code: u8) -> ColorCode {
        ColorCode(code)
    }
}

/// A screen character in the VGA text buffer, consisting of an ASCII character and a `ColorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

impl Deref for ScreenChar {
    type Target = ScreenChar ;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl DerefMut for ScreenChar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

/// The height of the text buffer (normally 25 lines).
const BUFFER_HEIGHT: usize = 25;
/// The width of the text buffer (normally 80 columns).
const BUFFER_WIDTH: usize = 80;

/// A structure representing the VGA text buffer.
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// A writer type that allows writing ASCII bytes and strings to an underlying `Buffer`.
///
/// Wraps lines at `BUFFER_WIDTH`. Supports newline characters and implements the
/// `core::fmt::Write` trait.
pub struct Writer {
    column_position: usize,
    row_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
    clipboard: String
}

impl Writer {
    /// Writes an ASCII byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    pub fn write_byte(&mut self, byte: u8) {
        let color_code = self.color_code;
        match byte {
            b'\n' => {
                self.new_line()
            }
            byte => {
                if self.column_position >= BUFFER_WIDTH { self.new_line(); }
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
                0x08 => { // backspace
                    self.clean_cursor_current_position();
                    self.move_left();
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
                0x1b => {
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
                0x03 =>{//Control-C
                    let mut tmp = String::new();
                    for i in 0..BUFFER_WIDTH{
                        let sc = self.buffer.chars[self.row_position][i].read();
                        if sc == EMPTY { break; }
                        let ch = sc.ascii_character as char;
                        tmp.push(ch)
                    }
                    self.clipboard = tmp;
                }
                0x16 => {//Control-v
                    self.clean_cursor_current_position();
                    self.clear_row(self.row_position);
                    self.column_position=0;
                    let sentence = self.clipboard.clone();
                    self.write_string(sentence.chars().as_str());
                    self.update_cursor();
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

    /// Clears a row by overwriting it with blank characters.
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: 0x0,
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    pub fn clear_all(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
    }

    ///moves down the cursor if possible, otherwise shifts lines up
    fn new_line(&mut self) {
        self.clean_cursor_current_position();
        if self.row_position == BUFFER_HEIGHT - 1 {
            self.shift_lines_up()
        } else {
            self.row_position += 1;
            self.column_position = 0;
        }
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
    /// Writes empty Screen char in current position if there is a CURSOR on it
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
                if !(self.read_relative_sc(0) == EMPTY) { self.column_position += 1 };
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

    pub fn move_left(&mut self) {
        self.clean_cursor_current_position();
        self.set_relative_position(-1);
        self.update_cursor()
    }
    pub fn move_right(&mut self) {
        self.clean_cursor_current_position();
        self.set_relative_position(1);
        self.update_cursor();
    }
    pub fn move_down(&mut self) {
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
    pub fn move_up(&mut self){
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
    pub fn update_char(&mut self, ascii_character: u8) {
        let sc = self.buffer.chars[self.row_position][self.column_position].read();
        self.buffer.chars[self.row_position][self.column_position].write(ScreenChar {
            ascii_character,
            color_code: sc.color_code,
        })
    }
    pub fn update_color_code(&mut self, color_code: u8) {
        let sc = self.buffer.chars[self.row_position][self.column_position].read();
        self.buffer.chars[self.row_position][self.column_position].write(ScreenChar {
            ascii_character: sc.ascii_character,
            color_code: ColorCode::new_from(color_code),
        })
    }
    pub fn update_cursor(&mut self) {
        self.update_color_code(CURSOR.color_code.0);
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Like the `print!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}


#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    let s = "Some test string that fits on a single line";
    without_interrupts(|| {
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writing failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}
