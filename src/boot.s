.section ".text.boot"
// Ensure the kernel's entry point is global
.globl _start
_start:
  mrs x5, CurrentEl // Move the CurrentEL system register into x5.
  ubfx x5, x5, #2, #2 // Extract the relevant bitfield (bits 3:2).


  // Set the SPSel register so that SP_EL0 is the stack pointer at all EL.
  mrs x6, SPSel        // Move the current SPSel  system register into x6.
  and x6, x6, ~1       // Clear the 0 bit of x6.
  msr SPSel, x6        // Set the value of SPSel to x6.

  // Set up the stack below our code (it grows downwards).
  // This should be plenty big enough: only the first 4KB of memory are used.
  ldr x6, =_start
  mov sp, x6

  ldr x6, =__bss_start
  ldr x7, =__bss_end
  b kernel_start
bss_clear_loop:
  cmp x6, x7
  b.ge bss_clear_done
  str xzr, [x6]
  add x6, x6, #8
  b bss_clear_loop
bss_clear_done:
