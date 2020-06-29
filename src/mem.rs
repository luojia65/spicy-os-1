use lazy_static::lazy_static;
use spin::Mutex;
use core::ops::Range;

lazy_static! {
    /// 内核代码结束的地址，即可以用来分配的内存起始地址
    ///
    /// 因为 Rust 语言限制，我们只能将其作为一个运行时求值的 static 变量，而不能作为 const
    pub static ref KERNEL_END_ADDRESS: PhysicalAddress = 
        PhysicalAddress(unsafe { &supervisor_end as *const _ as usize });
}

/// 页 / 帧大小，必须是 2^n
pub const PAGE_SIZE: usize = 4096;

/// 可以访问的内存区域起始地址
pub const MEMORY_START_ADDRESS: PhysicalAddress = PhysicalAddress(0x8000_0000);
/// 可以访问的内存区域结束地址
pub const MEMORY_END_ADDRESS: PhysicalAddress = PhysicalAddress(0x8800_0000);

extern "Rust" {
    /// 由 `linker.ld` 指定的内核代码结束位置
    ///
    /// 作为变量存在 [`KERNEL_END_ADDRESS`]
    static supervisor_end: u8;
}

#[derive(Debug, Clone, Copy)]
pub struct PhysicalAddress(usize);

// Physical Page Number, memory region [PPN * 4K, (PPN + 1) * 4K)
#[derive(Clone, Copy)]
pub struct Ppn(pub usize);

impl Ppn {
    /// 将地址转换为页号，向下取整
    pub const fn floor(address: PhysicalAddress) -> Self {
        Self(address.0 / PAGE_SIZE)
    }
    /// 将地址转换为页号，向上取整
    pub const fn ceil(address: PhysicalAddress) -> Self {
        Self(address.0 / PAGE_SIZE + (address.0 % PAGE_SIZE != 0) as usize)
    }
}

pub struct FrameHandle(PhysicalAddress);

impl FrameHandle {
    /// 帧的物理地址
    pub fn address(&self) -> PhysicalAddress {
        self.0
    }
    /// 帧的物理页号
    pub fn page_number(&self) -> Ppn {
        Ppn((self.0).0)
    }
}

/// 帧在释放时会放回 [`static@FRAME_ALLOCATOR`] 的空闲链表中
impl Drop for FrameHandle {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.lock().dealloc(self);
    }
}

pub struct FrameRange {
    range: Range<usize>,
}

lazy_static! {
    /// 帧分配器
    pub static ref FRAME_ALLOCATOR: Mutex<
        FrameAllocator<StackedAllocator>
    > = Mutex::new(FrameAllocator::new(
        Range::from(
            Ppn::ceil(PhysicalAddress::from(*KERNEL_END_ADDRESS))
            ..Ppn::floor(MEMORY_END_ADDRESS)
        )
    ));
}

/// 基于线段树的帧分配 / 回收
pub struct FrameAllocator<T: Allocator> {
    /// 可用区间的起始
    start_ppn: Ppn,
    /// 分配器
    allocator: T,
}

impl<T: Allocator> FrameAllocator<T> {
    /// 创建对象
    pub fn new(range: Range<Ppn>) -> Self {
        FrameAllocator {
            start_ppn: range.start,
            allocator: T::new(range.end.0 - range.start.0),
        }
    }

    /// 分配帧，如果没有剩余则返回 `Err`
    pub fn alloc(&mut self) -> MemoryResult<FrameHandle> {
        self.allocator
            .alloc()
            .ok_or("no available frame to allocate")
            .map(|offset| FrameHandle(PhysicalAddress(self.start_ppn.0 + offset)))
    }

    /// 将被释放的帧添加到空闲列表的尾部
    ///
    /// 这个函数会在 [`FrameHandle`] 被 drop 时自动调用，不应在其他地方调用
    pub fn dealloc(&mut self, frame: &FrameHandle) {
        self.allocator.dealloc(frame.page_number().0 - self.start_ppn.0);
    }
}

pub type MemoryResult<T> = core::result::Result<T, &'static str>;

/// 分配器：固定容量，每次分配 / 回收一个元素
pub trait Allocator {
    /// 给定容量，创建分配器
    fn new(capacity: usize) -> Self;
    /// 分配一个元素，无法分配则返回 `None`
    fn alloc(&mut self) -> Option<usize>;
    /// 回收一个元素
    fn dealloc(&mut self, index: usize);
}

use alloc::{vec, vec::Vec};

pub struct StackedAllocator {
    list: Vec<(usize, usize)>,
}

impl Allocator for StackedAllocator {
    fn new(capacity: usize) -> Self {
        Self {
            list: vec![(0, capacity)],
        }
    }

    fn alloc(&mut self) -> Option<usize> {
        if let Some((start, end)) = self.list.pop() {
            if end - start > 1 {
                self.list.push((start + 1, end));
            }
            Some(start)
        } else {
            None
        }
    }

    fn dealloc(&mut self, index: usize) {
        self.list.push((index, index + 1));
    }
}