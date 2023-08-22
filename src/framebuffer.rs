use crate::mailbox;
use ascii;
use crate::mailbox::tags::{
    FBAllocateBufferRequest, FBSetBitsPerPixelRequest, FBSetPhysicalSizeRequest,
    FBSetVirtualSizeRequest, TagInterfaceRequest,
};
use core::{fmt, num::NonZeroU8};
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use spin::{Mutex, Once};
use static_assertions::assert_eq_size;

const PREFERRED_WIDTH: usize = 640;
const PREFERRED_HEIGHT: usize = 480;
const MONO_TEXT_WIDTH: u32 = 6;
const MONO_TEXT_HEIGHT: u32 = 10;
const TEXT_BUFFER_LEN: usize =
    (PREFERRED_WIDTH / MONO_TEXT_WIDTH as usize) * (PREFERRED_HEIGHT / MONO_TEXT_HEIGHT as usize);

pub struct FrameBuffer(DisplayMode);

pub struct BufferData {
    buffer: BufferPtr,
    buff_size: usize,
    dims: Size,
}
struct BufferPtr(*mut FBPixel);
unsafe impl Send for BufferPtr {}
/// Describes some position on the display
#[derive(Clone, Copy, Debug)]
struct ScreenPos(u32, u32);

impl BufferData {
    /// Converts screen position to index in a linear buffer.
    ///
    /// Assumes 1 buffer index per screen position.
    /// Make sure to multiply if that does not hold.
    fn pos_to_idx(&self, ScreenPos(x, y): ScreenPos) -> usize {
        (x + y * self.dims.width) as usize
    }
}

enum DisplayMode {
    // Append text to show on screen.
    // If too big the oldest text is removed.
    // Writing to screen is done immediately; nothing is deferred.
    TextLog(TextLogData),
    Graphical,
}

impl FrameBuffer {
    pub fn buff_data(&self) -> &BufferData {
        match &self.0 {
            DisplayMode::TextLog(TextLogData { ref data, .. }) => data,
            _ => unimplemented!()
        }
    }

    pub fn buff_data_mut(&mut self) -> &mut BufferData {
        match &mut self.0 {
            DisplayMode::TextLog(TextLogData { ref mut data, .. }) => data,
            _ => unimplemented!()
        }
    }
}

pub struct TextLogData {
    data: BufferData,
    text: [AsciiChar; TEXT_BUFFER_LEN],
    // NOTE: Cursor is allowed to be past the end-of-line
    // Contract: 0 <= cursor.0 <= data.dims.width
    // Contract: 0 <= cursor.1 < data.dims.height
    cursor: TextPos,
}

/// Describes some position in the text log
#[derive(Clone, Copy, Debug)]
struct TextPos(u32, u32);

impl TextLogData {
    /// Check whether the cursor needs to be moved to a new line
    fn text_shift_required(&self) -> bool {
        self.cursor.0 == self.chars_width() &&
            self.cursor.1 == self.chars_height() - 1
    }

    /// Moves the cursor to a new line.
    /// Does this by shifting all text up
    /// Redraws the whole screen to handle this
    fn shift_text(&mut self) {
        let chars_width = self.chars_width() as usize;
        self.cursor.0 = 0;
        self.text[0..(chars_width)].fill(AsciiChar(None));
        // TODO: Check that this is correct
        self.text.rotate_left(chars_width);
        self.redraw_text();
    }

    /// Redraws entire screen
    fn redraw_text(&mut self) {
        for y in 0..(self.chars_height()) {
            let mut erase_rest = false;
            for x in 0..(self.chars_width()) {
                let text_pos = self.pos_to_idx(TextPos(x, y));
                let screen_pos = Self::text_pos_to_screen_pos(TextPos(x, y));

                let mut c = self.text[text_pos];
                if c == '\n'.into() {
                    erase_rest = true;
                }
                if erase_rest {
                    c = AsciiChar(None);
                }

                self.paint_char(c, screen_pos);
            }
        }
    }

