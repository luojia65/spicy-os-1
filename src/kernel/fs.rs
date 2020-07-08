use riscv_sbi_rt::TrapFrame as Context;
use crate::PROCESSOR;
use super::syscall::*;
use rcore_fs::vfs::INode;
use alloc::sync::Arc;

const FUNCTION_FS_READ: usize = 0x10002000;
const FUNCTION_FS_WRITE: usize = 0x30004000;

pub fn module_fs(function: usize, param0: usize, param1: usize, param2: usize) -> SyscallResult {
    match function {
        FUNCTION_FS_READ => function_fs_read(param0, param1 as *const u8 as *mut _, param2),
        FUNCTION_FS_WRITE => function_fs_write(param0, param1 as *const u8, param2),
        _ => unimplemented!()
    }
}

fn function_fs_read(fd: usize, buffer: *mut u8, size: usize) -> SyscallResult {
    // 从线程中获取 inode，注意避免锁
    let inode: Arc<dyn INode> =
        if let Some(inode) = PROCESSOR.get().current_thread().inner().descriptors.get(fd) {
            inode.clone()
        } else {
            return SyscallResult::ProceedTwo(0, 1); // err = 1
        };
    let buffer = unsafe { core::slice::from_raw_parts_mut(buffer, size) };
    if let Ok(ret) = inode.read_at(0, buffer) {
        let ret = ret as isize;
        if ret > 0 {
            return SyscallResult::ProceedTwo(ret, 0);
        }
        if ret == 0 {
            return SyscallResult::ParkTwo(ret, 0);
        }
    }
    SyscallResult::ProceedTwo(0, 1) // err = 1
}

fn function_fs_write(fd: usize, buffer: *const u8, size: usize) -> SyscallResult {
    if let Some(inode) = PROCESSOR.get().current_thread().inner().descriptors.get(fd) {
        let buffer = unsafe { core::slice::from_raw_parts(buffer, size) };
        if let Ok(ret) = inode.write_at(0, buffer) {
            let ret = ret as isize;
            if ret >= 0 {
                return SyscallResult::ProceedTwo(ret, 0);
            }
        }
    }
    SyscallResult::ProceedTwo(0, 1)
}
