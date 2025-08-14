OUTPUT_ARCH( "riscv" )
ENTRY( boot )

SECTIONS
{
  . = 0x80200000;
  __onix_kernel_start = .;
  __onix_rx_start = .;

  .text : ALIGN(16) {
    KEEP(*(.text.entry));
    *(.text .text.*);
  }

  . = ALIGN(4096);

  __onix_rx_end = .;
  __onix_ro_start = .;

  .rodata : ALIGN(16) {
    . = ALIGN(16);
    __onix_rodata_start = .;
    *(.srodata .srodata.*); /* do not need to distinguish this from .rodata */
    . = ALIGN(16);
    *(.rodata .rodata.*);
    __onix_rodata_end = .;
  }

  . = ALIGN(4096);

  __onix_ro_end = .;
  __onix_rw_start = .;

  .data : ALIGN(16) {
    *(.sdata .sdata.*); /* do not need to distinguish this from .data */
    . = ALIGN(16);
    *(.data .data.*);
  }

  .percpu : ALIGN(16) {
    __onix_percpu_start = .;
    *(.percpu .percpu.*);
    __onix_percpu_end = .;
  }

  .bss : ALIGN(16) {
    __onix_bss_start = .;
    *(.sbss .sbss.*) /* do not need to distinguish this from .bss */
    . = ALIGN(16);
    *(.bss .bss.*);
    __onix_bss_end = .;
  }

  . = ALIGN(4096);

  __onix_rw_end = .;
  __onix_kernel_end = .;

  __onix_boot_stack_start = .;
  . += 128 * 1024; /* 128 KiB stack */
  __onix_boot_stack_top = .;
  __onix_boot_stack_end = .;
}
