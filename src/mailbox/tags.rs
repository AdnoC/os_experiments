use bcm2837_lpa::VCMAILBOX;
use bitflags::bitflags;
use bitfield_struct::bitfield;
use paste::paste;
use core::fmt;

#[bitfield(u32)]
pub struct TagReqResCode {
    #[bits(31)]
    _reserved: u32,
    is_response: bool
}

#[derive(Clone, Copy)]
#[repr(C, align(32))]
pub struct Tag<Req: Copy, Res: Copy> {
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
pub union TagData<Req: Copy, Res: Copy> {
    req: Req,
    res: Res,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum TagValue {
    FirmwareRevision = 0x0_0001,
    BoardModel = 0x1_0001,
    FBAllocateBuffer = 0x4_0001,
    FBReleaseBuffer = 0x4_8001,
    FBGetPhysicalSize = 0x4_0003,
    FBSetPhysicalSize = 0x4_8003,
    FBSetVirtualSize = 0x4_8004,
    // AKA Depth
    FBSetBitsPerPixel = 0x4_8005,
}

pub trait TagInterface: fmt::Debug {
    const ID: TagValue;
    type Req;
    type Res;

    fn from_request(req: Self::Req) -> Self;
    fn request(&self) -> Option<Self::Req>;
    fn response(&self) -> Option<Self::Res>;
}

macro_rules! define_tags {
    ($({
        $name:ident, $enum_value:expr, {$($req_field_name:ident:$req_field_type:ty),*}, {$($res_field_name:ident:$res_field_type:ty),*}
    }),*) => {

        $(
            define_tag!($name, $enum_value, {$($req_field_name:$req_field_type),*}, {$($res_field_name:$res_field_type),*});
        )*
    }
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
define_tags! {
    {
        FirmwareRevision,
        TagValue::FirmwareRevision,
        {},
        {
            revision: u32
        }
    },
    {
        BoardModel,
        TagValue::BoardModel,
        {},
        {
            model: u32
        }
    },

    // Frame buffer stuff
    {
        FBAllocateBuffer,
        TagValue::FBAllocateBuffer,
        {
            alignment: u32
        },
        {
            base_address: u32,
            size: u32
        }
    },
    {
        FBReleaseBuffer,
        TagValue::FBReleaseBuffer,
        {},
        {}
    },
    {
        FBGetPhysicalSize,
        TagValue::FBGetPhysicalSize,
        {},
        {
            width: u32,
            height: u32
        }
    },
    {
        FBSetPhysicalSize,
        TagValue::FBSetPhysicalSize,
        {
            width: u32,
            height: u32
        },
        {
            width: u32,
            height: u32
        }
    },
    {
        FBSetVirtualSize,
        TagValue::FBSetVirtualSize,
        {
            width: u32,
            height: u32
        },
        {
            width: u32,
            height: u32
        }
    },
    {
        FBSetBitsPerPixel,
        TagValue::FBSetBitsPerPixel,
        {
            bpp: u32
        },
        {
            bpp: u32
        }
    }
}
