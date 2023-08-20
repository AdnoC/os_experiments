use bcm2837_lpa::VCMAILBOX;
use bitflags::bitflags;
use bitfield_struct::bitfield;
use paste::paste;
use core::fmt;

mod tags;
use tags::*;

#[repr(u8)]
enum Channel {
    CPU_TO_VC = 8,
    VC_TO_CPU = 9,
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    struct Status: u32 {
        const FULL = 0x80000000;
        const EMPTY = 0x40000000;
    }

    #[derive(Clone, Copy, Debug)]
    struct BufferReqResCode: u32 {
        const PROCESS_REQUEST = 0x0;
        const REQUEST_SUCCESSFUL = 0x80000000;
        const REQUEST_ERROR = 0x80000001;
    }
}

pub struct Mailbox {
    mbox: VCMAILBOX,
}

#[bitfield(u32)]
struct MessagePtr {
    #[bits(4)]
    channel: u8,
    #[bits(28)]
    ptr: u32,
}

impl MessagePtr {
    fn with_prop_buf<T>(self, buf: *mut PropertyBuffer<T>) -> Self {
        self.with_ptr((buf as u32) >> 4)
    }
    fn prop_buf<T>(&self) -> *mut PropertyBuffer<T> {
        (self.ptr() << 4) as *mut PropertyBuffer<T>
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
struct PropertyBuffer<T> {
    size: u32,
    req_res_code: BufferReqResCode,
    tags: T,
    end_tag: u32,
}

impl Mailbox {
    pub fn send_is_full(&mut self) -> bool {
        let state = Status::from_bits_retain(self.mbox.status1.read().bits());
        state.contains(Status::FULL)
    }

    pub fn read_is_empty(&mut self) -> bool {
        let state = Status::from_bits_retain(self.mbox.status0.read().bits());
        state.contains(Status::EMPTY)
    }

    pub fn send_and_poll_recieve_one<T>(&mut self, req: T) -> Result<<<T as TagInterfaceRequest>::Tag as TagInterface>::Res, ()> where
T: TagInterfaceRequest,
{
        use core::cell::UnsafeCell;

        while self.send_is_full() {}

        let message = UnsafeCell::new(
            PropertyBuffer {
                size: core::mem::size_of::<PropertyBuffer<T::Tag>>() as u32,
                req_res_code: BufferReqResCode::PROCESS_REQUEST,
                tags: req.into_tag(),
                end_tag: 0,
            }
        );
        let data = MessagePtr::new()
            .with_channel(8)
            .with_prop_buf(message.get()).into();
        unsafe {
            self.mbox.write.write_with_zero(|w| w.bits(data));
        }

        while self.read_is_empty() {}
        let mut res_ptr = MessagePtr::new();
        while res_ptr.channel() != 8 {
            let res = self.mbox.read.read().bits();
            res_ptr = MessagePtr::from(res);
        }
        let res_buf_ptr = res_ptr.prop_buf::<T::Tag>();
        let res_buf = unsafe { &*res_buf_ptr };
        if res_buf.req_res_code.contains(BufferReqResCode::REQUEST_ERROR) {
            return Err(());
        }

        res_buf.tags.response().ok_or(())
    }

// NOTE: Does not currently work. Must check on real hardware
    pub fn send_and_poll_recieve_batch<T: TagBatch>(&mut self, batch: T) -> Result<T::Res, ()> {
        use core::cell::UnsafeCell;

        while self.send_is_full() {}

        let message = UnsafeCell::new(
            PropertyBuffer {
                size: core::mem::size_of::<PropertyBuffer<T>>() as u32,
                req_res_code: BufferReqResCode::PROCESS_REQUEST,
                tags: batch,
                end_tag: 0,
            }
        );
        let data = MessagePtr::new()
            .with_channel(8)
            .with_prop_buf(message.get()).into();
        unsafe {
            self.mbox.write.write_with_zero(|w| w.bits(data));
        }

        while self.read_is_empty() {}
        let mut res_ptr = MessagePtr::new();
        while res_ptr.channel() != 8 {
            let res = self.mbox.read.read().bits();
            res_ptr = MessagePtr::from(res);
        }
        let res_buf_ptr = res_ptr.prop_buf::<T>();
        let res_buf = unsafe { &*res_buf_ptr };
        println!("buf: {:#?}", res_buf);
        if res_buf.req_res_code.contains(BufferReqResCode::REQUEST_ERROR) {
            return Err(());
        }
        Ok(res_buf.tags.responses())
    }
}


pub unsafe fn init(mbox: VCMAILBOX) {
    let mut mbox = Mailbox { mbox };
    println!("Gettting firmware revision");
    let _ = mbox.send_and_poll_recieve_one(BoardModelRequest {}).unwrap();
    let _ = mbox.send_and_poll_recieve_one(FBSetPhysicalSizeRequest { width: 640, height: 480 }).unwrap();
    let _ = mbox.send_and_poll_recieve_one(FBSetVirtualSizeRequest { width: 640, height: 480 }).unwrap();
    let _ = mbox.send_and_poll_recieve_one(FBSetBitsPerPixelRequest { bpp: core::mem::size_of::<Pixel>() as u32 * 8}).unwrap();
    let res = mbox.send_and_poll_recieve_one(FBAllocateBufferRequest { alignment: 16}).unwrap();
    println!("Res: {:?}", res);

    // let res = mbox.send_and_poll_recieve_batch((
    //         FBSetPhysicalSizeRequest { width: 640, height: 480 }.into_tag(),
    //         FBSetVirtualSizeRequest { width: 640, height: 480 }.into_tag(),
    //         FBSetBitsPerPixelRequest { bpp: core::mem::size_of::<Pixel>() as u32 * 8}.into_tag(),
    //         )).unwrap();



    println!("Responses: {:#?}", res);

    let res = mbox.send_and_poll_recieve_one(FBAllocateBufferRequest { alignment: 16}).unwrap();
    let ptr = res.base_address as *mut u32 as *mut Pixel;
    println!("================ MODULO = {}", res.size % 3);
    let size = res.size / 3;
    for i in 0..size {
        unsafe {

            (*ptr.add(i as usize)).0[0] = u8::MAX;
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Pixel([u8; 3]);
