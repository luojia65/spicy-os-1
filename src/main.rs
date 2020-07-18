#![no_std]
#![no_main]
#![feature(global_asm, llvm_asm)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(drain_filter)]
#![feature(slice_fill)]

mod algo;
mod driver;
mod fs;
mod kernel;
mod mem;
mod process;

use crate::process::{Process, Thread, PROCESSOR};
use riscv::register::{scause::Scause, sie, sip, time};
use riscv_sbi::{self as sbi, println, HartMask};
use riscv_sbi_rt::{entry, heap_start, interrupt, max_hart_id, pre_init, TrapFrame};

use linked_list_allocator::LockedHeap;
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE: usize = 0x100_0000; // 16MiB

#[cfg(target_pointer_width = "64")]
riscv_sbi_rt::boot_page_sv39! {
    (0xffffffff_80000000 => 0x00000000_80000000, rwx);
    (0xffffffff_00000000 => 0x00000000_00000000, rwx);
    (0x00000000_80000000 => 0x00000000_80000000, rwx);
}

#[pre_init]
unsafe fn pre_init() {
    println!("PreInit!")
}

extern crate alloc;

// 启动一个核，其它的核等待软中断
// 多核操作系统需要改这个函数
#[export_name = "_mp_hook"]
pub extern "C" fn mp_hook(hartid: usize, _dtb: usize) -> bool {
    if hartid == 0 {
        true
    } else {
        unsafe {
            sbi::legacy::clear_ipi();
            sie::set_ssoft();
            loop {
                riscv::asm::wfi();
                if sip::read().ssoft() {
                    break;
                }
            }
            sie::clear_ssoft();
            sbi::legacy::clear_ipi();
        }
        false
    }
}

#[entry]
fn main(hartid: usize, dtb_pa: usize) {
    println!("Hello, OpenSBI!");
    println!("hartid={}, dtb_pa={:#x}", hartid, dtb_pa);
    println!("spec_version = {:?}", sbi::base::get_spec_version());
    println!("impl_id      = {:?}", sbi::base::get_impl_id());
    println!("impl_version = {:?}", sbi::base::get_impl_version());
    println!("mvendorid    = {:?}", sbi::base::get_mvendorid());
    println!("marchid      = {:?}", sbi::base::get_marchid());
    println!("mimpid       = {:?}", sbi::base::get_mimpid());

    if hartid == 0 {
        unsafe {
            HEAP_ALLOCATOR.lock().init(heap_start() as usize, HEAP_SIZE);
        }
        // wake other harts
        let mut hart_mask = HartMask::all(max_hart_id());
        hart_mask.clear(0); // unset hart 0
        sbi::legacy::send_ipi(hart_mask);
    }

    use alloc::boxed::Box;
    use alloc::vec::Vec;
    let v = Box::new(5);
    assert_eq!(*v, 5);
    let mut vec = Vec::new();
    for i in 0..10000 {
        vec.push(i);
    }
    for i in 0..10000 {
        assert_eq!(vec[i], i);
    }
    println!("heap test passed");

    println!("frame start: {:016x?}", *mem::MEMORY_START_ADDRESS);
    println!("frame end: {:016x?}", *mem::MEMORY_END_ADDRESS);

    // 物理页分配
    for _ in 0..2 {
        let frame_0 = match mem::FRAME_ALLOCATOR.lock().alloc() {
            Result::Ok(frame_tracker) => frame_tracker,
            Result::Err(err) => panic!("{}", err),
        };
        let frame_1 = match mem::FRAME_ALLOCATOR.lock().alloc() {
            Result::Ok(frame_tracker) => frame_tracker,
            Result::Err(err) => panic!("{}", err),
        };
        println!("{:x?} and {:x?}", frame_0.address(), frame_1.address());
    }

    println!("Initializing page system");
    let remap = mem::MemorySet::new_kernel().unwrap();
    println!("Instance created");
    remap.activate();
    println!("Page system activated");
    // 允许内核读写用户态内存
    // 其实只需要在部分的syscall里打开就可以了
    // 第一次写操作系统，别忘了这玩意，否则会有莫名其妙的页异常
    unsafe { riscv::register::sstatus::set_sum() };

    // unsafe {
    //     llvm_asm!("ebreak"::::"volatile");
    // }

    driver::init(mem::PhysicalAddress(dtb_pa));
    fs::init();

    // let process = Process::new_kernel().unwrap();

    // for message in 0..4 {
    //     let thread = Thread::new(
    //         process.clone(),            // 使用同一个进程
    //         sample_process as usize,    // 入口函数
    //         Some(&[message]),           // 参数
    //     ).unwrap();
    //     PROCESSOR.get().add_thread(thread);
    // }
    start_user_thread("fibonacci");
    // for message in 5..8 {
    //     let thread = Thread::new(
    //         process.clone(),            // 使用同一个进程
    //         sample_process as usize,    // 入口函数
    //         Some(&[message]),           // 参数
    //     ).unwrap();
    //     PROCESSOR.get().add_thread(thread);
    // }

    // 把多余的 process 引用丢弃掉
    // drop(process);

    unsafe {
        // 开启 STIE，允许时钟中断
        sie::set_stimer();
        // // 开启 SIE（不是 sie 寄存器），允许内核态被中断打断
        // sstatus::set_sie();
    }
    // 设置下一次时钟中断
    const INTERVAL: u64 = 100000;
    sbi::legacy::set_timer(time::read64().wrapping_add(INTERVAL));

    process::PROCESSOR.get().run()
}

