.section ".text.exception"

__exception_vector_start:
  // synchronous
  .align  7
  ; mov     x0, #0
  msr sctlr_el1, #0
  mrs     x1, esr_el0
  mrs     x2, elr_el0
  mrs     x3, spsr_el0
  mrs     x4, far_el0
  b       __handle_exception

  // IRQ
  .align  7
  ; mov     x0, #1
  msr sctlr_el1, #0
  mrs     x1, esr_el0
  mrs     x2, elr_el0
  mrs     x3, spsr_el0
  mrs     x4, far_el0
  b       __handle_exception

  // FIQ
  .align  7
  ; mov     x0, #2
  msr sctlr_el1, #0
  mrs     x1, esr_el0
  mrs     x2, elr_el0
  mrs     x3, spsr_el0
  mrs     x4, far_el0
  b       __handle_exception

  // SError
  .align  7
  ; mov     x0, #3
  msr sctlr_el1, #0
  mrs     x1, esr_el0
  mrs     x2, elr_el0
  mrs     x3, spsr_el0
  mrs     x4, far_el0
  b       __handle_exception

