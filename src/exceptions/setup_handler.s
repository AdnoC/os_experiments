.section ".text.exception"

.global __exception_vector_table
.balign 2048
__exception_vector_table:
// Using SP_EL0 stack
  b __handle_exception
// Ensure each entry is 128 bytes
// Using SP_ELx stack
.balign 0x80
  b __handle_interrupt
// From lower EL in aarch64
.balign 0x80
  b __handle_interrupt
// From lower EL in aarch32
.balign 0x80
  b __handle_exception
/*



  // synchronous
  .align  7
  mov     x0, #0
  mrs     x1, esr_el1
  mrs     x2, elr_el1
  mrs     x3, spsr_el1
  mrs     x4, far_el1
  b       __handle_exception

  // IRQ
  .align  7
  mov     x0, #1
  mrs     x1, esr_el1
  mrs     x2, elr_el1
  mrs     x3, spsr_el1
  mrs     x4, far_el1
  b       __handle_exception

  // FIQ
  .align  7
  mov     x0, #2
  mrs     x1, esr_el1
  mrs     x2, elr_el1
  mrs     x3, spsr_el1
  mrs     x4, far_el1
  b       __handle_exception

  // SError
  .align  7
  mov     x0, #3
  mrs     x1, esr_el1
  mrs     x2, elr_el1
  mrs     x3, spsr_el1
  mrs     x4, far_el1
  b       __handle_exception
  */
