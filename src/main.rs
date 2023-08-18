#![no_std]
#![no_main]

// #![feature(custom_test_frameworks)]
// #![test_runner(crate::test_runner::test_runner)]
// #![reexport_test_harness_main = "test_main"]

use core::arch::asm;
use core::panic::PanicInfo;

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
    // println!("{}", info);
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

fn init() {
    // gdt::init();
    // interrupts::init_idt();
    // x86_64::instructions::interrupts::without_interrupts(|| {
    //     unsafe { interrupts::PICS.lock().initialize() };
    // });
    // x86_64::instructions::interrupts::enable();
}

// global_asm!(r#".section ".text.boot""#);
#[no_mangle]
#[link_section = ".text.boot"]
pub extern "C" fn _start() -> ! {
    let x: usize;
    unsafe {
        // asm!("mrs {0}, CurrentEl",
        //      "ubfx {0}, {0}",
        //      out(reg) x);
        asm!("mrs x5, CurrentEl",
             "ubfx x5, x5, #2, #2");
        asm!(
            "2:",
            "wfe",
            "b 2b"
            );
    }
    unsafe {
        let p: *mut usize = 0x80000 as *mut usize;
        *p = HELLO[0] as usize;
    }
    // init();
    //
    // println!("Hello world!!!!");

    // #[cfg(test)]
    // test_main();

    loop {}
}