    /// Advance the cursor to the next valid position
    fn advance_cursor(&mut self) {
        if self.cursor.0 < self.chars_width() {
        self.cursor.0 += 1;
        }

        if self.cursor.0 >= self.chars_width() {
            // If we have room.
            if self.cursor.1 + 1 < self.chars_height() {
                self.cursor.0 = 0;
                self.cursor.1 += 1;
            }
        }
    }

    fn advance_cursor_newline(&mut self) {
        if self.cursor.1 + 1 < self.chars_height() {
            self.cursor.0 = 0;
            self.cursor.1 += 1;
        } else {
            self.shift_text();
        }
    }

    /// How many characters can be rendered per line
    fn chars_width(&self) -> u32 {
        self.data.dims.width / MONO_TEXT_WIDTH as u32
    }
    /// How many lines of text can fit
    fn chars_height(&self) -> u32 {
        self.data.dims.height / MONO_TEXT_HEIGHT as u32
    }

    /// Converts text position to index in the text buffer.
    fn pos_to_idx(&self, TextPos(x, y): TextPos) -> usize {
        (x + y * self.chars_width()) as usize
    }
    /// Convert text-space to screen-space
    fn text_pos_to_screen_pos(pos: TextPos) -> ScreenPos {
        let x = pos.0 * MONO_TEXT_WIDTH;
        let y = pos.1 * MONO_TEXT_HEIGHT;
        ScreenPos(x, y)
    }

    fn write_char(&mut self, c: AsciiChar) {
        // match c.0.map(|c| c.get()) {
        //     Some(c) => println!("Writing {} ({:?}) to framebuffer @{:?}", c as char, c, self.cursor),
        //     None => println!("Writing <NONE> to framebuffer"),
        // }
        // crate::time::wait_microsec(5_000);

        if self.text_shift_required() {
            self.shift_text();
        }
        self.write_char_to_pos(c, self.cursor);


        if c == '\n'.into() {
            self.advance_cursor_newline();
        } else {
            self.advance_cursor();
        }
    }

    fn paint_char(&mut self, c: AsciiChar, screen_pos: ScreenPos) {
        use embedded_graphics::{
            primitives::{
                PrimitiveStyle,
                rectangle::Rectangle,
            },
            mono_font::{ascii::FONT_6X10, MonoTextStyle},
            text::{Baseline, Text},
        };

        let fill = PrimitiveStyle::with_fill(Rgb888::BLACK);
        // println!("Filling {:?} with BG", screen_pos);

        Rectangle::new(Point::new(screen_pos.0 as i32, screen_pos.1 as i32), Size::new(MONO_TEXT_WIDTH, MONO_TEXT_HEIGHT))
            .into_styled(fill)
            .draw(&mut self.data)
            .unwrap();

        let c = c.0.filter(|c| ascii::AsciiChar::from_ascii(c.get()).is_ok_and(|asci| asci.is_ascii_printable()));
        if let Some(c) = c.map(|c| c.get()) {
            let style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);

            let str_buf = [c];
            let s = core::str::from_utf8(&str_buf).expect("Tried to convert an invalid char to utf8");
            // println!("Paintin {} to {:?}", s, screen_pos);
            Text::with_baseline(s, Point::new(screen_pos.0 as i32, screen_pos.1 as i32), style, Baseline::Top)
                .draw(&mut self.data)
                .unwrap();
        };
    }

    fn write_char_to_pos(&mut self, c: AsciiChar, text_pos: TextPos) {
        self.text[self.pos_to_idx(text_pos)] = c;

        let screen_pos = Self::text_pos_to_screen_pos(text_pos);
        self.paint_char(c, screen_pos);
    }
}

impl fmt::Write for FrameBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let DisplayMode::TextLog(ref mut text_log) = self.0 {
            for c in s.chars() {
                text_log.write_char(c.into());
            }
            return Ok(());
        }

        return Err(fmt::Error);
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        if let DisplayMode::TextLog(ref mut text_log) = self.0 {
            text_log.write_char(c.into());
            return Ok(());
        }

        return Err(fmt::Error);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct AsciiChar(Option<NonZeroU8>);