#[interrupt]
fn SupervisorSoft() {
    println!("SupervisorSoft!");
}

// fn sample_process(message: usize) {
//     for i in 0..1000000 {
//         if i % 200000 == 0 {
//             println!("thread {}", message);
//         }
//     }
// }

fn start_user_thread(app_name: &str) {
    use crate::fs::*;
    use xmas_elf::ElfFile;
    // 从文件系统中找到程序
    let app = fs::ROOT_INODE.find(app_name).unwrap();
    // 读取数据
    let data = app.readall().unwrap();
    // 解析 ELF 文件
    let elf = ElfFile::new(data.as_slice()).unwrap();
    // 利用 ELF 文件创建线程，映射空间并加载数据
    let process = Process::from_elf(&elf, true).unwrap();
    // 再从 ELF 中读出程序入口地址
    let thread = Thread::new(process, elf.header.pt2.entry_point() as usize, None).unwrap();
    // 添加线程
    PROCESSOR.get().add_thread(thread);
}

const INTERVAL: u64 = 100000;

// #[interrupt]
// fn SupervisorTimer() {

#[export_name = "SupervisorTimer"]
unsafe extern "C" fn supervisor_timer(
    context: &mut TrapFrame,
    _scause: Scause,
    _stval: usize,
) -> *mut TrapFrame {
    static mut TICKS: usize = 0;

    sbi::legacy::set_timer(time::read64().wrapping_add(INTERVAL));
    TICKS += 1;
    if TICKS % 100 == 0 {
        println!("100 ticks~");
    }

    PROCESSOR.get().prepare_next_thread(context)
}

#[export_name = "ExceptionHandler"]
pub fn handle_exception(
    trap_frame: &mut TrapFrame,
    scause: Scause,
    stval: usize,
) -> *mut TrapFrame {
    // println!(
    //     "Exception occurred: {:?}; stval: 0x{:x}, sepc: 0x{:x}",
    //     scause.cause(),
    //     stval,
    //     trap_frame.sepc
    // );
    use riscv::register::scause::{Exception, Trap};
    if scause.cause() == Trap::Exception(Exception::Breakpoint) {
        println!("Breakpoint at 0x{:x}", trap_frame.sepc);
        trap_frame.sepc += 2;
    }
    if scause.cause() == Trap::Exception(Exception::UserEnvCall) {
        // println!("Syscall at 0x{:x}", trap_frame.sepc);
        // println!("{:x?}", trap_frame);
        return kernel::syscall::syscall_handler(trap_frame);
    }
    trap_frame as *mut _
}
