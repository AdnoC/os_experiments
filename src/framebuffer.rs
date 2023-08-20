use spin::{Once, Mutex};
use embedded_graphics::{
    prelude::*,
    pixelcolor::Rgb888,
};
use crate::mailbox;
use crate::mailbox::tags::{
    BoardModelRequest,
    FBSetPhysicalSizeRequest,
    FBSetVirtualSizeRequest,
    FBSetBitsPerPixelRequest,
    FBAllocateBufferRequest,
    TagInterfaceRequest
};

pub struct FrameBuffer {
    buffer: BufferPtr,
    buff_size: usize,
    dims: Size,
}
struct BufferPtr(*mut FBPixel);
unsafe impl Send for BufferPtr {}

impl core::ops::Index<(u32, u32)> for FrameBuffer {
    type Output = FBPixel;

    // Safety: Checks that coordinates are inside the buffer
    fn index(&self, (x, y): (u32, u32)) -> &Self::Output {
        if (0..self.dims.width).contains(&x) && (0..self.dims.height).contains(&y) {
            let index = x + y * self.dims.width;
            unsafe {
                &*(self.buffer.0.add(index as usize))
            }
        } else {
            panic!("FrameBuffer::Index out of bounds. {:?} is outside of {:?}", (x, y), self.dims)
        }
    }
}
impl core::ops::IndexMut<(u32, u32)> for FrameBuffer {
    // Safety: Checks that coordinates are inside the buffer
    fn index_mut(&mut self, (x, y): (u32, u32)) -> &mut Self::Output {
        if (0..self.dims.width).contains(&x) && (0..self.dims.height).contains(&y) {
            let index = x + y * self.dims.width;
            unsafe {
                &mut *(self.buffer.0.add(index as usize))
            }
        } else {
            panic!("FrameBuffer::Index out of bounds. {:?} is outside of {:?}", (x, y), self.dims)
        }
    }
}


impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        self.dims
    }
}

impl DrawTarget for FrameBuffer {
    type Color = Rgb888;
    // Since we just write to the framebuffer we have no failure points
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error> where
        I: IntoIterator<Item = Pixel<Self::Color>> {
        for Pixel(coord, color) in pixels.into_iter() {
            // Check if the pixel coordinates are out of bounds (negative or greater than
            // (width, height)). `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            if let Ok((x , y )) = coord.try_into() {
                if (0..self.dims.width).contains(&x) && (0..self.dims.height).contains(&y) {
                    self[(x, y)] = FBPixel([ color.r(), color.g(), color.b() ]);
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

    let res = mbox.send_and_poll_recieve_batch((
            FBSetPhysicalSizeRequest { width: 640, height: 480 }.into_tag(),
            FBSetVirtualSizeRequest { width: 640, height: 480 }.into_tag(),
            FBSetBitsPerPixelRequest { bpp: core::mem::size_of::<FBPixel>() as u32 * 8}.into_tag(),
            FBAllocateBufferRequest { alignment: 16}.into_tag(),
    )).map_err(|_| "Batch framebuffer init failed")?;


    let virt_res = res.1.ok_or("Framebuffer virt size request did not get a response")?;
    let height = virt_res.height;
    let width = virt_res.width;

    println!("Responses: {:#?}", res);
    let res = res.3.ok_or("FameBuffer buff allor request did not get a response")?;

    let ptr = res.base_address as *mut u32 as *mut FBPixel;
    println!("================ MODULO = {}", res.size % 3);
    let size = res.size / 3;
    for i in 0..size {
        unsafe {

            (*ptr.add(i as usize)).0[0] = u8::MAX;
        }
    }


    FRAMEBUFFER.call_once(|| Mutex::new(FrameBuffer {
        buffer: BufferPtr(res.base_address as *mut u32 as *mut FBPixel),
        buff_size: res.size as usize,
        dims: Size { width, height },
    }));

    Ok(())
}

pub fn draw_text(text: &str) {
    use embedded_graphics::{
        mono_font::{ascii::FONT_6X10, MonoTextStyle},
        text::Text,
    };

    let style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);
    Text::new(text, Point::new(20, 30), style).draw(&mut *get()).unwrap();
}

