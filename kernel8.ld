/* https://github.com/rlepigre/bare-metal-aarch64/blob/master/step-01/kernel8.ld */
/* Entry point defined in file "boot.S". */
ENTRY(_start)

SECTIONS {
  . = 0x0;
  /* Our kernel image will be placed at address 0x80000. */
  . = 0x80000;
  __kernel_stack_start = .;
  __start = .;
  __text_start = .;
  /* It starts with the ".text" segment. */
  .text : {
    /* The ".text" segment itself starts with the code from "boot.S". */
    /* The "_start" symbol (at the beginning of "boot.S") is at 0x80000. */
    *(.text.boot)
    *(.text.__start_kernel) /* Provided by main.rs */
    *(.text*) /* Everything else? */
  }
  __text_end = .;

  /* Read-only data segment (for initialised const C global variables). */
  .rodata : ALIGN(8) {
    __rodata_start = .;
    *(.rodata*)
    __rodata_end = .;
  }

  /* Data segment (for initialised, non-const C global variables). */
  .data : {
    __data_start = .;
    *(.data*)
    __data_end = .;
  }

  /* BSS segment (for uninitialised C global variables). */
  /* BSS stands for "block starting symbol". */
  /* The BSS segment must be zeroed prior to entering C code. */
  .bss (NOLOAD) : ALIGN(16) {
    __bss_start = .;
    *(.bss);
    . = ALIGN(16);
    __bss_end = .;
  }
  __end = .;
}
