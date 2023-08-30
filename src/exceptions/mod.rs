use tock_registers::interfaces::Writeable;
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

// https://krinkinmu.github.io/2021/01/10/aarch64-interrupt-handling.html
#[no_mangle]
pub unsafe extern "C" fn __handle_exception() -> ! {
    disable_interrupts();
    crate::mmu::without_mmu! {
        do_exc();
    };
    enable_interrupts();
    aarch64_cpu::asm::eret()
}

#[no_mangle]
pub unsafe extern "C" fn __handle_interrupt() -> ! {
    disable_interrupts();
    do_exc();
    enable_interrupts();
    aarch64_cpu::asm::eret()
}

fn do_exc() {
    use tock_registers::interfaces::Readable;
    let el = CurrentEL.read(CurrentEL::EL);
    // println!("Exception occured");
    use core::fmt::Write;
    let mut u = crate::uart::get();
    unsafe {
        write!(output, "IN EXC {}", el).ok();
        u.write_str(output.as_str()).unwrap();

    }

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
