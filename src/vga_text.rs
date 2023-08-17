use bitfield_struct::bitfield;
use spin::{
    Lazy,
    Mutex,
};
use num_enum::TryFromPrimitive;
use volatile::VolatileRef;
use core::{
    fmt,
    fmt::{
        Arguments,
        Write,
    },
};

const BUFFER_ADDR: usize = 0xb8000;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
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
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(bits: u8) -> Self {
        use Color::*;
        match bits {
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
            _ => Black,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct CharEntry {
    chr: u8,
    data: CharData,
}

#[bitfield(u8)]
struct CharData {
    #[bits(4, default=Color::White)]
    fg_color: Color,
    #[bits(3)]
    bg_color: Color,
    blink: bool

}

#[repr(transparent)]
struct Buffer([[CharEntry; WIDTH]; HEIGHT]);

#[derive(Copy, Clone, Default)]
pub struct Cursor {
    row: usize,
    col: usize,
}
pub struct Writer {
    buffer: &'static mut Buffer,
    cursor_pos: Cursor,
}

impl Writer {
    pub fn new_line(&mut self) {
        self.cursor_pos.row = 0;
        self.cursor_pos.col += 1;
        if self.cursor_pos.col >= HEIGHT {
            self.cursor_pos.col = 0;
        }
    }
    pub fn advance_cursor(&mut self) {
        self.cursor_pos.row += 1;
        if self.cursor_pos.row >= WIDTH {
            self.new_line();
        }
    }
}

impl Write for Writer {
   fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
       for c in s.chars() {
           self.write_char(c)?;
       }
       Ok(())
   }

   fn write_char(&mut self, c: char) -> Result<(), fmt::Error> {
       let char_addr = &mut self.buffer.0[self.cursor_pos.col][self.cursor_pos.row];
       let mut char_ref = VolatileRef::from_mut_ref(char_addr).write_only();
       let data = CharData::new();
       if c == '\n' {
           self.new_line();
           return Ok(());
       }
       let chr = if c as usize >= u8::MAX as usize {
           '~' as u8
       } else {
           c as u8
       };

       char_ref.as_mut_ptr().write(CharEntry {
           chr,
           data
       });

       self.advance_cursor();
       Ok(())
   }
}

unsafe fn buffer_ref() -> &'static mut Buffer {
    &mut *(BUFFER_ADDR as *mut Buffer)
}

pub static WRITER: Lazy<Mutex<Writer>> = Lazy::new(|| {
    Mutex::new(
        Writer {
            buffer: unsafe { buffer_ref() },
            cursor_pos: Cursor { row: 0, col: 0 },
        }
    )
});

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_text::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: Arguments) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}


mod tests {
    use super::*;

    #[test_case]
    fn char_entry_is_u16() {
        assert_eq!(core::mem::size_of::<CharEntry>(), 2);
        assert_eq!(core::mem::size_of::<CharData>(), 1);
    }

    #[test_case]
    fn char_entry_can_set_and_get() {
        let mut ce = CharData::new();
        assert_eq!(ce.fg_color(), Color::White);
        assert_eq!(ce.bg_color(), Color::Black);
        assert_eq!(ce.blink(), false);

        ce.set_fg_color(Color::Blue);
        ce.set_bg_color(Color::Magenta);
        ce.set_blink(true);

        assert_eq!(ce.fg_color(), Color::Blue);
        assert_eq!(ce.bg_color(), Color::Magenta);
        assert_eq!(ce.blink(), true);

        ce.set_fg_color(Color::LightRed);
        assert_eq!(ce.fg_color(), Color::LightRed);
        assert_eq!(ce.bg_color(), Color::Magenta);
    }
}

