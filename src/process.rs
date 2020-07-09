mod kernel_stack;
mod processor;
mod thread;

pub use processor::PROCESSOR;
pub use thread::Thread;

/// 每个线程的运行栈大小 512 KB
pub const STACK_SIZE: usize = 0x8_0000;
/// 共用的内核栈大小 512 KB
pub const KERNEL_STACK_SIZE: usize = 0x8_0000;

use crate::mem::{Flags, MapType, MemoryResult, MemorySet, Segment, VirtualAddress, PAGE_SIZE};
use alloc::sync::Arc;
use core::ops::Range;
use spin::RwLock;
use xmas_elf::ElfFile;

#[derive(Clone, Copy, Debug)]
pub struct ProcessId(pub u32);

fn next_process_id() -> ProcessId {
    static mut PROCESS_COUNTER: u32 = 0;
    let ans = unsafe { PROCESS_COUNTER };
    unsafe { PROCESS_COUNTER += 1 };
    ProcessId(ans)
}

#[derive(Debug)]
/// 进程的信息
pub struct Process {
    /// 线程是否在用户态
    pub is_user: bool,
    /// 进程中的线程公用页表 / 内存映射
    pub memory_set: MemorySet,
    // id
    id: ProcessId,
}

impl Process {
    /// 创建一个内核进程
    pub fn new_kernel() -> MemoryResult<Arc<RwLock<Self>>> {
        Ok(Arc::new(RwLock::new(Self {
            is_user: false,
            memory_set: MemorySet::new_kernel()?,
            id: next_process_id(),
        })))
    }

    /// 创建进程，从文件中读取代码
    pub fn from_elf(file: &ElfFile, is_user: bool) -> MemoryResult<Arc<RwLock<Self>>> {
        Ok(Arc::new(RwLock::new(Self {
            is_user,
            memory_set: MemorySet::from_elf(file, is_user)?,
            id: next_process_id(),
        })))
    }

    /// 得到进程编号
    pub fn process_id(&self) -> ProcessId {
        self.id
    }

    /// 分配一定数量的连续虚拟空间
    ///
    /// 从 `memory_set` 中找到一段给定长度的未占用虚拟地址空间，分配物理页面并建立映射。返回对应的页面区间。
    ///
    /// `flags` 只需包括 rwx 权限，user 位会根据进程而定。
    pub fn alloc_page_range(
        &mut self,
        size: usize,
        flags: Flags,
    ) -> MemoryResult<Range<VirtualAddress>> {
        // memory_set 只能按页分配，所以让 size 向上取整页
        let alloc_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        // 从 memory_set 中找一段不会发生重叠的空间
        let mut range = VirtualAddress(0x1000000)..VirtualAddress(0x1000000 + alloc_size);
        while self
            .memory_set
            .overlap_with(range.start.into()..range.end.into())
        {
            range.start += alloc_size;
            range.end += alloc_size;
        }
        // 分配物理页面，建立映射
        self.memory_set.add_segment(
            Segment {
                map_type: MapType::Framed,
                range: range.clone(),
                flags: flags | Flags::user(self.is_user),
            },
            None,
        )?;
        // riscv_sbi::println!("range: {:?}", range);
        // riscv_sbi::println!("Memory set: {:?}", self.memory_set);
        // 返回地址区间（使用参数 size，而非向上取整的 alloc_size）
        Ok(Range::from(range.start..(range.start + size)))
    }
}
