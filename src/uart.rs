use crate::{MMIODerefWrapper, bus_to_phys};
use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::ReadWrite,
};

type Uart = MMIODerefWrapper<Registers>;
type Aux = MMIODerefWrapper<aux::Registers>;

use core::fmt;
use spin::{Mutex, Once};

pub struct Controller {
    uart: Uart,
}

impl Controller {
    pub fn read_char(&mut self) -> char {
        while !self.uart.lsr.is_set(LSR::DATA_READY) {}
        self.uart.io.read(IO::DATA) as u8 as char
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
        } else {
            c as u8
        };
        while !self.uart.lsr.is_set(LSR::TRANSMITTER_EMPTY) {}
        self.uart.io.write(IO::DATA.val(c as u32));
        Ok(())
    }
}

static UART: Once<Mutex<Controller>> = Once::new();

pub fn try_get() -> Option<spin::MutexGuard<'static, Controller>> {
    UART.get().and_then(|m| m.try_lock())
}

pub fn get() -> spin::MutexGuard<'static, Controller> {
    UART.get().unwrap().lock()
}
// Section 2.2
// https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf
register_structs! {
    Registers {
        (0x00 => io: ReadWrite<u32, IO::Register>),
        (0x04 => _reserved_iir),
        (0x08 => ier: ReadWrite<u32, IER::Register>),
        (0x0C => lcr: ReadWrite<u32, LCR::Register>),
        (0x10 => _reserved_mcr),
        (0x14 => lsr: ReadWrite<u32, LSR::Register>),
        (0x18 => _reserved_msr),
        (0x1C => _reserved_scratch),
        (0x20 => cntrl: ReadWrite<u32, CNTRL::Register>),
        (0x24 => _reserved_stat),
        (0x28 => baud: ReadWrite<u32, BAUD::Register>),
        (0x2C => @END),
    }
}

register_bitfields! {
    // 32 bit registers
    u32,

    IO [
        DATA    OFFSET(0)   NUMBITS(8),
    ],
    // IIR [],
    IER [
        INTERRUPT_PENDING   OFFSET(0)   NUMBITS(1) [],
        // Also reads as interrupt ID bit
        INTERRUPTS_ENABLED  OFFSET(1)   NUMBITS(2) [
            BothOff = 0,
        ],
    ],
    LCR [
        DATA_SIZE       OFFSET(0)   NUMBITS(1) [
            SevenBit = 0,
            EightBit = 1,
        ],
        DLAB_ACCESS     OFFSET(7)   NUMBITS(1) [],
    ],
    // MCR [],
    LSR [
        DATA_READY          OFFSET(0)   NUMBITS(1) [],
        RECEIVER_OVERRUN    OFFSET(1)   NUMBITS(1) [],
        TRANSMITTER_EMPTY   OFFSET(5)   NUMBITS(1) [],
        TRANSMITTER_IDLE    OFFSET(6)   NUMBITS(1) [],
    ],
    // MSR [],
    // SCRATCH [],
    CNTRL [
        RECEIVER_ENABLE     OFFSET(0)   NUMBITS(1) [],
        TRANSMITTER_ENABLE  OFFSET(1)   NUMBITS(1) [],
    ],
    // STAT [],
    BAUD [
        BAUDRATE    OFFSET(0)   NUMBITS(16) [],
    ],
}

mod aux {
    use tock_registers::{
        register_bitfields, register_structs,
        registers::ReadWrite,
    };
    register_structs! {
        pub Registers {
            (0x00 => _reserved_irq),
            (0x04 => pub enable: ReadWrite<u32, ENABLES::Register>),
            (0x08 => @END),
        }
    }

    register_bitfields! {
        // 32 bit registers
        u32,
        pub ENABLES [
            MINI_UART_ENABLE    OFFSET(0)   NUMBITS(1) [],
            SPI1_ENABLE         OFFSET(1)   NUMBITS(1) [],
            SPI2_ENABLE         OFFSET(2)   NUMBITS(1) [],
        ],
    }
}

const BAUD_RATE: usize = 115200;
const ASSUMED_CPU_CLOCK_FREQ: usize = 250_000_000;
pub unsafe fn init() {
    // NOTE TO SELF: On real board will have to set GPIO first

    let aux = Aux::new(bus_to_phys(0x7E21_5000));
    let uart = Uart::new(bus_to_phys(0x7E21_5040));

    // Disable interrupts
    uart.ier.modify(IER::INTERRUPTS_ENABLED::BothOff);

    // Use 8-bit data
    uart.lcr.modify(LCR::DATA_SIZE::EightBit + LCR::DLAB_ACCESS::CLEAR);

    // Calculate and set baud rate
    let reg_baud = (ASSUMED_CPU_CLOCK_FREQ / BAUD_RATE / 8) - 1;
    uart.baud.write(BAUD::BAUDRATE.val(reg_baud as u32));


    // Enable reading and writing
    uart.cntrl.modify(CNTRL::RECEIVER_ENABLE::SET + CNTRL::TRANSMITTER_ENABLE::SET);

    // Enable use of UART
    aux.enable.modify(aux::ENABLES::MINI_UART_ENABLE::SET);

    UART.call_once(|| Mutex::new(Controller { uart }));
}

pub fn spin_until_enter() {
    let mut uart = get();
    loop {
        let c = uart.read_char();
        if c as usize == 13 {
            return;
        }
    }
}
