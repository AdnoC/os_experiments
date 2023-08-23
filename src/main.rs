#![no_std]
#![no_main]

#![allow(dead_code)]

// #![feature(custom_test_frameworks)]
// #![test_runner(crate::test_runner::test_runner)]
// #![reexport_test_harness_main = "test_main"]

use core::arch::global_asm;
use core::convert::Infallible;
use core::panic::PanicInfo;
use tock_registers::interfaces::Writeable;
use aarch64_cpu::{asm, registers::*};


#[allow(unused_macros)]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        write!(crate::uart::get(), $($arg)*).unwrap();
    }};
}
macro_rules! println {
    () => {{
        use core::fmt::Write;
        write!(crate::uart::get(), "\n").unwrap();
    }};
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        write!(crate::uart::get(), $($arg)*).unwrap();
        println!();
    }};
}

macro_rules! try_println {
    () => {{
        use core::fmt::Write;
        crate::uart::try_get()
            .ok_or(())
            .map(|mut w| write!(w, "\n"))
    }};
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        crate::uart::try_get()
            .ok_or(())
            .map(|mut w| {
                write!(w, $($arg)*)
                    .and(write!(w, "\n"))
            })
    }};
}

// Try to report an error in various ways
macro_rules! eprintln {
    () => {{
        try_println!("\n")
            .unwrap()
    }};
    ($($arg:tt)*) => {{
        try_println!($($arg)*)
            .unwrap()
            .unwrap()
    }};
}

mod framebuffer;
mod mailbox;
mod time;
mod uart;
mod mmu;


// our existing panic handler
// #[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprintln!("Panic Occured: {}", info);
    loop {}
}


#[no_mangle]
pub static HELLO: &[u8] = b"Hello World!";
static mut XYZ: [u8; 0xabc123] = [0; 0xabc123];

extern "C" {
    static __kernel_stack_start: usize;
}



global_asm!(include_str!("boot.s"));


/// Transition from hypervisor to OS
///
/// # Safety
///
/// `bss` hasn't been initialized, so do not touch it
unsafe fn prep_transition_el2_to_el1() {
    // Enable timer counter registers for EL1
    CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);

    // https://developer.arm.com/documentation/ddi0601/2023-06/AArch64-Registers/CNTVOFF-EL2--Counter-timer-Virtual-Offset-Register?lang=en
    // No offset for EL1 timers
    CNTVOFF_EL2.set(0);

    // EL1 should be aarch64
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    // Fake an exception return:
    //
    // Pretend we are in saved program state where all interrupts are masked an SP_EL1 was used as
    // stack pointer
    SPSR_EL2.write(
        SPSR_EL2::D::Masked
            + SPSR_EL2::A::Masked
            + SPSR_EL2::I::Masked
            + SPSR_EL2::F::Masked
            + SPSR_EL2::M::EL1h,
    );

    // Then let link register point to `kernel_main`
    ELR_EL2.set(kernel_init as *const() as u64);

    // Set up stack pointer. Just set it to the same stack we are using now.
    SP_EL1.set(__kernel_stack_start as u64);
}

#[no_mangle]
pub unsafe extern "C" fn __start_kernel() -> ! {
    kernel_init()
    // prep_transition_el2_to_el1();
    // asm::eret()
}

#[no_mangle]
pub extern "C" fn kernel_init() -> ! {
    if let Err(err) = main() {
        eprintln!("Error occured somewhere. Main returned Err.");
        eprintln!("{:?}", err);
    }

    loop {}
}
fn main() -> Result<Infallible, &'static str> {
    unsafe {
        uart::init();
        mailbox::init();
        framebuffer::init()?;
    }

    println!("Hello from println!!!!");



    // framebuffer::draw_text("HELLOOOOOOO");

    loop {
        time::wait_microsec(1_000_000);
        println!("Its been a second");
    }
}

/// Convert a bus address into a physical address.
///
///
/// The documentation for the BCM2837 gives peripheral bus addresses, which are
/// not directly mapped to physical addresses. Physical addresses 0x3f000000 to
/// 0x3fffffff, used for peripheral MMIO, are mapped starting at the peripheral
/// bus addresses range starting at 0x7e000000 (and ending at 0x7effffff).
///
/// Example: bus address 0x7e00beef corresponds to physical address 0x3f00beef.
pub const fn bus_to_phys(addr: usize) -> usize {
    addr - 0x3f000000
}

// Get the full address for a mmio peripheral
// https://jsandler18.github.io/extra/peripheral.html
// NOTE: This is a different address for the Pi 1
pub const fn phys_to_bus(base: usize) -> usize {
    base + 0x3f000000
}

#[derive(Debug)]
pub struct MMIODerefWrapper<T> {
    addr: usize,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> MMIODerefWrapper<T> {
    pub const unsafe fn new(addr: usize) -> Self {
        MMIODerefWrapper {
            addr,
            _phantom: core::marker::PhantomData,
        }
    }
}
impl<T> core::ops::Deref for MMIODerefWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*(self.addr as *const _)
        }
    }
}
