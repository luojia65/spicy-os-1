use crate::mem::MemorySet;

/// 进程的信息
pub struct Process {
    /// 线程属于的核心状态
    pub privilege: Privilege,
    /// 进程中的线程公用页表 / 内存映射
    pub memory_set: MemorySet,
}

pub enum Privilege {
    User,
    Supervisor,
}
