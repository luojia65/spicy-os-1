//! 一个线程中关于内存空间的所有信息 [`MemorySet`]
//!

use crate::mem::{
    address::*,
    frame::FrameTracker,
    mapping::Mapping,
    page_table_entry::Flags,
    segment::{MapType, Segment},
    MemoryResult,
};
use alloc::{vec, vec::Vec};
use core::ops::Range;

#[derive(Debug)]
/// 一个进程所有关于内存空间管理的信息
pub struct MemorySet {
    /// 维护页表和映射关系
    pub mapping: Mapping,
    /// 每个字段
    pub segments: Vec<Segment>,
    /// 所有分配的物理页面映射信息
    pub allocated_pairs: Vec<(VirtualPageNumber, FrameTracker)>,
}

impl MemorySet {
    /// 创建内核重映射
    pub fn new_kernel() -> MemoryResult<MemorySet> {
        // 在 linker.ld 里面标记的各个字段的起始点，均为 4K 对齐
        extern "C" {
            fn _stext();
            fn _etext();
            fn _srodata();
            fn _erodata();
            fn _sdata();
            fn _edata();
            fn _sbss();
            fn _ebss();
            fn _estack();
            fn _sstack();
            fn _sframe();
            fn _eframe();
        }

        // 建立字段
        let segments = vec![
            // DEVICE 段，rw-
            Segment {
                map_type: MapType::Linear,
                range: DEVICE_START_ADDRESS.into()..DEVICE_END_ADDRESS.into(),
                flags: Flags::READABLE | Flags::WRITABLE,
            },
            // .text 段，r-x
            Segment {
                map_type: MapType::Linear,
                range: (_stext as usize).into()..(_etext as usize).into(),
                flags: Flags::READABLE | Flags::EXECUTABLE,
            },
            // .rodata 段，r--
            Segment {
                map_type: MapType::Linear,
                range: (_srodata as usize).into()..(_erodata as usize).into(),
                flags: Flags::READABLE,
            },
            // .data 段，rw-
            Segment {
                map_type: MapType::Linear,
                range: (_sdata as usize).into()..(_edata as usize).into(),
                flags: Flags::READABLE | Flags::WRITABLE,
            },
            // .bss 段，rw-
            Segment {
                map_type: MapType::Linear,
                range: (_sbss as usize).into()..(_ebss as usize).into(),
                flags: Flags::READABLE | Flags::WRITABLE,
            },
            // frame, rw-
            Segment {
                map_type: MapType::Linear,
                range: (_sframe as usize).into()..(_eframe as usize).into(),
                flags: Flags::READABLE | Flags::WRITABLE,
            },
            // stack，rw-
            Segment {
                map_type: MapType::Linear,
                range: (_estack as usize).into()..(_sstack as usize).into(),
                flags: Flags::READABLE | Flags::WRITABLE,
            },
        ];
        let mut mapping = Mapping::new()?;
        // 准备保存所有新分配的物理页面
        let mut allocated_pairs = Vec::new();

        // 每个字段在页表中进行映射
        for segment in segments.iter() {
            // 同时将新分配的映射关系保存到 allocated_pairs 中
            allocated_pairs.extend(mapping.map(segment, None)?);
        }
        Ok(MemorySet {
            mapping,
            segments,
            allocated_pairs,
        })
    }

    /// 替换 `satp` 以激活页表
    ///
    /// 如果当前页表就是自身，则不会替换，但仍然会刷新 TLB。
    pub fn activate(&self) {
        self.mapping.activate();
    }

    /// 添加一个 [`Segment`] 的内存映射
    pub fn add_segment(&mut self, segment: Segment, init_data: Option<&[u8]>) -> MemoryResult<()> {
        // 检测 segment 没有重合
        assert!(!self.overlap_with(segment.page_range()));
        // 映射并将新分配的页面保存下来
        self.allocated_pairs
            .extend(self.mapping.map(&segment, init_data)?);
        self.segments.push(segment);
        Ok(())
    }

    /// 检测一段内存区域和已有的是否存在重叠区域
    pub fn overlap_with(&self, range: Range<VirtualPageNumber>) -> bool {
        fn range_overlap<T: core::cmp::PartialOrd>(a: &Range<T>, b: &Range<T>) -> bool {
            b.contains(&a.start) || a.contains(&b.start) || b.contains(&a.end) || a.contains(&b.end)
        }
        for seg in self.segments.iter() {
            if range_overlap(&range, &seg.page_range()) {
                return true;
            }
        }
        false
    }
}
