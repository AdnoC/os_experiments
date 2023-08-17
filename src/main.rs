#![no_std]
#![no_main]

#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

mod serial;
mod test_runner;

mod vga_text;
mod gdt;
mod interrupts;

// our existing panic handler
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

// our panic handler in test mode
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use test_runner::*;
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

static HELLO: &[u8] = b"Hello World!";

fn init() {
    gdt::init();
    interrupts::init_idt();
    x86_64::instructions::interrupts::without_interrupts(|| {
        unsafe { interrupts::PICS.lock().initialize() };
    });
    x86_64::instructions::interrupts::enable();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();

    println!("Hello world!!!!");

    // #[cfg(test)]
    // test_main();

    loop {}
}
