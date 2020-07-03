#![no_std]
#![no_main]
#![feature(global_asm, llvm_asm)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(drain_filter)]
#![feature(slice_fill)]

mod algo;
mod mem;
mod process;

use riscv::register::{scause::Scause, sie, sip, sstatus, time};
use riscv_sbi::{self as sbi, println};
use riscv_sbi_rt::{entry, interrupt, pre_init, TrapFrame};

use linked_list_allocator::LockedHeap;
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE: usize = 0x100_0000;

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[pre_init]
unsafe fn pre_init() {
    println!("PreInit!")
}

extern crate alloc;

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
fn main(hartid: usize, dtb: usize) {
    println!("Hello, OpenSBI!");
    println!("hartid={}, dtb={:#x}", hartid, dtb);
    println!("spec_version = {:?}", sbi::base::get_spec_version());
    println!("impl_id      = {:?}", sbi::base::get_impl_id());
    println!("impl_version = {:?}", sbi::base::get_impl_version());
    println!("mvendorid    = {:?}", sbi::base::get_mvendorid());
    println!("marchid      = {:?}", sbi::base::get_marchid());
    println!("mimpid       = {:?}", sbi::base::get_mimpid());

    if hartid == 0 {
        unsafe {
            HEAP_ALLOCATOR
                .lock()
                .init(HEAP.as_ptr() as usize, HEAP_SIZE);
        }
        // wake other harts
        // let hart_mask: [usize; 4] = [1, 0, 0, 0];
        let hart_mask = 0b1110; // todo
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

    println!("frame start: {:?}", *mem::MEMORY_START_ADDRESS);
    println!("frame end: {:?}", *mem::MEMORY_END_ADDRESS);

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

    // unsafe {
    //     llvm_asm!("ebreak"::::"volatile");
    // }

    // 新建一个带有内核映射的进程。需要执行的代码就在内核中
    let process = process::Process::new_kernel().unwrap();

    for message in 0..8 {
        println!("message: {}", message);
        let thread = process::Thread::new(
            process.clone(),         // 使用同一个进程
            sample_process as usize, // 入口函数
            Some(&[message]),        // 参数
        )
        .unwrap();
        process::PROCESSOR.get().add_thread(thread);
    }

    // 把多余的 process 引用丢弃掉
    drop(process);

    unsafe {
        // 开启 STIE，允许时钟中断
        sie::set_stimer();
        // // 开启 SIE（不是 sie 寄存器），允许内核态被中断打断
        sstatus::set_sie();
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

fn sample_process(message: usize) {
    for i in 0..1000000 {
        if i % 200000 == 0 {
            println!("thread {}", message);
        }
    }
}

const INTERVAL: u64 = 100000;

// #[interrupt]
// fn SupervisorTimer() {

#[export_name = "SupervisorTimer"]
unsafe extern "C" fn supervisor_timer(context: &mut TrapFrame) -> *mut TrapFrame {
    static mut TICKS: usize = 0;
    use crate::process::PROCESSOR;

    sbi::legacy::set_timer(time::read64().wrapping_add(INTERVAL));
    TICKS += 1;
    if TICKS % 100 == 0 {
        println!("100 ticks~");
    }

    PROCESSOR.get().tick(context)
}

#[export_name = "ExceptionHandler"]
pub fn handle_exception(
    trap_frame: &mut TrapFrame,
    scause: Scause,
    stval: usize,
) -> *mut TrapFrame {
    println!(
        "Exception occurred: {:?}; stval: 0x{:x}, sepc: 0x{:x}",
        scause.cause(),
        stval,
        trap_frame.sepc
    );
    use riscv::register::scause::{Exception, Trap};
    if scause.cause() == Trap::Exception(Exception::Breakpoint) {
        println!("Breakpoint at 0x{:x}", trap_frame.sepc);
        trap_frame.sepc += 2;
    }
    trap_frame as *mut _
}
