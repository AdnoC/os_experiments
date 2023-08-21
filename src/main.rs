#![no_std]
#![no_main]

// #![feature(custom_test_frameworks)]
// #![test_runner(crate::test_runner::test_runner)]
// #![reexport_test_harness_main = "test_main"]

use core::arch::{global_asm};
use core::convert::Infallible;
use core::panic::PanicInfo;


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
    }};
}

mod framebuffer;
mod mailbox;
mod time;
mod uart;

// mod serial;
// mod test_runner;
//
// mod vga_text;
// mod gdt;
// mod interrupts;

// our existing panic handler
// #[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprintln!("Panic Occured: {}", info);
    loop {}
}

// our panic handler in test mode
// #[cfg(test)]
// #[panic_handler]
// fn panic(info: &PanicInfo) -> ! {
//     use test_runner::*;
//     serial_println!("[failed]\n");
//     serial_println!("Error: {}\n", info);
//     exit_qemu(QemuExitCode::Failed);
//     loop {}
// }

#[no_mangle]
pub static HELLO: &[u8] = b"Hello World!";
static mut XYZ: [u8; 0xabc123] = [0; 0xabc123];

extern "C" {
    static __end: usize;
}
fn init() {
    // gdt::init();
    // interrupts::init_idt();
    // x86_64::instructions::interrupts::without_interrupts(|| {
    //     unsafe { interrupts::PICS.lock().initialize() };
    // });
    // x86_64::instructions::interrupts::enable();
}

global_asm!(include_str!("boot.s"));

// #[no_mangle]
// #[link_section = ".text.boot"]
// pub extern "C" fn _start() -> ! {
//     unsafe {
//         asm!("mrs x5, CurrentEl", // Move the CurrentEL system register into x5.
//              "ubfx x5, x5, #2, #2"); // Extract the relevant bitfield (bits 3:2).
//
//         asm!(
//             // Set the SPSel register so that SP_EL0 is the stack pointer at all EL.
//             "mrs x6, SPSel",        // Move the current SPSel  system register into x6.
//             "and x6, x6, ~1",       // Clear the 0 bit of x6.
//             "msr SPSel, x6",        // Set the value of SPSel to x6.
//
//             // Set up the stack below our code (it grows downwards).
//             // This should be plenty big enough: only the first 4KB of memory are used.
//             "ldr x6, =_start",
//             "mov sp, x6"
//             );
//         asm!(
//             "ldr x6, =__bss_start",
//             "ldr x7, =__bss_end",
//             "21:",
//             "cmp x6, x7",
//             "b.ge 21f",
//             "str xzr, [x6]",
//             "add x6, x6, #8",
//             "b 21b",
//             "21:"
//             );
//     }
//     kernel_start()
// }

#[no_mangle]
pub extern "C" fn __start_kernel() -> ! {
    if let Err(err) = main() {
        eprintln!("Error occured somewhere. Main returned Err.");
        eprintln!("{:?}", err);
    }

    loop {}
}
fn main() -> Result<Infallible, &'static str> {
    let mut periphs = unsafe { bcm2837_lpa::Peripherals::steal() };

    unsafe {
        uart::init(periphs.UART1, &mut periphs.AUX);
        mailbox::init(periphs.VCMAILBOX);
        framebuffer::init()?;
    }

    println!("Hello from println!!!!");
    println!("End of kernel addr = {}", unsafe { __end });

    framebuffer::draw_text("HELLOOOOOOO");

    loop {
        time::wait_microsec(1_000_000);
        println!("Its been a second");
    }

    // init();
    //
    // println!("Hello world!!!!");

    // #[cfg(test)]
    // test_main();
}
