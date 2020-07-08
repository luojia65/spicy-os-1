use riscv_sbi::println;
use crate::PROCESSOR;
use super::syscall::SyscallResult;

const FUNCTION_PROCESS_EXIT: usize = 0x99998888;
const FUNCTION_PROCESS_GET_ID: usize = 0x77776666;

pub fn module_process(function: usize, param0: usize) -> SyscallResult {
    match function {
        FUNCTION_PROCESS_EXIT => function_process_exit(param0),
        FUNCTION_PROCESS_GET_ID => function_process_get_id(),
        _ => unimplemented!()
    }
}

fn function_process_exit(code: usize) -> SyscallResult {
    let thread_id = PROCESSOR.get().current_thread().thread_id();
    println!(
        "[Kernel] Thread {:?} exited with code {}",
        thread_id,
        code
    );
    SyscallResult::Kill
}

fn function_process_get_id() -> SyscallResult {
    let process_id = PROCESSOR.get().current_thread().process().read().process_id();
    SyscallResult::Proceed(process_id.0 as isize)
}
