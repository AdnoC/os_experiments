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

// TODO: convert to macro with ASM so that it is exact # of cycles
pub fn wait_cycle(mut num: usize) {
    while num > 0 {
        num -= 1;
        unsafe { asm!("noop") };
    }
}
pub fn wait_microsec(msec: u64) {
    let freq = timer_frequency();
    let dt = ((freq as u64 / 1000) * msec) / 1000;
    let then = timer_count();
    println!(
        "timer freq = {}, dt = {}, then = {}, target = {}",
        freq,
        dt,
        then,
        then + dt
    );
    let target = then.saturating_add(dt) as u64;
    while timer_count() < target {}
    println!("done waiting");
    println!("now = {}", timer_count());
}
