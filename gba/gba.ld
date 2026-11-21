/* GBA memory map: code and payload in GamePak ROM, hot code and data in IWRAM. */
ENTRY(_start)

MEMORY {
    ewram (w!x) : ORIGIN = 0x02000000, LENGTH = 256K
    iwram (w!x) : ORIGIN = 0x03000000, LENGTH = 31K  /* top 1K: stacks + BIOS area */
    rom   (rx)  : ORIGIN = 0x08000000, LENGTH = 16M  /* upper 16M is guest RAM */
}

SECTIONS {
    .text : {
        KEEP(*(.text.crt0))
        *(.text .text.*)
        . = ALIGN(4);
    } >rom

    .rodata : {
        *(.rodata .rodata.*)
        . = ALIGN(4);
    } >rom

    /* Copied from ROM to IWRAM by crt0: the emulator loop and initialized data. */
    .iwram : {
        . = ALIGN(4);
        __iwram_start = .;
        *(.iwram .iwram.*)
        *(.data .data.*)
        . = ALIGN(4);
        __iwram_end = .;
    } >iwram AT>rom
    __iwram_lma = LOADADDR(.iwram);

    .bss (NOLOAD) : {
        . = ALIGN(4);
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } >iwram

    .ewram (NOLOAD) : {
        *(.ewram .ewram.*)
    } >ewram

    /DISCARD/ : {
        *(.ARM.exidx .ARM.exidx.*)
        *(.ARM.extab .ARM.extab.*)
    }
}
