use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use pic8259::ChainedPics;
use crate::{print, println};
use spin::{Lazy, Mutex};

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(handle_breakpoint);
    idt[InterruptIndex::Timer.as_usize()].set_handler_fn(handle_timer);
    unsafe {
        idt.double_fault
            .set_handler_fn(handle_double_fault)
            .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt
});

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn handle_breakpoint(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Breakpoint {:#?}", stack_frame);
}

extern "x86-interrupt" fn handle_timer(stack_frame: InterruptStackFrame) {
    // println!("EXCEPTION: Timer {:#?}", stack_frame);
    print!(".");

    x86_64::instructions::interrupts::without_interrupts(|| {
        unsafe {
            PICS.lock()
                .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
        }
    });
}

extern "x86-interrupt" fn handle_double_fault(
    stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

mod tests {
    #[test_case]
    fn breakpoints() {
        // As long as this test does not hang we handle this interrupt type
        x86_64::instructions::interrupts::int3();
    }
}
