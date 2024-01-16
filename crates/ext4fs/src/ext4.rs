extern crate alloc;

use core::mem::size_of;
use alloc::string::*;
use alloc::string;
use core::str;
use alloc::vec::*;


use super::defs::*;


// A function that takes a &str and returns a &[char]
pub fn get_name(name: [u8; 255], len: usize) -> Result<String, string::FromUtf8Error> {
    let mut v: Vec<u8> = Vec::new();
    for i in 0..len {
        v.push(name[i]);
    }
    let s = String::from_utf8(v);
    s
}

// 打印目录项的名称和类型
pub fn print_dir_entry(entry: &Ext4DirEntry) {
    let name = str::from_utf8(&entry.name[..entry.name_len as usize]).unwrap();
    let file_type = DirEntryType::from_bits(entry.file_type).unwrap();
    match file_type {
        DirEntryType::REG_FILE => log::info!("{}: regular file", name),
        DirEntryType::DIR => log::info!("{}: directory", name),
        DirEntryType::CHRDEV => log::info!("{}: character device", name),
        DirEntryType::BLKDEV => log::info!("{}: block device", name),
        DirEntryType::FIFO => log::info!("{}: fifo", name),
        DirEntryType::SOCK => log::info!("{}: socket", name),
        DirEntryType::SYMLINK => log::info!("{}: symbolic link", name),
        _ => log::info!("{}: unknown type", name),
    }
}

pub fn ext4_path_check(path: &str, is_goal: &mut bool) -> usize {
    for (i, c) in path.chars().enumerate() {
        if c == '/' {
            *is_goal = false;
            return i;
        }
    }
    let path = path.to_string();
    *is_goal = true;
    return path.len();
}

pub fn ext4_first_extent(hdr: *const Ext4ExtentHeader) -> *const Ext4Extent {
    // 使用unsafe块，因为涉及到裸指针的操作
    unsafe {
        let offset = core::mem::size_of::<Ext4ExtentHeader>();

        (hdr as *const u8).add(offset) as *const Ext4Extent
    }
}

pub fn ext4_last_extent(hdr: *const Ext4ExtentHeader) -> *const Ext4Extent {
    // 使用unsafe块，因为涉及到裸指针的操作
    unsafe {
        // 使用core::mem::size_of!宏来计算ExtentHeader结构体的大小
        let hdr_size = core::mem::size_of::<Ext4ExtentHeader>();
        // 使用core::mem::size_of!宏来计算Extent结构体的大小
        let ext_size = core::mem::size_of::<Ext4Extent>();
        // 使用core::mem::transmute函数来将裸指针转换为引用
        let hdr_ref = core::mem::transmute::<*const Ext4ExtentHeader, &Ext4ExtentHeader>(hdr);
        // 使用eh_entries域来获取extent的个数
        let ext_count = hdr_ref.eh_entries as usize;
        // 使用add方法来计算指向最后一个Extent的裸指针
        (hdr as *const u8).add(hdr_size + (ext_count - 1) * ext_size) as *const Ext4Extent
    }
}


pub fn ext_inode_hdr(inode: &Ext4Inode) -> *const Ext4ExtentHeader {
    let eh = &inode.block as *const [u32; 15] as *const Ext4ExtentHeader;
    eh
}
