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

// TODO: Verify # of cycles is correct
pub fn wait_cycle(mut num: usize) {
    while num > 0 {
        num -= 1;
        unsafe { asm!("nop") };
    }
}
pub fn wait_microsec(msec: usize) {
    let freq = timer_frequency();
    let dt = ((freq as usize / 1000) * msec) / 1000;
    let then = timer_count() as usize;
    println!("timer freq = {}, dt = {}, then = {}, target = {}", freq, dt, then, then + dt);
    let mut now = then;
    let target = now.saturating_add(dt);
    while now < target {
        let count;
        unsafe {
            asm!("mrs {}, cntpct_el0", out(reg) count);
        }
        now = count;
    }
    println!("done waiting");
    println!("now = {}", timer_count());
}
