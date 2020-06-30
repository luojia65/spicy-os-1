use core::ops::Range;
use core::ops::{Add, AddAssign};
use lazy_static::lazy_static;
use bitflags::bitflags;
use bit_field::BitField;
use spin::Mutex;
use riscv_sbi::println;

// todo: on embedded devices, we know how much memory are there on SoC,
// but on pc or other platforms where we can install external memory,
// we should auto detect them other than hardcode it into linker script.

extern "Rust" {
    static _sframe: u8;
    static _eframe: u8;
}

/// 页 / 帧大小，必须是 2^n
pub const PAGE_SIZE: usize = 4096;

/// 内核使用线性映射的偏移量
pub const KERNEL_MAP_OFFSET: usize = 0xffff_ffff_0000_0000;

lazy_static! {
    /// 可以访问的内存区域起始地址
    pub static ref MEMORY_START_ADDRESS: PhysicalAddress = 
        PhysicalAddress(unsafe { &_sframe as *const _ as usize });
    /// 可以访问的内存区域结束地址
    pub static ref MEMORY_END_ADDRESS: PhysicalAddress = 
        PhysicalAddress(unsafe { &_eframe as *const _ as usize });
}

#[derive(Debug, Clone, Copy)]
pub struct PhysicalAddress(usize);

impl PhysicalAddress {
    /// 从物理地址经过线性映射取得 &mut 引用
    pub fn deref_kernel<T>(self) -> &'static mut T {
        VirtualAddress::from(self).deref()
    }

    /// 取得页内偏移
    pub fn page_offset(&self) -> usize {
        self.0 % PAGE_SIZE
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VirtualAddress(usize);

impl VirtualAddress {
    /// 从虚拟地址取得某类型的 &mut 引用
    pub fn deref<T>(self) -> &'static mut T {
        unsafe { &mut *(self.0 as *mut T) }
    }
    /// 取得页内偏移
    pub fn page_offset(&self) -> usize {
        self.0 % PAGE_SIZE
    }
}

/// 虚实地址之间的线性映射
impl From<PhysicalAddress> for VirtualAddress {
    fn from(pa: PhysicalAddress) -> Self {
        Self(pa.0 + KERNEL_MAP_OFFSET)
    }
}
/// 虚实地址之间的线性映射
impl From<VirtualAddress> for PhysicalAddress {
    fn from(va: VirtualAddress) -> Self {
        Self(va.0 - KERNEL_MAP_OFFSET)
    }
}

// Physical Page Number, memory region [PPN * 4K, (PPN + 1) * 4K)
#[derive(Debug, Clone, Copy, Default)]
pub struct Ppn(pub usize);

impl Ppn {
    // todo: const fn
    /// 将地址转换为页号，向下取整
    pub fn floor(address: PhysicalAddress) -> Self {
        let address = address.0;
        Self(address / PAGE_SIZE)
    }

    /// 将地址转换为页号，向上取整
    pub fn ceil(address: PhysicalAddress) -> Self {
        let address = address.0;
        Self(address / PAGE_SIZE + (address % PAGE_SIZE != 0) as usize)
    }
}

impl Add for Ppn {
    type Output = Ppn;

    fn add(self, rhs: Ppn) -> Self::Output {
        Ppn(self.0 + rhs.0)
    }
}

impl Add<usize> for Ppn {
    type Output = Ppn;

    fn add(self, rhs: usize) -> Self::Output {
        Ppn(self.0 + rhs)
    }
}

impl AddAssign for Ppn {
    fn add_assign(&mut self, rhs: Ppn) {
        self.0 += rhs.0
    }
}

/// 和 usize 相互转换
impl From<usize> for Ppn {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// 和 usize 相互转换
impl From<Ppn> for usize {
    fn from(value: Ppn) -> Self {
        value.0
    }
}

impl From<Ppn> for PhysicalAddress {
    /// 从页号转换为地址
    fn from(page_number: Ppn) -> Self {
        Self(page_number.0 * PAGE_SIZE)
    }
}

impl From<PhysicalAddress> for Ppn {
    /// 从地址转换为页号，直接进行移位操作
    ///
    /// 不允许转换没有对齐的地址，这种情况应当使用 `floor()` 和 `ceil()`
    fn from(address: PhysicalAddress) -> Self {
        assert!(address.0 % PAGE_SIZE == 0);
        Self(address.0 / PAGE_SIZE)
    }
}

impl Ppn {
    /// 从物理地址经过线性映射取得页面
    pub fn deref_kernel(self) -> &'static mut [u8; PAGE_SIZE] {
        PhysicalAddress::from(self).deref_kernel()
    }
}

