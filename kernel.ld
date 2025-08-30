ENTRY(boot)

SECTIONS {
    . = 0x80200000;
    __kernel_base = .;

    .text :{
        KEEP(*(.text.boot));
        *(.text .text.*);
    }

    .rodata : ALIGN(4) {
        *(.rodata .rodata.*);
    }

    .data : ALIGN(4) {
        *(.data .data.*);
    }

    .bss : ALIGN(4) {
        __bss = .;
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }

    . = ALIGN(4);
    . += 128 * 1024; /* 128KiB */
    __stack_top = .;

    . = ALIGN(4);
    __kernel_heap = .;
    . += 1024 * 1024 * 4; /* 4MiB */
    __kernel_heap_end = .;

    /* . = ALIGN(4096); */
    /* __free_ram = .; */
    /* . += 64 * 1024 * 1024; */
    /* __free_ram_end = .; */
}
