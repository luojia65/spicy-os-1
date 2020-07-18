MEMORY {
    /* 存储单元的地址，可以是虚拟地址 */
	VIRT_DRAM : ORIGIN = 0xffffffff80000000, LENGTH = 24M /* 剩下的给frame分配 */
}

/* 通常情况下都是从REGION_TEXT区域的起始位置运行的，但qemu的opensbi规定了入口位置，就把程序放在这里 */
/* 这里用虚拟地址替换物理地址 */
PROVIDE(_stext = 0xffffffff80200000);
/* 如果要扩栈就改这个数 */
PROVIDE(_hart_stack_size = 128K);
/* 加核心的时候同时需要改这个数 */
PROVIDE(_max_hart_id = 1);
PROVIDE(_heap_size = 16M);

REGION_ALIAS("REGION_TEXT", VIRT_DRAM);
REGION_ALIAS("REGION_RODATA", VIRT_DRAM);
REGION_ALIAS("REGION_DATA", VIRT_DRAM);
REGION_ALIAS("REGION_BSS", VIRT_DRAM);
REGION_ALIAS("REGION_HEAP", VIRT_DRAM);
REGION_ALIAS("REGION_STACK", VIRT_DRAM);
