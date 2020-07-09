use crate::PROCESSOR;
use riscv_sbi_rt::TrapFrame as Context;

const MODULE_PROCESS: usize = 0x23336666;
const MODULE_FS: usize = 0xF0114514;

pub enum SyscallResult {
    /// 继续执行，带返回值
    Proceed(isize),
    /// 返回两个值
    ProceedTwo(isize, isize),
    /// 记录返回值，但暂存当前线程
    Park(isize),
    /// 返回两个值
    ParkTwo(isize, isize),
    /// 丢弃当前 context，调度下一个线程继续执行
    Kill,
}

pub fn syscall_handler(context: &mut Context) -> *mut Context {
    // 无论如何处理，一定会跳过当前的 ecall 指令
    context.sepc += 4;

    let ans = match context.a0 {
        MODULE_PROCESS => super::process::module_process(context.a1, context.a2),
        MODULE_FS => super::fs::module_fs(context.a1, context.a2, context.a3, context.a4),
        _ => unimplemented!(),
    };

    match ans {
        SyscallResult::Proceed(ret) => {
            // 将返回值放入 context 中
            context.a0 = ret as usize;
            context
        }
        SyscallResult::ProceedTwo(ans, err) => {
            context.a0 = ans as usize;
            context.a1 = err as usize;
            context
        }
        SyscallResult::Park(ret) => {
            // 将返回值放入 context 中
            context.a0 = ret as usize;
            // 保存 context，准备下一个线程
            PROCESSOR.get().park_current_thread(context);
            PROCESSOR.get().prepare_next_thread(context)
        }
        SyscallResult::ParkTwo(ans, err) => {
            context.a0 = ans as usize;
            context.a1 = err as usize;
            PROCESSOR.get().park_current_thread(context);
            PROCESSOR.get().prepare_next_thread(context)
        }
        SyscallResult::Kill => {
            // 终止，跳转到 PROCESSOR 调度的下一个线程
            PROCESSOR.get().kill_current_thread();
            PROCESSOR.get().prepare_next_thread(context)
        }
    }
}
