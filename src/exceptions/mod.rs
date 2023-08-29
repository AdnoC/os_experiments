use tock_registers::interfaces::Writeable;
use aarch64_cpu::{
    asm::barrier,
    registers::*,
};
use core::arch::global_asm;
global_asm!(include_str!("setup_handler.s"));

extern "Rust" {
    static __exception_vector_start: core::cell::UnsafeCell<()>;
}

#[no_mangle]
pub unsafe extern "C" fn __handle_exception() -> ! {
    do_exc();
    loop {}
}
fn do_exc() {
    use tock_registers::interfaces::Readable;
    let el = CurrentEL.read(CurrentEL::EL);
    println!("Exception occured");
    use core::fmt::Write;
    write!(crate::uart::get(), "IN EXC {}", el).ok();
    panic!("Exceptio");
}

pub unsafe fn init_el2() {
    VBAR_EL2.set(__exception_vector_start.get() as u64);
    barrier::isb(barrier::SY);
}
pub unsafe fn init() {
    VBAR_EL1.set(__exception_vector_start.get() as u64);
    barrier::isb(barrier::SY);
}
