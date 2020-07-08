use riscv_sbi_rt::TrapFrame as Context;
use riscv_sbi::println;
use crate::PROCESSOR;

const MODULE_PROCESS: usize = 0x23336666;
const FUNCTION_PROCESS_EXIT: usize = 0x99998888;

pub fn syscall_handler(context: &mut Context) -> *mut Context {
    // 无论如何处理，一定会跳过当前的 ecall 指令
    context.sepc += 4;

    // println!("Syscall! a0: {:x}, a1: {:x}", context.a0, context.a1);

    match context.a0 {
        MODULE_PROCESS => module_process(context),
        _ => context as *mut _
    }
}

fn module_process(context: &mut Context) -> *mut Context {
    match context.a1 {
        FUNCTION_PROCESS_EXIT => function_process_exit(context),
        _ => context as *mut _
    }
}

fn function_process_exit(context: &mut Context) -> *mut Context {
    let code = context.a2;
    // println!("code: {}", code);
    // let thread_id = PROCESSOR.get().current_thread().id;
    // println!("id: {:?}", thread_id);
    PROCESSOR.get().kill_current_thread();
    let ans = PROCESSOR.get().prepare_next_thread(context);
    println!(
        "[Kernel] Thread exited with code {}",
        // thread_id,
        code
    );
    ans
}