// Virtual Page Number
#[derive(Debug, Clone, Copy)]
pub struct Vpn(pub usize);

impl Vpn {
    /// 得到一、二、三级页号
    pub fn levels(self) -> [usize; 3] {
        [
            self.0.get_bits(18..27),
            self.0.get_bits(9..18),
            self.0.get_bits(0..9),
        ]
    }
}

impl Vpn {
    // todo: const fn
    /// 将地址转换为页号，向下取整
    pub fn floor(address: VirtualAddress) -> Self {
        let address = address.0;
        Self(address / PAGE_SIZE)
    }

    /// 将地址转换为页号，向上取整
    pub fn ceil(address: VirtualAddress) -> Self {
        let address = address.0;
        Self(address / PAGE_SIZE + (address % PAGE_SIZE != 0) as usize)
    }
}

impl Vpn {
    /// 从虚拟地址取得页面
    pub fn deref(self) -> &'static mut [u8; PAGE_SIZE] {
        VirtualAddress::from(self).deref()
    }
}

/// 和 usize 相互转换
impl From<usize> for Vpn {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// 和 usize 相互转换
impl From<Vpn> for usize {
    fn from(value: Vpn) -> Self {
        value.0
    }
}

impl From<Vpn> for VirtualAddress {
    /// 从页号转换为地址
    fn from(page_number: Vpn) -> Self {
        Self(page_number.0 * PAGE_SIZE)
    }
}

impl From<VirtualAddress> for Vpn {
    /// 从地址转换为页号，直接进行移位操作
    ///
    /// 不允许转换没有对齐的地址，这种情况应当使用 `floor()` 和 `ceil()`
    fn from(address: VirtualAddress) -> Self {
        assert!(address.0 % PAGE_SIZE == 0);
        Self(address.0 / PAGE_SIZE)
    }
}

/// 虚实页号之间的线性映射
impl From<Ppn> for Vpn {
    fn from(ppn: Ppn) -> Self {
        Self(ppn.0 + KERNEL_MAP_OFFSET / PAGE_SIZE)
    }
}
/// 虚实页号之间的线性映射
impl From<Vpn> for Ppn {
    fn from(vpn: Vpn) -> Self {
        Self(vpn.0 - KERNEL_MAP_OFFSET / PAGE_SIZE)
    }
}

/// Sv39 结构的页表项
#[derive(Copy, Clone, Default)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    /// 将相应页号和标志写入一个页表项
    pub fn new(page_number: Ppn, flags: Flags) -> Self {
        Self(
            *0usize
                .set_bits(..8, flags.bits() as usize)
                .set_bits(10..54, page_number.into()),
        )
    }

    /// 获取页号
    pub fn page_number(&self) -> Ppn {
        Ppn::from(self.0.get_bits(10..54))
    }

    /// 获取地址
    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress::from(self.page_number())
    }

    /// 获取标志位
    pub fn flags(&self) -> Flags {
        unsafe { Flags::from_bits_unchecked(self.0.get_bits(..8) as u8) }
    }

    /// 是否为空（可能非空也非 Valid）
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn get_next_table<'a>(&self) -> &'a mut PageTable {
        self.address().deref_kernel()
    }
}

impl core::fmt::Debug for PageTableEntry {
    fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter
            .debug_struct("PageTableEntry")
            .field("value", &self.0)
            .field("page_number", &self.page_number())
            .field("flags", &self.flags())
            .finish()
    }
}

bitflags! {
    /// 页表项中的 8 个标志位
    #[derive(Default)]
    pub struct Flags: u8 {
        /// 有效位
        const VALID =       1 << 0;
        /// 可读位
        const READABLE =    1 << 1;
        /// 可写位
        const WRITABLE =    1 << 2;
        /// 可执行位
        const EXECUTABLE =  1 << 3;
        /// 用户位
        const USER =        1 << 4;
        /// 全局位，我们不会使用
        const GLOBAL =      1 << 5;
        /// 已使用位，用于替换算法
        const ACCESSED =    1 << 6;
        /// 已修改位，用于替换算法
        const DIRTY =       1 << 7;
    }
}

pub struct FrameTracker(Ppn);

impl FrameTracker {
    /// 帧的物理地址
    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress((self.0).0 * PAGE_SIZE)
    }

    /// 帧的物理页号
    pub fn page_number(&self) -> Ppn {
        self.0
    }
}

