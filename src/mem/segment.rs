//! 映射类型 [`MapType`] 和映射片段 [`Segment`]

use crate::mem::{address::*, page_table_entry::Flags};
use core::ops::Range;

/// 映射的类型
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MapType {
    /// 线性映射，操作系统使用
    Linear,
    /// 按帧分配映射
    Framed,
}

/// 一个映射片段（对应旧 tutorial 的 `MemoryArea`）
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Segment {
    /// 映射类型
    pub map_type: MapType,
    /// 所映射的虚拟地址
    pub range: Range<VirtualAddress>,
    /// 权限标志
    pub flags: Flags,
}

impl Segment {
    // /// 遍历对应的物理地址（如果可能）
    // pub fn iter_mapped(&self) -> Option<impl Iterator<Item = PhysicalPageNumber>> {
    //     match self.map_type {
    //         // 线性映射可以直接将虚拟地址转换
    //         MapType::Linear => Some(RangeIter(self.page_range()).map(PhysicalPageNumber::from)),
    //         // 按帧映射无法直接获得物理地址，需要分配
    //         MapType::Framed => None,
    //     }
    // }

    /// 将地址相应地上下取整，获得虚拟页号区间
    pub fn page_range(&self) -> Range<VirtualPageNumber> {
        VirtualPageNumber::floor(self.range.start)..VirtualPageNumber::ceil(self.range.end)
    }
}

// get rid of this after `Step` is stabilized
pub(crate) struct RangeIter<A>(pub Range<A>);

impl Iterator for RangeIter<VirtualPageNumber> {
    type Item = VirtualPageNumber;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.start.0 < self.0.end.0 {
            let mut n = VirtualPageNumber(self.0.start.0 + 1);
            core::mem::swap(&mut n, &mut self.0.start);
            Some(n)
        } else {
            None
        }
    }
}
