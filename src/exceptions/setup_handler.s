.section ".text.exception"

.global __exception_vector_table
.balign 2048
__exception_vector_table:
// Using SP_EL0 stack
// First entry is Synchronous exception
  b .
// Ensure each entry is 128 bytes
// IRQ
.balign 0x80
  b .
// FIQ
.balign 0x80
  b .
// SError
.balign 0x80
  b .

// Using SP_ELx stack
.balign 0x80
  b exception_entry
.balign 0x80
  b interrupt_entry
.balign 0x80
  b interrupt_entry
.balign 0x80
  b exception_entry

// From lower EL in aarch64
.balign 0x80
  b .
.balign 0x80
  b .
.balign 0x80
  b .
.balign 0x80
  b .

// From lower EL in aarch32
.balign 0x80
  b .
.balign 0x80
  b .
.balign 0x80
  b .
.balign 0x80
  b .

exception_entry:
    sub sp, sp, #192
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x29, [sp, #144]
    stp x30, xzr, [sp, #160]

    mrs x0, ESR_EL1
    mrs x1, FAR_EL1
    stp x0, x1, [sp, #176]

    mov x0, sp
    bl __handle_exception

    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x29, [sp, #144]
    ldp x30, xzr, [sp, #160]
    add sp, sp, #192
    eret

interrupt_entry:
    sub sp, sp, #192
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x29, [sp, #144]
    stp x30, xzr, [sp, #160]

    mrs x0, ESR_EL1
    mrs x1, FAR_EL1
    stp x0, x1, [sp, #176]

    mov x0, sp
    bl __handle_interrupt

    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x29, [sp, #144]
    ldp x30, xzr, [sp, #160]
    add sp, sp, #192
    eret

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
