use device_tree::{util::SliceRead, Node};
use virtio_drivers::{DeviceType, VirtIOHeader};
use crate::mem::{PhysicalAddress, VirtualAddress};
use riscv_sbi::println;

/// 从设备树的某个节点探测 virtio 协议具体类型
pub fn virtio_probe(node: &Node) {
    // reg 属性中包含了描述设备的 Header 的位置
    let reg = match node.prop_raw("reg") {
        Some(reg) => reg,
        _ => return,
    };
    let pa = PhysicalAddress(reg.as_slice().read_be_u64(0).unwrap() as usize);
    let va = VirtualAddress::from(pa);
    let header = unsafe { &mut *(va.0 as *mut VirtIOHeader) };
    // 目前只支持某个特定版本的 virtio 协议
    if !header.verify() {
        return;
    }
    // 判断设备类型
    match header.device_type() {
        DeviceType::Block => virtio_blk::add_driver(header),
        device => println!("unrecognized virtio device: {:?}", device),
    }
}
