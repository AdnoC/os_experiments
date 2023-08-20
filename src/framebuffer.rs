use spin::{Once, Mutex};
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
    size: usize,
}
struct BufferPtr(*mut Pixel);
unsafe impl Send for BufferPtr {}

static FRAMEBUFFER: Once<Mutex<FrameBuffer>> = Once::new();

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Pixel([u8; 3]);

pub fn get() -> spin::MutexGuard<'static, FrameBuffer> {
    FRAMEBUFFER.get().unwrap().lock()
}

pub unsafe fn init() -> Result<(), &'static str> {
    let mut mbox = mailbox::get();

    let res = mbox.send_and_poll_recieve_batch((
            FBSetPhysicalSizeRequest { width: 640, height: 480 }.into_tag(),
            FBSetVirtualSizeRequest { width: 640, height: 480 }.into_tag(),
            FBSetBitsPerPixelRequest { bpp: core::mem::size_of::<Pixel>() as u32 * 8}.into_tag(),
            FBAllocateBufferRequest { alignment: 16}.into_tag(),
    )).map_err(|_| "Batch framebuffer init failed")?;



    println!("Responses: {:#?}", res);
    let res = res.3.ok_or("FameBuffer mail did not get a response")?;

    let ptr = res.base_address as *mut u32 as *mut Pixel;
    println!("================ MODULO = {}", res.size % 3);
    let size = res.size / 3;
    for i in 0..size {
        unsafe {

            (*ptr.add(i as usize)).0[0] = u8::MAX;
        }
    }

    ({let a: Option<u32> = None; a}).unwrap();

    FRAMEBUFFER.call_once(|| Mutex::new(FrameBuffer { buffer: BufferPtr(res.base_address as *mut u32 as *mut Pixel), size: res.size as usize }));

    Ok(())
}

// pub fn
