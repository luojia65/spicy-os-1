use alloc::sync::Arc;
use crate::mem::VirtualAddress;
use spin::Mutex;

pub struct ThreadID(usize);

// /// 线程的信息
// pub struct Thread {
//     /// 线程 ID
//     pub id: ThreadID,
//     /// 线程的栈
//     pub stack: Range<VirtualAddress>,
//     /// 线程执行上下文
//     ///
//     /// 当且仅当线程被暂停执行时，`context` 为 `Some`
//     pub context: Mutex<Option<Context>>,
//     /// 所属的进程
//     pub process: Arc<RwLock<Process>>,
// }
