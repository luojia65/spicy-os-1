/// 线程调度器
///
/// `ThreadType` 应为 `Arc<Thread>`
///
/// ### 使用方法
/// - 在每一个时间片结束后，调用 [`Scheduler::get_next()`] 来获取下一个时间片应当执行的线程。
///   这个线程可能是上一个时间片所执行的线程。
/// - 当一个线程结束时，需要调用 [`Scheduler::remove_thread()`] 来将其移除。这个方法必须在
///   [`Scheduler::get_next()`] 之前调用。
pub trait Scheduler<ThreadType: Clone + PartialEq>: Default {
    /// 向线程池中添加一个线程
    fn add_thread<T>(&mut self, thread: ThreadType, priority: T);
    /// 获取下一个时间段应当执行的线程
    fn get_next(&mut self) -> Option<ThreadType>;
    /// 移除一个线程
    fn remove_thread(&mut self, thread: &ThreadType);
    /// 设置线程的优先级
    fn set_priority<T>(&mut self, thread: ThreadType, priority: T);
}

pub type SchedulerImpl<T> = fifo_scheduler::FifoScheduler<T>;

mod hrrn_scheduler {
    //! 最高响应比优先算法的调度器 [`HrrnScheduler`]

    use super::Scheduler;
    use alloc::collections::LinkedList;

    /// 将线程和调度信息打包
    struct HrrnThread<ThreadType: Clone + PartialEq> {
        /// 进入线程池时，[`current_time`] 中的时间
        birth_time: usize,
        /// 被分配时间片的次数
        service_count: usize,
        /// 线程数据
        pub thread: ThreadType,
    }

    /// 采用 HRRN（最高响应比优先算法）的调度器
    pub struct HrrnScheduler<ThreadType: Clone + PartialEq> {
        /// 当前时间，单位为 `get_next()` 调用次数
        current_time: usize,
        /// 带有调度信息的线程池
        pool: LinkedList<HrrnThread<ThreadType>>,
    }

    /// `Default` 创建一个空的调度器
    impl<ThreadType: Clone + PartialEq> Default for HrrnScheduler<ThreadType> {
        fn default() -> Self {
            Self {
                current_time: 0,
                pool: LinkedList::new(),
            }
        }
    }

    impl<ThreadType: Clone + PartialEq> Scheduler<ThreadType> for HrrnScheduler<ThreadType> {
        fn add_thread<T>(&mut self, thread: ThreadType, _priority: T) {
            self.pool.push_back(HrrnThread {
                birth_time: self.current_time,
                service_count: 0,
                thread,
            })
        }
        fn get_next(&mut self) -> Option<ThreadType> {
            // 计时
            self.current_time += 1;

            // 遍历线程池，返回响应比最高者
            let current_time = self.current_time; // borrow-check
            if let Some(best) = self.pool.iter_mut().max_by(|x, y| {
                ((current_time - x.birth_time) * y.service_count)
                    .cmp(&((current_time - y.birth_time) * x.service_count))
            }) {
                best.service_count += 1;
                Some(best.thread.clone())
            } else {
                None
            }
        }
        fn remove_thread(&mut self, thread: &ThreadType) {
            // 移除相应的线程并且确认恰移除一个线程
            let mut removed = self.pool.drain_filter(|t| t.thread == *thread);
            assert!(removed.next().is_some() && removed.next().is_none());
        }
        fn set_priority<T>(&mut self, _thread: ThreadType, _priority: T) {}
    }
}

mod fifo_scheduler {
    use super::Scheduler;
    use alloc::collections::LinkedList;

    /// 采用 FIFO 算法的线程调度器
    pub struct FifoScheduler<ThreadType: Clone + PartialEq> {
        pool: LinkedList<ThreadType>,
    }

    /// `Default` 创建一个空的调度器
    impl<ThreadType: Clone + PartialEq> Default for FifoScheduler<ThreadType> {
        fn default() -> Self {
            Self {
                pool: LinkedList::new(),
            }
        }
    }

    impl<ThreadType: Clone + PartialEq + core::fmt::Debug> Scheduler<ThreadType>
        for FifoScheduler<ThreadType>
    {
        fn add_thread<T>(&mut self, thread: ThreadType, _priority: T) {
            // riscv_sbi::println!("[!] add thread! {:?}", thread);
            // 加入链表尾部
            self.pool.push_back(thread);
        }
        fn get_next(&mut self) -> Option<ThreadType> {
            // 从头部取出放回尾部，同时将其返回
            if let Some(thread) = self.pool.pop_front() {
                self.pool.push_back(thread.clone());
                Some(thread)
            } else {
                None
            }
        }
        fn remove_thread(&mut self, thread: &ThreadType) {
            // 移除相应的线程并且确认恰移除一个线程
            let mut removed = self.pool.drain_filter(|t| t == thread);
            assert!(removed.next().is_some() && removed.next().is_none());
        }
        fn set_priority<T>(&mut self, _thread: ThreadType, _priority: T) {}
    }
}
