use bcm2837_lpa::{AUX, UART1};

use arrayvec::ArrayString;
use core::fmt;
use spin::{Mutex, Once};

#[derive(Debug)]
pub struct Controller {
    uart: UART1,
}

impl Controller {
    pub fn read_char(&mut self) -> char {
        while self.uart.lsr.read().data_ready().bit_is_clear() {}
        self.uart.io().read().data().bits() as char
    }
}

impl fmt::Write for Controller {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        let c = if c as usize > u8::MAX.into() {
            '~' as u8
        } else { c as u8 };
        while self.uart.lsr.read().tx_empty().bit_is_clear() {}
        self.uart.io().write(|w| {
            w.data().variant(c)
        });
        Ok(())
    }
}

static UART: Once<Mutex<Controller>> = Once::new();

pub fn get() -> spin::MutexGuard<'static, Controller> {
    UART.get().unwrap().lock()
}

const BAUD_RATE: usize = 115200;
const ASSUMED_CPU_CLOCK_FREQ: usize = 250_000_000;
pub unsafe fn init(uart: UART1, &mit aux: AUX) {
    // NOTE TO SELF: On real board will have to set GPIO first

    let aux = periphs.AUX;

    // Enable use of UART
    aux.enables.write(|w| {
        w.uart_1().set_bit()
    });

    // Disable interrupts
    uart.ier().write(|w| w.bits(0));

    // Calculate and set baud rate
    let reg_baud = (ASSUMED_CPU_CLOCK_FREQ / BAUD_RATE / 8) - 1;
    uart.baud.write(|w| { w.bits(reg_baud as u16) });
    // Use 8-bit data
    uart.lcr.write(|w| { w.data_size()._8bit() });
    // Enable reading and writing
    uart.cntl.write(|w| {
        w.tx_enable().set_bit();
        w.rx_enable().set_bit();
        w
    });


    // {
    //     use fmt::Write;
    //     let mut c = Controller { uart };
    //     let _ = c.write_str("Hello World!");
    //
    //     let mut s = ArrayString::<10>::new();
    //     for _ in 0..s.capacity() {
    //         s.push(c.read_char());
    //     }
    //     let _ = c.write_str(&s);
    // }
    UART.call_once(|| Mutex::new(Controller { uart }));
}
