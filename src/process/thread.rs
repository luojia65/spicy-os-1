use super::kernel_stack::KERNEL_STACK;
use super::STACK_SIZE;
use crate::mem::{Flags, MemoryResult, VirtualAddress};
use crate::process::Process;
use alloc::sync::Arc;
use core::mem::size_of;
use core::ops::Range;
use riscv::register::sstatus;
use spin::{Mutex, RwLock};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ThreadId(usize);

type Context = riscv_sbi_rt::TrapFrame;

/// 为线程构建初始 `Context`
pub fn new_context(
    stack_top: usize,
    entry_point: usize,
    arguments: Option<&[usize]>,
    is_user: bool,
) -> Context {
    pub fn set_sp(ctx: &mut Context, value: usize) {
        ctx.x[2] = value;
    }
    pub fn set_ra(ctx: &mut Context, value: usize) {
        ctx.x[1] = value;
    }
    /// 按照函数调用规则写入参数
    ///
    /// 没有考虑一些特殊情况，例如超过 8 个参数，或 struct 空间展开
    pub fn set_arguments(ctx: &mut Context, arguments: &[usize]) {
        assert!(arguments.len() <= 8);
        ctx.x[10..(10 + arguments.len())].copy_from_slice(arguments);
    }
    let mut context = unsafe { core::mem::MaybeUninit::uninit().assume_init() };
    // 设置栈顶指针
    set_sp(&mut context, stack_top);
    set_ra(&mut context, -1isize as usize);
    // 设置初始参数
    if let Some(args) = arguments {
        set_arguments(&mut context, args);
    }
    // 设置入口地址
    context.sepc = entry_point;
    // 设置 sstatus
    context.sstatus = sstatus::read();
    if is_user {
        // context.sstatus.set_spp(User);
        unsafe {
            let mut a: usize = core::mem::transmute(context.sstatus);
            a &= !(1 << 8);
            context.sstatus = core::mem::transmute(a);
        }
    } else {
        // context.sstatus.set_spp(Supervisor);
        unsafe {
            let mut a: usize = core::mem::transmute(context.sstatus);
            a |= 1 << 8;
            context.sstatus = core::mem::transmute(a);
        }
    }
    // 这样设置 SPIE 位，使得替换 sstatus 后关闭中断，
    // 而在 sret 到用户线程时开启中断。详见 SPIE 和 SIE 的定义
    // context.sstatus.set_spie();
    unsafe {
        let mut a: usize = core::mem::transmute(context.sstatus);
        a |= 1 << 5;
        context.sstatus = core::mem::transmute(a);
    }
    context
}

#[derive(Debug)]
/// 线程的信息
pub struct Thread {
    /// 线程 ID
    pub id: ThreadId,
    /// 线程的栈
    pub stack: Range<VirtualAddress>,
    /// 线程执行上下文
    ///
    /// 当且仅当线程被暂停执行时，`context` 为 `Some`
    pub context: Mutex<Option<Context>>,
    /// 所属的进程
    pub process: Arc<RwLock<Process>>,
}

/// 通过线程 ID 来判等
impl PartialEq for Thread {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

static mut THREAD_COUNTER: usize = 0;

impl Thread {
    /// 创建一个线程
    pub fn new(
        process: Arc<RwLock<Process>>,
        entry_point: usize,
        arguments: Option<&[usize]>,
    ) -> MemoryResult<Arc<Thread>> {
        // 让所属进程分配并映射一段空间，作为线程的栈
        let stack = process
            .write()
            .alloc_page_range(STACK_SIZE, Flags::READABLE | Flags::WRITABLE)?;

        // 构建线程的 Context
        let context = new_context(
            stack.end.into(),
            entry_point,
            arguments,
            process.read().is_user,
        );

        // 打包成线程
        let thread = Arc::new(Thread {
            id: unsafe {
                THREAD_COUNTER += 1;
                ThreadId(THREAD_COUNTER)
            },
            stack,
            process,
            context: Mutex::new(Some(context)),
        });

        Ok(thread)
    }

    /// 准备执行一个线程
    ///
    /// 激活对应进程的页表，并返回其 Context
    pub fn run(&self) -> *mut Context {
        // 激活页表
        self.process.read().memory_set.activate();
        // 取出 Context
        let parked_frame = self.context.lock().take().unwrap();

        if self.process.read().is_user {
            // 用户线程则将 Context 放至内核栈顶
            KERNEL_STACK.push_context(parked_frame)
        } else {
            // 内核线程则将 Context 放至 sp 下
            let context = (parked_frame.x[2] - size_of::<Context>()) as *mut Context;
            unsafe { *context = parked_frame };
            context
        }
    }

    /// 发生时钟中断后暂停线程，保存状态
    pub fn park(&self, context: Context) {
        // 检查目前线程内的 context 应当为 None
        let mut slot = self.context.lock();
        assert!(slot.is_none());
        // 将 Context 保存到线程中
        slot.replace(context);
    }
}
