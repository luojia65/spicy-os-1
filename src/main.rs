#![no_std]
#![no_main]
#![feature(global_asm, llvm_asm)]

use riscv::register::{scause::Scause, sie, sstatus, time};
use riscv_sbi::{self as sbi, println};
use riscv_sbi_rt::{entry, interrupt, pre_init, TrapFrame};

const INTERVAL: u64 = 100000;

#[pre_init]
unsafe fn pre_init() {
    println!("PreInit!")
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
    
    unsafe {
        // 开启 STIE，允许时钟中断
        sie::set_stimer();
        // 开启 SIE（不是 sie 寄存器），允许内核态被中断打断
        sstatus::set_sie();
    }
    // 设置下一次时钟中断
    sbi::legacy::set_timer(time::read64().wrapping_add(INTERVAL));
    unsafe {
        llvm_asm!("ebreak"::::"volatile");
    }
    loop {}
}

#[interrupt]
fn SupervisorTimer() {
    static mut TICKS: usize = 0;
    sbi::legacy::set_timer(time::read64().wrapping_add(INTERVAL));
    *TICKS += 1;
    if *TICKS % 100 == 0 {
        println!("100 ticks~");
    }
}

#[export_name = "ExceptionHandler"]
pub fn handle_exception(trap_frame: &mut TrapFrame, scause: Scause, stval: usize) {
    println!(
        "Exception occurred: {:?}; stval: 0x{:x}",
        scause.cause(),
        stval
    );
    use riscv::register::scause::{Exception, Trap};
    if scause.cause() == Trap::Exception(Exception::Breakpoint) {
        println!("Breakpoint at 0x{:x}", trap_frame.sepc);
        trap_frame.sepc += 2;
    }
}
