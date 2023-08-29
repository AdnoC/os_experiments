.section ".text.exception"

__exception_vector_start:
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
