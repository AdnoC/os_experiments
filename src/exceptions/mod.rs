use bitflags::bitflags;
use bit_field::BitField;
use num_enum::TryFromPrimitive;
use tock_registers::{
    interfaces::{Readable, Writeable},
    registers::InMemoryRegister,
};
use aarch64_cpu::{
    asm::barrier,
    registers::*,
};
use arrayvec::ArrayString;
use core::arch::global_asm;
use crate::units::KB;
global_asm!(include_str!("setup_handler.s"));

extern "Rust" {
    static __exception_vector_table: core::cell::UnsafeCell<()>;
}

static mut output: ArrayString::<{1 * KB as usize}> = ArrayString::new_const();
struct ExceptionCause(InMemoryRegister<u64, ESR_EL1::Register>);
impl ExceptionCause {
    pub fn new() -> Self {
        ExceptionCause(InMemoryRegister::new(ESR_EL1.get()))
    }
    pub fn get_cause(&self) -> ESR_EL1::EC::Value {
        self.0.read_as_enum(ESR_EL1::EC).unwrap()
    }
    pub fn get_data_abort_cause(&self) -> Option<DataAbortCause> {

        if !matches!(self.get_cause(), ESR_EL1::EC::Value::DataAbortCurrentEL | ESR_EL1::EC::Value::DataAbortLowerEL) {
            return None;
        }

        let val = self.0.get();
        let iss2 = val.get_bits(32..=55);
        println!("iss = {:b}", iss2);
        iss2.try_into().ok()
    }
}

#[repr(u64)]
#[derive(Debug, TryFromPrimitive)]
enum DataAbortCause {
    AllocTagWrite = 1 << 10,
    GaurdedStackAccess = 1 << 8,
    OverlayPermissions = 1 << 6,
    DirtyBit = 1 << 5,
}


// https://krinkinmu.github.io/2021/01/10/aarch64-interrupt-handling.html
#[no_mangle]
pub extern "C" fn __handle_exception(frame: &mut InterruptFrame) {

    use ESR_EL1::EC::Value as Cause;
    let syndrome_reg = ExceptionCause::new();
    let cause = syndrome_reg.get_cause();
    match cause {
        Cause::DataAbortCurrentEL => {
            use DataAbortCause::*;
            println!("Kernel experienced a page fault.");
            let abort_cause = syndrome_reg.get_data_abort_cause();
            println!("Fault cause: {:?}", abort_cause);
        },
        _ => {
            println!("Unknown/unhandled exception type. 0x{:x}", cause as  u64);
            loop {}
        },

    }
    crate::mmu::without_mmu! {
        do_exc(InterruptType::Exception, &mut *frame);
    };
}

#[no_mangle]
pub unsafe extern "C" fn __handle_interrupt(frame: &mut InterruptFrame) {
    crate::mmu::without_mmu! {
        do_exc(InterruptType::Interrupt, &mut *frame);
    }
}

fn do_exc(int_type: InterruptType, frame: &mut InterruptFrame) {
    let el = CurrentEL.read(CurrentEL::EL);
    // println!("Exception occured");
    use core::fmt::Write;
    println!("Handling interrupt. Type = {:?}", int_type);
    println!("{:x?}", frame);
    // let mut u = crate::uart::get();
    // unsafe {
    //     write!(output, "IN EXC {}", el).ok();
    //     u.write_str(output.as_str()).unwrap();
    //
    // }

    loop {}
    // panic!("Exceptio");
}

pub unsafe fn init_el2() {
    VBAR_EL2.set(__exception_vector_table.get() as u64);
    barrier::isb(barrier::SY);
    enable_interrupts();
}
pub unsafe fn init() {
    VBAR_EL1.set(__exception_vector_table.get() as u64);
    barrier::isb(barrier::SY);
    enable_interrupts();
}

pub fn disable_interrupts() {
    DAIF.write(DAIF::D::Masked
               + DAIF::A::Masked
               + DAIF::I::Masked
               + DAIF::F::Masked);
}

pub fn enable_interrupts() {
    DAIF.write(DAIF::D::Unmasked
               + DAIF::A::Unmasked
               + DAIF::I::Unmasked
               + DAIF::F::Unmasked);
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum InterruptType {
    Interrupt,
    Exception,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct InterruptFrame {
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
    x8: u64,
    x9: u64,
    x10: u64,
    x11: u64,
    x12: u64,
    x13: u64,
    x14: u64,
    x15: u64,
    x16: u64,
    x17: u64,
    x18: u64,
    fp: u64,
    lr: u64,
    xzr: u64,
    esr: u64,
    far: u64,
}
