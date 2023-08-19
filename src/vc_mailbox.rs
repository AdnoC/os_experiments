use bcm2837_lpa::VCMAILBOX;
use bitflags::bitflags;
use bitfield_struct::bitfield;
use paste::paste;
use core::fmt;

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

#[bitfield(u32)]
struct TagReqResCode {
    #[bits(31)]
    _reserved: u32,
    is_response: bool
}

#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
struct PropertyBuffer<T> {
    size: u32,
    req_res_code: BufferReqResCode,
    tags: T,
    end_tag: u32,
}

#[derive(Clone, Copy)]
#[repr(C, align(32))]
struct Tag<Req: Copy, Res: Copy> {
    id: TagValue,
    size: u32,
    req_res_code: TagReqResCode,
    data: TagData<Req, Res>,
}

impl<Req: Copy, Res: Copy> Tag<Req, Res> {
    pub fn is_request(&self) -> bool {
        !self.req_res_code.is_response()
    }

    pub fn is_response(&self) -> bool {
        self.req_res_code.is_response()
    }
}

impl<Req, Res> fmt::Debug for Tag<Req, Res> where
    Req: Copy + fmt::Debug,
    Res: Copy + fmt::Debug {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            static INVALID_STR: &'static str = "<INVALID>";
            let mut f = f.debug_struct("Tag");
            f.field("id", &self.id)
                .field("size", &self.size)
                .field("req_res_code", &self.req_res_code);
            if self.is_request() {
                unsafe { f.field("req", &self.data.req); }
            } else if self.is_response() {
                unsafe { f.field("req", &self.data.res); }
            } else {
                f.field("union<req, res>", &INVALID_STR);
            }
            f.finish()
        }
}

#[repr(C)]
#[derive(Copy, Clone)]
union TagData<Req: Copy, Res: Copy> {
    req: Req,
    res: Res,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
enum TagValue {
    FirmwareRevision = 0x0_0001,
    BoardModel = 0x1_0001,
}

trait TagInterface: fmt::Debug {
    const ID: TagValue;
    type Req;
    type Res;

    fn from_request(req: Self::Req) -> Self;
    fn request(&self) -> Option<Self::Req>;
    fn response(&self) -> Option<Self::Res>;
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

    pub fn send_and_poll_recieve_one<T: TagInterface>(&mut self, req: T::Req) -> T::Res {
        use core::cell::UnsafeCell;

        println!("Waiting for space in outbound");
        while self.send_is_full() {}

        let message = UnsafeCell::new(
            PropertyBuffer {
                size: core::mem::size_of::<PropertyBuffer<T>>() as u32,
                req_res_code: BufferReqResCode::PROCESS_REQUEST,
                tags: T::from_request(req),
                end_tag: 0,
            }
        );
        let data = MessagePtr::new()
            .with_channel(8)
            .with_prop_buf(message.get()).into();
        println!("Writing ptr to outbount");
        unsafe {
            self.mbox.write.write_with_zero(|w| w.bits(data));
        }

        println!("Waiting for inbound");
        while self.read_is_empty() {}
        let mut res_ptr = MessagePtr::new();
        println!("Got res for channel #{}", res_ptr.channel());
        while res_ptr.channel() != 8 {
            println!("Got new res for channel #{}", res_ptr.channel());
            let res = self.mbox.read.read().bits();
            res_ptr = MessagePtr::from(res);
        }
        println!("Happy with the channel. loading response buffer");
        let res_buf_ptr = res_ptr.prop_buf::<T>();
        let res_buf = unsafe { &*res_buf_ptr };
        println!("buf = {:#?}", res_buf);
        println!("tag = {:#?}", res_buf.tags);
        res_buf.tags.response().unwrap()
    }
}
pub unsafe fn init(mbox: VCMAILBOX) {
    let mut mbox = Mailbox { mbox };
    println!("Gettting firmware revision");
    let res = mbox.send_and_poll_recieve_one::<BoardModelTag>(BoardModelRequest {});
    println!("Res: {:?}", res);
    println!("firmware = {}", res.model);
}

macro_rules! define_tag {
    ($name:ident, $enum_value:expr, {$($req_field_name:ident:$req_field_type:ty),*}, {$($res_field_name:ident:$res_field_type:ty),*}) => {
        paste! {
define_tag!([<$name Tag>], $enum_value, [<$name Request>], [<$name Response>], {$($req_field_name: $req_field_type),*}, {$($res_field_name: $res_field_type),*});
        }
    };
    ($tag_name:ident, $enum_value:expr, $req_name:ident, $res_name:ident, {$($req_field_name:ident:$req_field_type:ty),*}, {$($res_field_name:ident:$res_field_type:ty),*}) => {
            #[repr(C)]
            #[derive(Clone,Copy, Debug)]
            pub struct $req_name {
                $(
                    pub $req_field_name: $req_field_type,
                )*
            }

            #[repr(C)]
            #[derive(Clone,Copy, Debug)]
            pub struct $res_name {
                $(
                    pub $res_field_name: $res_field_type,
                )*
            }
            pub type $tag_name = Tag<$req_name, $res_name>;
            impl TagInterface for $tag_name {
                const ID: TagValue = $enum_value;
                type Req = $req_name;
                type Res = $res_name;
                fn from_request(req: $req_name) -> $tag_name {
                    Tag {
                        id: $enum_value,
                        size: core::mem::size_of::<$req_name>() as u32,
                        req_res_code: TagReqResCode::new(),
                        data: TagData { req, },
                    }
                }

                fn request(&self) -> Option<$req_name> {
                    match self.is_request() {
                        true => unsafe { Some(self.data.req) },
                        false => None,
                    }
                }

                fn response(&self) -> Option<$res_name> {
                    match self.is_response() {
                        true => unsafe { Some(self.data.res) },
                        false => None,
                    }
                }
            }

    };
}

// https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface
define_tag! {
    FirmwareRevision,
    TagValue::FirmwareRevision,
    {},
    {
        revision: u32
    }
}

define_tag! {
    BoardModel,
    TagValue::BoardModel,
    {},
    {
        model: u32
    }
}
