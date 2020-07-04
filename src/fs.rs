use lazy_static::lazy_static;
use alloc::sync::Arc;
use crate::driver::{Driver, DeviceType, DRIVERS, block::BlockDevice};
use rcore_fs::{dev::block_cache::BlockCache, vfs::*};
use rcore_fs_sfs::SimpleFileSystem;
use riscv_sbi::{print, println};

/// 块设备的 Cache 块个数
pub const BLOCK_CACHE_CAPACITY: usize = 0x10;

lazy_static! {
    /// 根文件系统的根目录的 INode
    pub static ref ROOT_INODE: Arc<dyn INode> = {
        // 选择第一个块设备
        for driver in DRIVERS.read().iter() {
            if driver.device_type() == DeviceType::Block {
                let device = BlockDevice(driver.clone());
                // 动态分配一段内存空间作为设备 Cache
                let device_with_cache = Arc::new(BlockCache::new(device, BLOCK_CACHE_CAPACITY));
                return SimpleFileSystem::open(device_with_cache)
                    .expect("failed to open SFS")
                    .root_inode();
            }
        }
        panic!("failed to load fs")
    };
}

/// 打印某个目录的全部文件
pub fn ls(path: &str) {
    let mut id = 0;
    let dir = ROOT_INODE.lookup(path).unwrap();
    print!("files in {}: \n  ", path);
    while let Ok(name) = dir.get_entry(id) {
        id += 1;
        print!("{} ", name);
    }
    print!("\n");
}

/// 触发 [`static@ROOT_INODE`] 的初始化并打印根目录内容
pub fn init() {
    ls("/");
    println!("mod fs initialized");
}
