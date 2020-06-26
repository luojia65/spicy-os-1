#![no_std]
#![no_main]
#![feature(global_asm, llvm_asm)]

use opensbi_rt::println;
use opensbi_rt::sbi;
use opensbi_rt::interrupt::TrapFrame;
use riscv::register::scause::Scause;

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
        llvm_asm!("ebreak"::::"volatile");
    };
}

#[export_name = "ExceptionHandler"]
pub fn handle_exception(_trap_frame: &TrapFrame, scause: Scause, stval: usize) {
    panic!(
        "Exception occurred: {:?}; stval: 0x{:x}", scause.cause(), stval
    );
}
