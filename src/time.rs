use core::arch::asm;

fn timer_frequency() -> u32 {
    let freq;
    unsafe {
        asm!("mrs {}, cntfrq_el0", out(reg) freq);
    }
    freq
}

fn timer_count() -> u64 {
    let count;
    unsafe {
        asm!("mrs {}, cntpct_el0", out(reg) count);
    }
    count
}

pub fn wait_cycle(mut num: usize) {
    while num > 0 {
        num -= 1;
        unsafe { asm!("noop") };
    }
}
pub fn wait_microsec(msec: usize) {
    let freq = timer_frequency();
    let dt = ((freq as usize / 1000) * msec) / 250;// / 1000;
    let now = timer_count() as usize;
    println!("timer freq = {}, dt = {}, now = {}", freq, dt, now);
    let mut then = now;
    while now < then - (dt as usize) as usize {
        let count;
        unsafe {
            asm!("mrs {}, cntpct_el0", out(reg) count);
        }
        then = count;
    }
    println!("done waiting");
    println!("now = {}", timer_count());
}
