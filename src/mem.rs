use lazy_static::lazy_static;
// use riscv::register::satp;

// todo: on embedded devices, we know how much memory are there on SoC,
// but on pc or other platforms where we can install external memory,
// we should auto detect them other than hardcode it into linker script.

extern "Rust" {
    static _sframe: u8;
    static _eframe: u8;
}

use self::address::*;
lazy_static! {
    /// 可以访问的内存区域起始地址
    pub static ref MEMORY_START_ADDRESS: PhysicalAddress =
        PhysicalAddress(unsafe { &_sframe as *const _ as usize } - KERNEL_MAP_OFFSET);
    /// 可以访问的内存区域结束地址
    pub static ref MEMORY_END_ADDRESS: PhysicalAddress =
        PhysicalAddress(unsafe { &_eframe as *const _ as usize } - KERNEL_MAP_OFFSET);
}
pub use self::memory_set::MemorySet;

mod address;
mod frame;
mod page_table_entry;
mod mapping;
mod page_table;
mod memory_set;
mod segment;

pub(crate) use self::frame::FRAME_ALLOCATOR;

pub type MemoryResult<T> = core::result::Result<T, &'static str>;

