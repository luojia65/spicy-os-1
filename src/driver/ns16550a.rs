#![allow(unused)]

use volatile::Volatile;
use device_tree::{util::SliceRead, Node};

#[repr(C)]
#[derive(Debug)]
struct Ns16550a {
    rbr_thr_dll: Volatile<u16>,
    dlh_ier: Volatile<u16>,
    iir_fcr: Volatile<u16>,
    lcr: Volatile<u16>,
    mcr: Volatile<u16>,
    lsr: Volatile<u16>,
    msr: Volatile<u16>,
    spr: Volatile<u16>,
}

pub fn ns16550a_probe(node: &Node) {
    // todo!!!
    // riscv_sbi::println!("{:#?}", node);
}
