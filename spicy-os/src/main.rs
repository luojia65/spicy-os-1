#![no_std]
#![no_main]
#![feature(global_asm, llvm_asm)]

use opensbi_rt::println;
use opensbi_rt::sbi;
use riscv::register::{sie, sstatus, time};

static INTERVAL: u64 = 100000;

#[opensbi_rt::entry]
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
    loop {}
}

pub static mut TICKS: usize = 0;

#[export_name = "SupervisorTimer"]
fn on_timer() {
    sbi::legacy::set_timer(time::read64().wrapping_add(INTERVAL));
    unsafe {
        TICKS += 1;
        if TICKS % 100 == 0 {
            println!("100 ticks~");
        }
    };
}
