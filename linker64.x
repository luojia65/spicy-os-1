MEMORY {
    /* 起始地址 */
	DRAM : ORIGIN = 0x80000000, LENGTH = 128M - 128K
}

/* 通常情况下都是从REGION_TEXT区域的起始位置运行的，但qemu的opensbi规定了入口位置，就把程序放在这里 */
PROVIDE(_stext = 0x80200000);

REGION_ALIAS("REGION_TEXT", DRAM);
REGION_ALIAS("REGION_RODATA", DRAM);
REGION_ALIAS("REGION_DATA", DRAM);
REGION_ALIAS("REGION_BSS", DRAM);
REGION_ALIAS("REGION_HEAP", DRAM);
REGION_ALIAS("REGION_STACK", DRAM);

/* 这个可以自己定义，剩下的就是stack的长度 */
PROVIDE(_heap_size = 0x4000000);
/* 双核系统，这个部分可以自己定义 */
PROVIDE(_max_hart_id = 1);
