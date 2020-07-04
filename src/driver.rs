use crate::mem::{PhysicalAddress, VirtualAddress};

mod device_tree;
pub mod block;
mod virtio;

use riscv_sbi::println;

/// 从设备树的物理地址来获取全部设备信息并初始化
pub fn init(dtb_pa: PhysicalAddress) {
    let dtb_va = VirtualAddress::from(dtb_pa);
    device_tree::init(dtb_va);
    println!("mod driver initialized")
}


/// 驱动类型
///
/// 目前只有块设备，可能还有网络、GPU 设备等
#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DeviceType {
    Block,
}

use spin::RwLock;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;

/// 驱动的接口
pub trait Driver: Send + Sync {
    /// 设备类型
    fn device_type(&self) -> DeviceType;

    /// 读取某个块到 buf 中（块设备接口）
    fn read_block(&self, _block_id: usize, _buf: &mut [u8]) -> bool {
        unimplemented!("not a block driver")
    }

    /// 将 buf 中的数据写入块中（块设备接口）
    fn write_block(&self, _block_id: usize, _buf: &[u8]) -> bool {
        unimplemented!("not a block driver")
    }
}

lazy_static! {
    /// 所有驱动
    pub static ref DRIVERS: RwLock<Vec<Arc<dyn Driver>>> = RwLock::new(Vec::new());
}
