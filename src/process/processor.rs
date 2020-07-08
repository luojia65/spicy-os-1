use crate::algo::Scheduler;
use crate::algo::SchedulerImpl;
use crate::process::Thread;
use alloc::sync::Arc;
use hashbrown::HashSet;
use lazy_static::lazy_static;
use riscv_sbi_rt::TrapFrame;
use riscv_sbi::println;

mod unsafe_wrapper {
    use core::cell::UnsafeCell;

    /// 允许从 &self 获取 &mut 内部变量
    pub struct UnsafeWrapper<T> {
        object: UnsafeCell<T>,
    }

    impl<T> UnsafeWrapper<T> {
        #[allow(unused)]
        pub fn new(object: T) -> Self {
            Self {
                object: UnsafeCell::new(object),
            }
        }

        #[allow(clippy::mut_from_ref)]
        pub fn get(&self) -> &mut T {
            unsafe { &mut *self.object.get() }
        }
    }

    impl<T: Default> Default for UnsafeWrapper<T> {
        fn default() -> Self {
            Self {
                object: UnsafeCell::new(T::default()),
            }
        }
    }

    unsafe impl<T> Sync for UnsafeWrapper<T> {}
}

use unsafe_wrapper::UnsafeWrapper;
lazy_static! {
    /// 全局的 [`Processor`]
    pub static ref PROCESSOR: UnsafeWrapper<Processor> = Default::default();
}

/// 线程调度和管理
#[derive(Default)]
pub struct Processor {
    /// 当前正在执行的线程
    current_thread: Option<Arc<Thread>>,
    /// 线程调度器，记录所有线程
    scheduler: SchedulerImpl<Arc<Thread>>,
    /// 保存休眠线程
    sleeping_threads: HashSet<Arc<Thread>>,
}

impl Processor {
    /// 获取一个当前线程的 `Arc` 引用
    pub fn current_thread(&self) -> Arc<Thread> {
        self.current_thread.as_ref().unwrap().clone()
    }

    /// 第一次开始运行
    pub fn run(&mut self) -> ! {
        // interrupt.asm 中的标签
        extern "C" {
            fn __restore(context: usize);
        }
        if self.current_thread.is_none() {
            panic!("no thread to run, shutting down");
        }
        // 从 current_thread 中取出 Context
        let context = self.current_thread().prepare();
        // 从此将没有回头
        unsafe {
            __restore(context as usize);
        }
        unreachable!()
    }

    /// 在一个时钟中断时，替换掉 context
    pub fn prepare_next_thread(&mut self, context: &mut TrapFrame) -> *mut TrapFrame {
        loop {
            // 向调度器询问下一个线程
            if let Some(next_thread) = self.scheduler.get_next() {
                if next_thread == self.current_thread() {
                    // 没有更换线程，直接返回 Context
                    return context;
                } else {
                    // 准备下一个线程
                    let next_context = next_thread.prepare();
                    let current_thread = self.current_thread.replace(next_thread).unwrap();
                    // 储存当前线程 Context
                    current_thread.park(context.clone());
                    // 返回下一个线程的 Context
                    return next_context;
                }
            } else {
                // 没有活跃线程
                if self.sleeping_threads.is_empty() {
                    // 没有休眠线程，退出
                    println!("[Kernel] All threads terminated, shutting down");
                    riscv_sbi::legacy::shutdown()
                } else {
                    // 有休眠线程，等待中断
                    unsafe { riscv::asm::wfi() }
                }
            }
        }
    }

    /// 添加一个待执行的线程
    pub fn add_thread(&mut self, thread: Arc<Thread>) {
        if self.current_thread.is_none() {
            self.current_thread = Some(thread.clone());
        }
        // riscv_sbi::println!("[add_thread] add {:x?}", thread);
        self.scheduler.add_thread(thread, 0);
    }

    /// 唤醒一个休眠线程
    pub fn wake_thread(&mut self, thread: Arc<Thread>) {
        self.sleeping_threads.remove(&thread);
        self.scheduler.add_thread(thread, 0);
    }

    /// 保存当前线程的 `Context`
    pub fn park_current_thread(&mut self, context: &TrapFrame) {
        self.current_thread().park(*context);
    }

    /// 令当前线程进入休眠
    pub fn sleep_current_thread(&mut self) {
        // 从 current_thread 中取出
        let current_thread = self.current_thread();
        // 从 scheduler 移出到 sleeping_threads 中
        self.scheduler.remove_thread(&current_thread);
        self.sleeping_threads.insert(current_thread);
    }

    /// 终止当前的线程
    pub fn kill_current_thread(&mut self) {
        assert!(!self.current_thread.is_none());
        // 从调度器中移除
        let thread = self.current_thread.take().unwrap();
        self.scheduler.remove_thread(&thread);
    }
}
