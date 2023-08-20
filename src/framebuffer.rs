use crate::mailbox;
use crate::mailbox::tags::{
    BoardModelRequest,
    FBSetPhysicalSizeRequest,
    FBSetVirtualSizeRequest,
    FBSetBitsPerPixelRequest,
    FBAllocateBufferRequest,
    TagInterfaceRequest
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Pixel([u8; 3]);

pub fn frame() {
    let mut mbox = mailbox::get();
    println!("Gettting firmware revision");

    let res = mbox.send_and_poll_recieve_batch((
            FBSetPhysicalSizeRequest { width: 640, height: 480 }.into_tag(),
            FBSetVirtualSizeRequest { width: 640, height: 480 }.into_tag(),
            FBSetBitsPerPixelRequest { bpp: core::mem::size_of::<Pixel>() as u32 * 8}.into_tag(),
            FBAllocateBufferRequest { alignment: 16}.into_tag(),
    )).unwrap();



    println!("Responses: {:#?}", res);
    let res = res.3.unwrap();

    let ptr = res.base_address as *mut u32 as *mut Pixel;
    println!("================ MODULO = {}", res.size % 3);
    let size = res.size / 3;
    for i in 0..size {
        unsafe {

            (*ptr.add(i as usize)).0[0] = u8::MAX;
        }
    }
}
