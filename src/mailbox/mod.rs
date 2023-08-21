use crate::{MMIODerefWrapper, phys_to_bus};
use bitfield_struct::bitfield;
use bitflags::bitflags;
use core::arch::asm;
use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};


use spin::{Mutex, Once};

pub mod tags;
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

type MBox = MMIODerefWrapper<Registers>;
pub struct Mailbox {
    mbox: MBox,
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
        let state = Status::from_bits_retain(self.mbox.write_status.get());
        state.contains(Status::FULL)
    }

    pub fn read_is_empty(&mut self) -> bool {
        let state = Status::from_bits_retain(self.mbox.read_status.get());
        state.contains(Status::EMPTY)
    }

    pub fn send_and_poll_recieve_one<T>(
        &mut self,
        req: T,
    ) -> Result<<<T as TagInterfaceRequest>::Tag as TagInterface>::Res, ()>
    where
        T: TagInterfaceRequest,
    {
        use core::cell::UnsafeCell;

        while self.send_is_full() {
            unsafe { asm!("nop") };
        }

        let message = UnsafeCell::new(PropertyBuffer {
            size: core::mem::size_of::<PropertyBuffer<T::Tag>>() as u32,
            req_res_code: BufferReqResCode::PROCESS_REQUEST,
            tags: req.into_tag(),
            end_tag: 0,
        });
        let m = message.get();
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        let data = MessagePtr::new().with_channel(8).with_prop_buf(m).into();
        unsafe {
            self.mbox.write.set(data);
        }

        while self.read_is_empty() {}
        let mut res_ptr = MessagePtr::new();
        while res_ptr.channel() != 8 {
            unsafe { asm!("nop") };
            let res = self.mbox.read.get();
            res_ptr = MessagePtr::from(res);
        }
        let res_buf_ptr = res_ptr.prop_buf::<T::Tag>();
        let res_buf = unsafe { &*res_buf_ptr };
        if res_buf
            .req_res_code
            .contains(BufferReqResCode::REQUEST_ERROR)
        {
            return Err(());
        }

        res_buf.tags.response().ok_or(())
    }

    // NOTE: Does not currently work. Must check on real hardware
    pub fn send_and_poll_recieve_batch<T: TagBatch>(&mut self, batch: T) -> Result<T::Res, ()> {
        use core::cell::UnsafeCell;

        while self.send_is_full() {}

        let message = UnsafeCell::new(PropertyBuffer {
            size: core::mem::size_of::<PropertyBuffer<T>>() as u32,
            req_res_code: BufferReqResCode::PROCESS_REQUEST,
            tags: batch,
            end_tag: 0,
        });
        let data = MessagePtr::new()
            .with_channel(8)
            .with_prop_buf(message.get())
            .into();
        unsafe {
            self.mbox.write.set(data);
        }

        while self.read_is_empty() {}
        let mut res_ptr = MessagePtr::new();
        while res_ptr.channel() != 8 {
            let res = self.mbox.read.get();
            res_ptr = MessagePtr::from(res);
        }
        let res_buf_ptr = res_ptr.prop_buf::<T>();
        let res_buf = unsafe { &*res_buf_ptr };
        if res_buf
            .req_res_code
            .contains(BufferReqResCode::REQUEST_ERROR)
        {
            return Err(());
        }
        Ok(res_buf.tags.responses())
    }
}

static MAILBOX: Once<Mutex<Mailbox>> = Once::new();

// https://github.com/raspberrypi/firmware/wiki/Mailboxes
register_structs! {
    Registers {
        (0x00 => read: ReadOnly<u32>),
        (0x04 => _padding1),
        (0x10 => _reserved_peek_reader),
        (0x14 => _reserved_sender_reader),
        (0x18 => read_status: ReadOnly<u32>),
        (0x1C => _reserved_read_config),
        (0x20 => write: WriteOnly<u32>),
        (0x24 => _padding2),
        (0x30 => _reserved_peek_sender),
        (0x34 => _reserved_sender_sender),
        (0x38 => write_status: ReadOnly<u32>),
        (0x3C => _reserved_write_config),
        (0x40 => @END),
    }
}

pub fn get() -> spin::MutexGuard<'static, Mailbox> {
    MAILBOX.get().unwrap().lock()
}

pub unsafe fn init() {
    let mbox = MBox::new(phys_to_bus(0xB880));
    MAILBOX.call_once(|| Mutex::new(Mailbox { mbox }));
}
