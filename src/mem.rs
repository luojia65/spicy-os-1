use lazy_static::lazy_static;
// use riscv::register::satp;

// todo: on embedded devices, we know how much memory are there on SoC,
// but on pc or other platforms where we can install external memory,
// we should auto detect them other than hardcode it into linker script.

extern "Rust" {
    static _sstack: u8;
}

pub use self::address::*;
lazy_static! {
    /// 可以访问的内存区域起始地址
    pub static ref MEMORY_START_ADDRESS: PhysicalAddress =
        PhysicalAddress(unsafe { &_sstack as *const _ as usize } - KERNEL_MAP_OFFSET);
    /// 可以访问的内存区域结束地址
    pub static ref MEMORY_END_ADDRESS: PhysicalAddress =
        PhysicalAddress(0x8800_0000);
}
pub use self::memory_set::MemorySet;

mod address;
mod frame;
mod mapping;
mod memory_set;
mod page_table;
mod page_table_entry;
mod segment;

pub use self::frame::{FrameTracker, FRAME_ALLOCATOR};
pub use self::mapping::Mapping;
pub use self::page_table_entry::Flags;
pub use self::segment::{MapType, Segment};

pub type MemoryResult<T> = core::result::Result<T, &'static str>;