/// 帧在释放时会放回 [`static@FRAME_ALLOCATOR`] 的空闲链表中
impl Drop for FrameTracker {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.lock().dealloc(self);
    }
}

/// `FrameTracker` 可以 deref 得到对应的 `[u8; PAGE_SIZE]`
impl core::ops::Deref for FrameTracker {
    type Target = [u8; PAGE_SIZE];
    fn deref(&self) -> &Self::Target {
        self.page_number().deref_kernel()
    }
}

/// `FrameTracker` 可以 deref 得到对应的 `[u8; PAGE_SIZE]`
impl core::ops::DerefMut for FrameTracker {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.page_number().deref_kernel()
    }
}

lazy_static! {
    /// 帧分配器
    pub static ref FRAME_ALLOCATOR: Mutex<
        FrameAllocator<StackedAllocator>
    > = Mutex::new(FrameAllocator::new(
        Ppn::ceil(*MEMORY_START_ADDRESS)
        ..Ppn::floor(*MEMORY_END_ADDRESS)
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
    pub fn alloc(&mut self) -> MemoryResult<FrameTracker> {
        self.allocator
            .alloc()
            .ok_or("no available frame to allocate")
            .map(|offset| FrameTracker(self.start_ppn + offset))
    }

    /// 将被释放的帧添加到空闲列表的尾部
    ///
    /// 这个函数会在 [`FrameTracker`] 被 drop 时自动调用，不应在其他地方调用
    pub fn dealloc(&mut self, frame: &FrameTracker) {
        self.allocator
            .dealloc(frame.page_number().0 - self.start_ppn.0);
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

/// 存有 512 个页表项的页表
///
/// 注意我们不会使用常规的 Rust 语法来创建 `PageTable`。相反，我们会分配一个物理页，
/// 其对应了一段物理内存，然后直接把其当做页表进行读写。我们会在操作系统中用一个「指针」
/// [`PageTableTracker`] 来记录这个页表。
#[repr(C)]
pub struct PageTable {
    pub entries: [PageTableEntry; PAGE_SIZE / 8],
}

impl PageTable {
    /// 将页表清零
    pub fn zero_init(&mut self) {
        self.entries = [Default::default(); PAGE_SIZE / 8];
    }
}

/// 类似于 [`FrameTracker`]，用于记录某一个内存中页表
///
/// 注意到，「真正的页表」会放在我们分配出来的物理页当中，而不应放在操作系统的运行栈或堆中。
/// 而 `PageTableTracker` 会保存在某个线程的元数据中（也就是在操作系统的堆上），指向其真正的页表。
///
/// 当 `PageTableTracker` 被 drop 时，会自动 drop `FrameTracker`，进而释放帧。
pub struct PageTableTracker(pub FrameTracker);

impl PageTableTracker {
    /// 将一个分配的帧清零，形成空的页表
    pub fn new(frame: FrameTracker) -> Self {
        let mut page_table = Self(frame);
        page_table.zero_init();
        page_table
    }

    /// 获取物理页号
    pub fn page_number(&self) -> Ppn {
        self.0.page_number()
    }
}

impl core::ops::Deref for PageTableTracker {
    type Target = PageTable;
    fn deref(&self) -> &Self::Target {
        self.0.address().deref_kernel()
    }
}

impl core::ops::DerefMut for PageTableTracker {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.address().deref_kernel()
    }
}

/// 映射的类型
#[derive(Debug)]
pub enum MapType {
    /// 线性映射，操作系统使用
    Linear,
    /// 按帧分配映射
    Framed,
}

/// 一个映射片段（对应旧 tutorial 的 `MemoryArea`）
#[derive(Debug)]
pub struct Segment {
    /// 映射类型
    pub map_type: MapType,
    /// 所映射的虚拟地址
    pub range: Range<VirtualAddress>,
    /// 权限标志
    pub flags: Flags,
}

// get rid of this after `Step` is stabilized
struct RangeIter<A>(Range<A>);

impl Iterator for RangeIter<Vpn> {
    type Item = Vpn;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.start.0 < self.0.end.0 {
            let mut n = Vpn(self.0.start.0 + 1);
            core::mem::swap(&mut n, &mut self.0.start);
            Some(n)
        } else {
            None
        }
    }
}

impl Segment {
    /// 遍历对应的物理地址（如果可能）
    pub fn iter_mapped(&self) -> Option<impl Iterator<Item = Ppn>> {
        match self.map_type {
            // 线性映射可以直接将虚拟地址转换
            MapType::Linear => Some(RangeIter(self.page_range()).map(Ppn::from)),
            // 按帧映射无法直接获得物理地址，需要分配
            MapType::Framed => None,
        }
    }

    /// 将地址相应地上下取整，获得虚拟页号区间
    pub fn page_range(&self) -> Range<Vpn> {
        Vpn::floor(self.range.start)..Vpn::ceil(self.range.end)
    }
    
    pub fn iter(&self) -> impl Iterator<Item = Vpn> {
        // todo
        RangeIter(self.page_range())
    }
}

#[derive(Default)]
/// 某个线程的内存映射关系
pub struct Mapping {
    /// 保存所有使用到的页表
    page_tables: Vec<PageTableTracker>,
    /// 根页表的物理页号
    root_ppn: Ppn,
}

impl Mapping {
    /// 创建一个有根节点的映射
    pub fn new() -> MemoryResult<Mapping> {
        let root_table = PageTableTracker::new(FRAME_ALLOCATOR.lock().alloc()?);
        let root_ppn = root_table.page_number();
        Ok(Mapping {
            page_tables: vec![root_table],
            root_ppn,
        })
    }

    /// 找到给定虚拟页号的三级页表项
    ///
    /// 如果找不到对应的页表项，则会相应创建页表
    pub fn find_entry(&mut self, vpn: Vpn) -> MemoryResult<&mut PageTableEntry> {
        // 从根页表开始向下查询
        // 这里不用 self.page_tables[0] 避免后面产生 borrow-check 冲突（我太菜了）
        let root_table: &mut PageTable = PhysicalAddress::from(self.root_ppn).deref_kernel();
        let mut entry = &mut root_table.entries[vpn.levels()[0]];
        // println!("[{}] = {:x?}", vpn.levels()[0], entry);
        for vpn_slice in &vpn.levels()[1..] {
            if entry.is_empty() {
                // 如果页表不存在，则需要分配一个新的页表
                let new_table = PageTableTracker::new(FRAME_ALLOCATOR.lock().alloc()?);
                let new_ppn = new_table.page_number();
                // 将新页表的页号写入当前的页表项
                *entry = PageTableEntry::new(new_ppn, Flags::VALID);
                // 保存页表
                self.page_tables.push(new_table);
            }
            // 进入下一级页表（使用偏移量来访问物理地址）
            entry = &mut entry.get_next_table().entries[*vpn_slice];
        }
        // 此时 entry 位于第三级页表
        Ok(entry)
    }

    /// 为给定的虚拟 / 物理页号建立映射关系
    fn map_one(
        &mut self,
        vpn: Vpn,
        ppn: Ppn,
        flags: Flags,
    ) -> MemoryResult<()> {
        // 定位到页表项
        let entry = self.find_entry(vpn)?;
        assert!(entry.is_empty(), "virtual address is already mapped");
        // 页表项为空，则写入内容
        *entry = PageTableEntry::new(ppn, flags);
        Ok(())
    }

    /// 加入一段映射，可能会相应地分配物理页面
    ///
    /// - `init_data`
    ///     复制一段内存区域来初始化新的内存区域，其长度必须等于 `segment` 的大小。
    ///
    ///
    /// 未被分配物理页面的虚拟页号暂时不会写入页表当中，它们会在发生 PageFault 后再建立页表项。
    pub fn map(
        &mut self,
        segment: &Segment,
    ) -> MemoryResult<Vec<(Vpn, FrameTracker)>> {
        // segment 可能可以内部做好映射
        if let Some(ppn_iter) = segment.iter_mapped() {
            // segment 可以提供映射，那么直接用它得到 vpn 和 ppn 的迭代器
            println!("map {:x?}", segment.page_range());
            for (vpn, ppn) in segment.iter().zip(ppn_iter) {
                self.map_one(vpn, ppn, segment.flags)?;
            }
            Ok(vec![])
        } else {
            // 需要再分配帧进行映射
            // 记录所有成功分配的页面映射
            let mut allocated_pairs = vec![];
            for vpn in segment.iter() {
                let frame: FrameTracker = FRAME_ALLOCATOR.lock().alloc()?;
                println!("map {:x?} -> {:x?}", vpn, frame.page_number());
                self.map_one(vpn, frame.page_number(), segment.flags)?;
                allocated_pairs.push((vpn, frame));
            }
            Ok(allocated_pairs)
        }
    }
}
