use core::mem::size_of;
use riscv_sbi_rt::TrapFrame as Context;

use super::{KERNEL_STACK_SIZE, STACK_SIZE};

/// 内核栈
#[repr(align(16))]
#[repr(C)]
pub struct KernelStack([u8; KERNEL_STACK_SIZE]);

/// 公用的内核栈
pub static KERNEL_STACK: KernelStack = KernelStack([0; STACK_SIZE]);

impl KernelStack {
    /// 在栈顶加入 Context 并且返回新的栈顶指针
    pub fn push_context(&self, context: Context) -> *mut Context {
        // 栈顶
        let stack_top = &self.0 as *const _ as usize + size_of::<Self>();
        riscv_sbi::println!("Top: {:p}", unsafe { stack_top as *mut () });
        riscv_sbi::println!("Bottom: {:p}", &self.0 as *const _);
        // Context 的位置
        let push_address = (stack_top - size_of::<Context>()) as *mut Context;
        riscv_sbi::println!("Push addr: {:p}", unsafe { push_address as *mut () });
        unsafe {
            *push_address = context;
        }
        push_address
    }
}
