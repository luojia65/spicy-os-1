use super::kernel_stack::KERNEL_STACK;
use super::STACK_SIZE;
use crate::fs::{STDIN, STDOUT};
use crate::mem::{Flags, MemoryResult, VirtualAddress};
use crate::process::Process;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};
use core::ops::Range;
use rcore_fs::vfs::INode;
use riscv::register::sstatus;
use spin::{Mutex, RwLock};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ThreadId(usize);

type Context = riscv_sbi_rt::TrapFrame;

/// 线程的信息
pub struct Thread {
    /// 线程 ID
    id: ThreadId,
    /// 线程的栈
    stack: Range<VirtualAddress>,
    /// 用 `Mutex` 包装一些可变的变量
    inner: Mutex<ThreadInner>,
    /// 所属的进程
    process: Arc<RwLock<Process>>,
}

// todo: private
pub struct ThreadInner {
    /// 线程执行上下文
    ///
    /// 当且仅当线程被暂停执行时，`context` 为 `Some`
    context: Option<Context>,
    // 占用的资源等等
    /// 打开的文件
    pub descriptors: Vec<Arc<dyn INode>>,
}

/// 通过线程 ID 来判等
impl PartialEq for Thread {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// 通过线程 ID 来判等
///
/// 在 Rust 中，[`PartialEq`] trait 不要求任意对象 `a` 满足 `a == a`。
/// 将类型标注为 [`Eq`]，会沿用 `PartialEq` 中定义的 `eq()` 方法，
/// 同时声明对于任意对象 `a` 满足 `a == a`。
impl Eq for Thread {}

/// 通过线程 ID 来哈希
impl Hash for Thread {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.id.0);
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
            inner: Mutex::new(ThreadInner {
                context: Some(context),
                descriptors: vec![STDIN.clone(), STDOUT.clone()],
            }),
        });

        Ok(thread)
    }

    pub fn thread_id(&self) -> ThreadId {
        self.id
    }

    pub fn process(&self) -> Arc<RwLock<Process>> {
        self.process.clone()
    }

    pub fn inner(&self) -> spin::MutexGuard<ThreadInner> {
        self.inner.lock()
    }

    /// 准备执行一个线程
    ///
    /// 激活对应进程的页表，并返回其 Context
    pub fn prepare(&self) -> *mut Context {
        // 激活页表
        self.process.read().memory_set.activate();
        // 取出 Context
        let parked_frame = self.inner().context.take().unwrap();

        if self.process.read().is_user {
            // 用户线程则将 Context 放至内核栈顶
            KERNEL_STACK.push_context(parked_frame)
        } else {
            // 内核线程则将 Context 放至 sp 下
            let context = (parked_frame.sp - core::mem::size_of::<Context>()) as *mut Context;
            unsafe { *context = parked_frame };
            context
        }
    }

    /// 发生时钟中断后暂停线程，保存状态
    pub fn park(&self, context: Context) {
        // 检查目前线程内的 context 应当为 None
        let slot = &mut self.inner().context;
        assert!(slot.is_none());
        // 将 Context 保存到线程中
        slot.replace(context);
    }
}

/// 为线程构建初始 `Context`
pub fn new_context(
    stack_top: usize,
    entry_point: usize,
    arguments: Option<&[usize]>,
    is_user: bool,
) -> Context {
    /// 按照函数调用规则写入参数
    ///
    /// 没有考虑一些特殊情况，例如超过 8 个参数，或 struct 空间展开
    pub fn set_arguments(ctx: &mut Context, arguments: &[usize]) {
        assert!(arguments.len() <= 8);
        if arguments.len() >= 1 {
            ctx.a0 = arguments[0];
        }
        if arguments.len() >= 2 {
            ctx.a1 = arguments[1];
        }
        if arguments.len() >= 3 {
            ctx.a2 = arguments[2];
        }
        if arguments.len() >= 4 {
            ctx.a3 = arguments[3];
        }
        if arguments.len() >= 5 {
            ctx.a4 = arguments[4];
        }
        if arguments.len() >= 6 {
            ctx.a5 = arguments[5];
        }
        if arguments.len() >= 7 {
            ctx.a6 = arguments[6];
        }
        if arguments.len() >= 8 {
            ctx.a7 = arguments[7];
        }
    }
    let mut context = riscv_sbi_rt::TrapFrame {
        ..unsafe { core::mem::MaybeUninit::zeroed().assume_init() }
    };
    // 设置栈顶指针
    context.sp = stack_top;
    riscv_sbi::println!("SP: {:016x}", context.sp);
    pub fn bottom_ra_called() {
        riscv_sbi::println!("You shouldn't call this function")
    }
    context.ra = bottom_ra_called as usize;
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