assert_eq_size!(AsciiChar, u8); // Make sure the compiler finds the '0' hole for 'None'
impl core::convert::From<char> for AsciiChar {
    fn from(val: char) -> Self {
        if !val.is_ascii() {
            return AsciiChar(None);
        }
        AsciiChar(NonZeroU8::new(val as u8))
    }
}

impl core::ops::Index<ScreenPos> for BufferData {
    type Output = FBPixel;

    // Safety: Checks that coordinates are inside the buffer
    fn index(&self, pos: ScreenPos) -> &Self::Output {
        if (0..self.dims.width).contains(&pos.0) && (0..self.dims.height).contains(&pos.1) {
            let index = self.pos_to_idx(pos);
            unsafe { &*(self.buffer.0.add(index as usize)) }
        } else {
            panic!(
                "FrameBuffer::Index out of bounds. {:?} is outside of {:?}",
                pos,
                self.dims
            )
        }
    }
}
impl core::ops::IndexMut<ScreenPos> for BufferData {
    // Safety: Checks that coordinates are inside the buffer
    fn index_mut(&mut self, pos: ScreenPos) -> &mut Self::Output {
        if (0..self.dims.width).contains(&pos.0) && (0..self.dims.height).contains(&pos.1) {
            let index = self.pos_to_idx(pos);
            unsafe { &mut *(self.buffer.0.add(index as usize)) }
        } else {
            panic!(
                "FrameBuffer::Index out of bounds. {:?} is outside of {:?}",
                pos,
                self.dims
            )
        }
    }
}

impl OriginDimensions for BufferData {
    fn size(&self) -> Size {
        self.dims
    }
}

impl DrawTarget for BufferData {
    type Color = Rgb888;
    // Since we just write to the framebuffer we have no failure points
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // Check if the pixel coordinates are out of bounds (negative or greater than
            // (width, height)). `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            if let Ok((x, y)) = coord.try_into() {
                if (0..self.dims.width).contains(&x) && (0..self.dims.height).contains(&y) {
                    self[ScreenPos(x, y)] = FBPixel([color.r(), color.g(), color.b()]);
                }
            }
        }
        Ok(())
    }
}

static FRAMEBUFFER: Once<Mutex<FrameBuffer>> = Once::new();

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FBPixel([u8; 3]);

pub fn try_get() -> Option<spin::MutexGuard<'static, FrameBuffer>> {
    FRAMEBUFFER.get().and_then(|m| m.try_lock())
}

pub fn get() -> spin::MutexGuard<'static, FrameBuffer> {
    FRAMEBUFFER.get().unwrap().lock()
}


pub unsafe fn init() -> Result<(), &'static str> {
    let mut mbox = mailbox::get();

    let res = mbox
        .send_and_poll_recieve_batch((
            FBSetPhysicalSizeRequest {
                width: 640,
                height: 480,
            }
            .into_tag(),
            FBSetVirtualSizeRequest {
                width: 640,
                height: 480,
            }
            .into_tag(),
            FBSetBitsPerPixelRequest {
                bpp: core::mem::size_of::<FBPixel>() as u32 * 8,
            }
            .into_tag(),
            FBAllocateBufferRequest { alignment: 16 }.into_tag(),
        ))
        .map_err(|_| "Batch framebuffer init failed")?;

    let virt_res = res
        .1
        .ok_or("Framebuffer virt size request did not get a response")?;
    let height = virt_res.height;
    let width = virt_res.width;

    let res = res
        .3
        .ok_or("FameBuffer buff allor request did not get a response")?;

    let ptr = res.base_address as *mut u32 as *mut FBPixel;
    let size = res.size / 3;
    for i in 0..size {
        unsafe {
            (*ptr.add(i as usize)).0[0] = u8::MAX;
        }
    }

    let tld = TextLogData {
        data: BufferData {
            buffer: BufferPtr(res.base_address as *mut u32 as *mut FBPixel),
            buff_size: res.size as usize,
            dims: Size { width, height },
        },
        text: [AsciiChar(None); TEXT_BUFFER_LEN],
        cursor: TextPos(0, 0),
    };
    let fb = FrameBuffer(DisplayMode::TextLog(tld));
    FRAMEBUFFER.call_once(|| Mutex::new(fb));

    Ok(())
}
