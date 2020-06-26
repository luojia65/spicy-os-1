#![no_std]
#![no_main]
#![feature(global_asm, llvm_asm)]
mod interrupt;

use opensbi_rt::println;
use opensbi_rt::sbi;

#[opensbi_rt::entry]
fn main(hartid: usize, dtb: usize) {
    interrupt::init();
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
