use crate::mailbox;
use crate::mailbox::tags::{
    BoardModelRequest, FBAllocateBufferRequest, FBSetBitsPerPixelRequest, FBSetPhysicalSizeRequest,
    FBSetVirtualSizeRequest, TagInterfaceRequest,
};
use core::{fmt, num::NonZeroU8};
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use spin::{Mutex, Once};
use static_assertions::assert_eq_size;

const PREFERRED_WIDTH: usize = 640;
const PREFERRED_HEIGHT: usize = 480;
const MONO_TEXT_WIDTH: usize = 6;
const MONO_TEXT_HEIGHT: usize = 10;
const TEXT_BUFFER_LEN: usize =
    (PREFERRED_WIDTH / MONO_TEXT_WIDTH) * (PREFERRED_HEIGHT / MONO_TEXT_HEIGHT);

pub struct FrameBuffer(DisplayMode);

pub struct BufferData {
    buffer: BufferPtr,
    buff_size: usize,
    dims: Size,
}
struct BufferPtr(*mut FBPixel);
unsafe impl Send for BufferPtr {}

enum DisplayMode {
    // Append text to show on screen.
    // If too big the oldest text is removed.
    TextLog(TextLogData),
}

impl FrameBuffer {
    pub fn buff_data(&self) -> &BufferData {
        match &self.0 {
            DisplayMode::TextLog(TextLogData { ref data, .. }) => data,
        }
    }

    pub fn buff_data_mut(&mut self) -> &mut BufferData {
        match &mut self.0 {
            DisplayMode::TextLog(TextLogData { ref mut data, .. }) => data,
        }
    }
}

pub struct TextLogData {
    data: BufferData,
    text: [AsciiChar; TEXT_BUFFER_LEN],
    cursor: (usize, usize),
}

impl FrameBuffer {
    fn write_char_impl(&mut self, c: AsciiChar) {
        todo!()
    }
}

impl fmt::Write for FrameBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if !matches!(self.0, DisplayMode::TextLog(_)) {
            return Err(fmt::Error);
        }
        for c in s.chars() {
            self.write_char_impl(c.into());
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        if !matches!(self.0, DisplayMode::TextLog(_)) {
            return Err(fmt::Error);
        }

        self.write_char_impl(c.into());
        Ok(())
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

impl core::ops::Index<(u32, u32)> for BufferData {
    type Output = FBPixel;

    // Safety: Checks that coordinates are inside the buffer
    fn index(&self, (x, y): (u32, u32)) -> &Self::Output {
        if (0..self.dims.width).contains(&x) && (0..self.dims.height).contains(&y) {
            let index = x + y * self.dims.width;
            unsafe { &*(self.buffer.0.add(index as usize)) }
        } else {
            panic!(
                "FrameBuffer::Index out of bounds. {:?} is outside of {:?}",
                (x, y),
                self.dims
            )
        }
    }
}
impl core::ops::IndexMut<(u32, u32)> for BufferData {
    // Safety: Checks that coordinates are inside the buffer
    fn index_mut(&mut self, (x, y): (u32, u32)) -> &mut Self::Output {
        if (0..self.dims.width).contains(&x) && (0..self.dims.height).contains(&y) {
            let index = x + y * self.dims.width;
            unsafe { &mut *(self.buffer.0.add(index as usize)) }
        } else {
            panic!(
                "FrameBuffer::Index out of bounds. {:?} is outside of {:?}",
                (x, y),
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
                    self[(x, y)] = FBPixel([color.r(), color.g(), color.b()]);
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

    println!("Responses: {:#?}", res);
    let res = res
        .3
        .ok_or("FameBuffer buff allor request did not get a response")?;

    let ptr = res.base_address as *mut u32 as *mut FBPixel;
    println!("================ MODULO = {}", res.size % 3);
    let size = res.size / 3;
    for i in 0..size {
        unsafe {
            (*ptr.add(i as usize)).0[0] = u8::MAX;
        }
    }

    let fb = FrameBuffer(DisplayMode::TextLog(TextLogData {
        data: BufferData {
            buffer: BufferPtr(res.base_address as *mut u32 as *mut FBPixel),
            buff_size: res.size as usize,
            dims: Size { width, height },
        },
        text: [AsciiChar(None); TEXT_BUFFER_LEN],
        cursor: (0, 0),
    }));
    FRAMEBUFFER.call_once(|| Mutex::new(fb));

    Ok(())
}

pub fn draw_text(text: &str) {
    use embedded_graphics::{
        mono_font::{ascii::FONT_6X10, MonoTextStyle},
        text::Text,
    };

    let style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);
    Text::new(text, Point::new(20, 30), style)
        .draw(get().buff_data_mut())
        .unwrap();
}
