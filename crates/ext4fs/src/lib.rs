#![no_std]
#![feature(allocator_api)]
#![feature(new_uninit)]
extern crate alloc;

use alloc::string;
use alloc::string::*;
use alloc::vec;
use alloc::{sync::Arc, vec::Vec};
use bitflags::bitflags;
use core::cell::{RefCell, UnsafeCell};
use core::marker::PhantomData;
use core::mem::size_of;
use core::str;


mod blockdev;
mod defs;
mod ext4;

pub use blockdev::*;
pub use defs::*;
pub use ext4::*;

struct Ext4TraitsImpl {
    pub block_device: Arc<dyn BlockDevice>,
}

impl Ext4Traits for Ext4TraitsImpl {
    fn read_block(&self, offset: u64) -> Vec<u8> {
        let mut buf = vec![0u8; BLOCK_SIZE as usize];
        
        let blk_id = (offset / BLOCK_SIZE) as usize;
        log::info!("read_offset {:?}  blkid {:?}", offset, blk_id);
        self.block_device.read_block(blk_id, &mut buf);
        buf
    }
}

pub struct Ext4Fs {
    pub super_block: Ext4SuperBlock,
    block_device: Arc<dyn BlockDevice>,
    // phantomdata: PhantomData<A>,
}

impl Ext4Traits for Ext4Fs {
    fn read_block(&self, offset: u64) -> Vec<u8> {
        let mut buf = vec![0u8; BLOCK_SIZE as usize];

        self.block_device.read_block(offset as usize, &mut buf);

        buf
    }
}


impl Ext4Fs {
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Self {
        log::info!("---------------open-------------------");

        let mut data = [0u8; 4096];
        // block_device block_size=512  super_block in offset 1024
        block_device.read_block(BASE_OFFSET as usize, &mut data);
        let mut buf = [0u8; size_of::<Ext4SuperBlock>()];
        buf.copy_from_slice(&data[..size_of::<Ext4SuperBlock>()]);

        let super_block = unsafe { core::ptr::read(buf.as_ptr() as *const _) };
        // log::info!("super_block {:x?}", super_block);

        Self {
            super_block: super_block,
            block_device: block_device,
        }
    }

    pub fn root_inode(&self) -> Ext4Inode {
        // log::info!("super_block {:x?}", &self.super_block);
        let inode = self.ext4_read_inode(2, &self.super_block);
        inode
    }

    pub fn read_super_block(&self) -> Ext4SuperBlock {
        let data = self.read_block(BASE_OFFSET);
        let mut buf = [0u8; size_of::<Ext4SuperBlock>()];
        buf.copy_from_slice(&data[..size_of::<Ext4SuperBlock>()]);
        unsafe { core::ptr::read(buf.as_ptr() as *const _) }
    }

    // A function that takes a &str and returns a &[char]
    pub fn get_name(&self, name: [u8; 255], len: usize) -> Result<String, string::FromUtf8Error> {
        let mut v: Vec<u8> = Vec::new();
        for i in 0..len {
            v.push(name[i]);
        }
        let s = String::from_utf8(v);
        s
    }

    // 打印目录项的名称和类型
    pub fn print_dir_entry(&self, entry: &Ext4DirEntry) {
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

    pub fn ext4_add_extent(
        &self,
        inode: &Ext4Inode,
        depth: u16,
        data: &[u32],
        extents: &mut Vec<Ext4Extent>,
        first_level: bool,
    ) {
        let extent_header = Ext4ExtentHeader::from_bytes_u32(data);
        let extent_entries = extent_header.eh_entries;

        if depth == 0 {
            for en in 0..extent_entries {
                let idx = (3 + en * 3) as usize;
                let extent = Ext4Extent::from_bytes_u32(&data[idx..]);
                let ee_block = extent.ee_block;
                let ee_len = extent.ee_len;
                let ee_start_hi = extent.ee_start_hi;
                let ee_start_lo = extent.ee_start_lo;
                extents.push(extent)
            }

            return;
        }

        for en in 0..extent_entries {
            let idx = (3 + en * 3) as usize;
            if idx == 12 {
                break;
            }
            let extent_index = Ext4ExtentIndex::from_bytes_u32(&data[idx..]);
            log::info!("extent_index {:x?}", extent_index);
            let ei_leaf_lo = extent_index.ei_leaf_lo;
            let ei_leaf_hi = extent_index.ei_leaf_hi;
            let mut block = ei_leaf_lo;
            block |= ((ei_leaf_hi as u32) << 31) << 1;
            let data = self.read_block(block as u64 * BLOCK_SIZE);
            let data: Vec<u32> = unsafe { core::mem::transmute(data) };
            self.ext4_add_extent(inode, depth - 1, &data, extents, false);
        }
    }

    pub fn ext4_get_block_group(&self, block_group: u64, super_block: &Ext4SuperBlock) -> u64 {
        let block_size = BLOCK_SIZE;
        let dsc_cnt = block_size / super_block.desc_size as u64;
        let dsc_per_block = dsc_cnt;
        let dsc_id = block_group / dsc_cnt;
        let first_meta_bg = super_block.first_meta_bg;
        let first_data_block = super_block.first_data_block;

        let block_id = first_data_block as u64 + dsc_id + 1;

        let offset = (block_group % dsc_cnt) * super_block.desc_size as u64;

        // 读取组描述符表的数据块的内容
        let gd_block_data = self.read_block(block_id as u64 * BLOCK_SIZE);
        let gd_data = &gd_block_data[offset as usize..offset as usize + size_of::<GroupDesc>()];

        let mut gd = GroupDesc::default();

        let ptr = &mut gd as *mut GroupDesc as *mut u8;

        unsafe {
            core::ptr::copy_nonoverlapping(
                gd_data.as_ptr(),
                ptr,
                core::mem::size_of::<GroupDesc>(),
            );
        }

        let inode_table_blk_num =
            ((gd.bg_inode_table_hi as u64) << 32) | gd.bg_inode_table_lo as u64;

        return inode_table_blk_num;
    }

    // 从文件中读取inode
    pub fn ext4_read_inode(&self, inode: u64, super_block: &Ext4SuperBlock) -> Ext4Inode {
        // log::info!("read inode {:?}", inode);
        let inodes_per_group = super_block.inodes_per_group;
        let inode_size = super_block.inode_size as u64;
        let group = (inode - 1) / inodes_per_group as u64;
        let index = (inode - 1) % inodes_per_group as u64;

        let mut inode_table_blk_num = self.ext4_get_block_group(group, super_block);

        let mut offset = inode_table_blk_num * BLOCK_SIZE + index * inode_size;
        let data = self.read_block(offset);
        let mut buf = [0u8; size_of::<Ext4Inode>()];
        buf.copy_from_slice(&data[..size_of::<Ext4Inode>()]);
        unsafe { core::ptr::read(buf.as_ptr() as *const _) }
    }

    // 从文件中读取目录项
    pub fn read_dir_entry(&self, inode: u64, super_block: &Ext4SuperBlock) -> Vec<Ext4DirEntry> {
        // 调用get_inode函数，根据inode编号，获取inode的内容，存入一个Inode类型的结构体中
        let inode_data = self.ext4_read_inode(inode, super_block);

        let mut extents: Vec<Ext4Extent> = Vec::new();
        self.ext4_find_extent(&inode_data, &mut extents);

        // 调用get_extents函数，根据inode的内容，获取inode的数据块的范围，存入一个Extent类型的向量中
        let e = &extents[0];

        // 创建一个空的DirEntry类型的向量entries，用来存放目录的目录项
        let mut entries = Vec::<Ext4DirEntry>::new();


        for e in extents{
            let blk_no: u64 = ((e.ee_start_hi as u64) << 32) | e.ee_start_lo as u64;
    
            for i in 0..e.ee_len{
                let block = self.read_block((blk_no + i as u64) * BLOCK_SIZE);
                let mut offset = 0;
                while offset < block.len() {
                    let de = Ext4DirEntry::from_bytes_offset(&block, offset);
                    offset = offset + de.rec_len as usize;
                    if de.inode == 0 {
                        continue;
                    }
                    // let s = self.get_name(de.name, de.name_len as usize);
                    // self.print_dir_entry(&de);   
                    entries.push(de);
                }
            }

        }

        entries
    }

    pub fn ext4_find_extent(&self, inode: &Ext4Inode, extents: &mut Vec<Ext4Extent>) {
        let extent_header = Ext4ExtentHeader::from_bytes_u32(&inode.block[..2]);

        let data = &inode.block;

        let mut depth = extent_header.eh_depth;

        self.ext4_add_extent(inode, depth, data, extents, true);
    }

    // 定义ext4_ext_binsearch函数，接受一个指向ext4_extent_path的可变引用和一个逻辑块号，返回一个布尔值，表示是否找到了对应的extent
    pub fn ext4_ext_binsearch(&self, path: &mut Ext4ExtentPath, block: ext4_lblk_t) -> bool {
        // 获取extent header的引用
        let eh = unsafe { &*path.header };

        // 定义左右两个指针，分别指向第一个和最后一个extent
        let mut l = unsafe { ext4_first_extent(eh).add(1) };
        let mut r = unsafe { ext4_last_extent(eh) };

        // 如果extent header中没有有效的entry，直接返回false
        if eh.eh_entries == 0 {
            return false;
        }
        // 使用while循环进行二分查找
        while l <= r {
            // 计算中间指针
            let m = unsafe { l.add((r as usize - l as usize) / 2) };
            // 获取中间指针所指向的extent的引用
            let ext = unsafe { &*m };
            // 比较逻辑块号和extent的第一个块号
            if block < ext.ee_block {
                // 如果逻辑块号小于extent的第一个块号，说明目标在左半边，将右指针移动到中间指针的左边
                r = unsafe { m.sub(1) };
            } else {
                // 如果逻辑块号大于或等于extent的第一个块号，说明目标在右半边，将左指针移动到中间指针的右边
                l = unsafe { m.add(1) };
            }
        }
        // 循环结束后，将path的extent字段设置为左指针的前一个位置
        path.extent = unsafe { l.sub(1) };
        // 返回true，表示找到了对应的extent
        true
    }

    pub fn ext4_find_extent_new(
        &self,
        inode: &Ext4Inode,
        iblock: ext4_lblk_t,
        orig_path: &mut Ext4ExtentPath,
        v: &mut Vec<Ext4ExtentPath>,
    ) {
        let eh = &inode.block as *const [u32; 15] as *const Ext4ExtentHeader;

        let extent_header = Ext4ExtentHeader::from_bytes_u32(&inode.block[..2]);
        let depth = extent_header.eh_depth;

        let mut extent_path = Ext4ExtentPath::default();
        extent_path.depth = depth;
        extent_path.header = eh;
        extent_path.block = Ext4Block::default();

        // depth = 0
        self.ext4_ext_binsearch(&mut extent_path, iblock);

        let extent = unsafe { *extent_path.extent };
        let pblock = extent.ee_start_lo | (((extent.ee_start_hi as u32) << 31) << 1);
        extent_path.p_block = pblock as u64;

        v.push(extent_path);

        // log::info!("v {:x?}", path_vec);

        // *orig_path = extent_path;
    }

    pub fn ext_inode_hdr(inode: &Ext4Inode) -> *const Ext4ExtentHeader {
        let eh = &inode.block as *const [u32; 15] as *const Ext4ExtentHeader;
        eh
    }

    pub fn ext4_extent_get_blocks(
        &self,
        inode: &Ext4Inode,
        iblock: ext4_lblk_t,
        max_blocks: u32,
        result: &mut ext4_fsblk_t,
        extent_create: bool,
    ) {
        let mut vec_extent_path: Vec<Ext4ExtentPath> = Vec::with_capacity(3);

        let mut extent_path = Ext4ExtentPath::default();

        self.ext4_find_extent_new(inode, iblock, &mut extent_path, &mut vec_extent_path);

        let depth = unsafe { *ext_inode_hdr(inode) }.eh_depth;

        let ex = unsafe { *vec_extent_path[depth as usize].extent };

        let ee_block = ex.ee_block;
        let ee_start = ex.ee_start_lo | (((ex.ee_start_hi as u32) << 31) << 1);
        let ee_len = ex.ee_len;

        if iblock >= ee_block && iblock <= (ee_block + ee_len as u32) {
            let newblock = iblock - ee_block + ee_start;
            *result = newblock as u64;

            return;
        }
    }

    pub fn ext4_fs_get_inode_dblk_idx(
        &self,
        inode_ref: &Ext4Inode,
        iblock: ext4_lblk_t,
        fblock: &mut ext4_fsblk_t,
        extent_create: bool,
    ) {
        let mut current_block: ext4_fsblk_t;
        let mut current_fsblk: ext4_fsblk_t = 0;
        self.ext4_extent_get_blocks(inode_ref, iblock, 1, &mut current_fsblk, false);

        current_block = current_fsblk;
        *fblock = current_block;
    }

    pub fn ext4_dir_find_in_block(
        &self,
        block: &Ext4Block,
        name_len: u32,
        name: &str,
        result: &mut Ext4DirSearchResult,
    ) -> bool {
        let mut offset = 0;

        while offset < block.data.len() {
            let de = Ext4DirEntry::from_bytes_offset(&block.data, offset);
            offset = offset + de.rec_len as usize;
            if de.inode == 0 {
                continue;
            }
            let s = get_name(de.name, de.name_len as usize);

            if let Ok(s) = s {
                if name_len == de.name_len as u32 {
                    if name.to_string() == s {
                        result.dentry.entry_len = de.rec_len;
                        result.dentry.name = de.name;
                        result.dentry.name_len = de.name_len;
                        result.dentry.name_length_high = de.file_type;
                        result.dentry.inode = de.inode;

                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn ext4_dir_find_entry(
        &self,
        parent: &Ext4Inode,
        name: &str,
        name_len: u32,
        result: &mut Ext4DirSearchResult,
    ) {
        let mut iblock: u32;
        let mut fblock: ext4_fsblk_t = 0;

        let inode_size: u32 = parent.size;
        let total_blocks: u32 = inode_size / BLOCK_SIZE as u32;

        /* Walk through all data blocks */
        iblock = 0;

        while iblock < total_blocks {
            self.ext4_fs_get_inode_dblk_idx(parent, iblock, &mut fblock, false);

            let mut b = Ext4Block::default();

            let data = self.read_block(fblock * BLOCK_SIZE);

            b.lb_id = iblock as u64;
            b.data = data;

            let r = self.ext4_dir_find_in_block(&mut b, name_len, name, result);

            if r {
                return;
            }

            iblock += 1;
        }

        // return ENOENT;
    }

    pub fn ext4_generic_open(&self, ext4_file: &mut Ext4File, path: &str) {
        let mp = &ext4_file.mp;

        let super_block = self.read_super_block();

        let mut dir_search_result = Ext4DirSearchResult::default();

        let mut is_goal = false;

        // start from root
        dir_search_result.dentry.inode = 2;


        let mut search_path = ext4_path_skip_dot(&path);

        
        let mut len = 0;
        loop {
            search_path = ext4_path_skip_slash(search_path);
            log::info!("search path {:?}", search_path);
            len = ext4_path_check(search_path, &mut is_goal);
            let inode_data =
                self.ext4_read_inode(dir_search_result.dentry.inode as u64, &super_block);

            self.ext4_dir_find_entry(
                &inode_data,
                &search_path[..len],
                len as u32,
                &mut dir_search_result,
            );

            let name = get_name(
                dir_search_result.dentry.name,
                dir_search_result.dentry.name_len as usize,
            )
            .unwrap();

            if is_goal{
                ext4_file.inode = dir_search_result.dentry.inode;
                return;
            }else{
                search_path = &search_path[len..];
            }
        }

    }

    pub fn ext4_file_read(&self, ext4_file: &Ext4File){
        let super_block = self.read_super_block();
        let inode_data = self.ext4_read_inode(ext4_file.inode as u64, &super_block);
        // let mut extents:Vec<Ext4Extent> = Vec::new();
        // ext4_find_extent(file, &inode_data, &mut extents);

        let size = inode_data.size;

        // 创建一个空的向量，用于存储文件的内容
        let mut file_data: Vec<u8> = Vec::new();

        // 创建一个空的向量，用于存储文件的所有extent信息
        let mut extents: Vec<Ext4Extent> = Vec::new();

        // 从inode_data中获取文件的所有extent信息，并存储在extents向量中
        self.ext4_find_extent(&inode_data, &mut extents);

        // 遍历extents向量，对每个extent，计算它的物理块号，然后调用read_block函数来读取数据块，并将结果追加到file_data向量中
        for extent in extents {
            // 获取extent的起始块号、块数和逻辑块号
            let start_block = extent.ee_start_lo as u64 | ((extent.ee_start_hi as u64) << 32);
            let block_count = extent.ee_len as u64;
            let logical_block = extent.ee_block as u64;

            // 计算extent的物理块号
            let physical_block = start_block + logical_block;

            // 从file中读取extent的所有数据块，并将结果追加到file_data向量中
            for i in 0..block_count {
                let block_num = physical_block + i;
                let block_data = self.read_block(block_num * BLOCK_SIZE);
                file_data.extend(block_data);
            }
        }
        
        log::info!("size {:x?}", size);
    }

    pub fn ext4_file_read_to_buf(&self, ext4_file: &Ext4File, buf: &mut [u8]) -> usize{
        let super_block = self.read_super_block();
        let inode_data = self.ext4_read_inode(ext4_file.inode as u64, &super_block);
        // let mut extents:Vec<Ext4Extent> = Vec::new();
        // ext4_find_extent(file, &inode_data, &mut extents);

        let size: usize = inode_data.size as usize;

        // 创建一个空的向量，用于存储文件的内容
        let mut file_data: Vec<u8> = Vec::new();

        // 创建一个空的向量，用于存储文件的所有extent信息
        let mut extents: Vec<Ext4Extent> = Vec::new();

        // 从inode_data中获取文件的所有extent信息，并存储在extents向量中
        self.ext4_find_extent(&inode_data, &mut extents);

        // 遍历extents向量，对每个extent，计算它的物理块号，然后调用read_block函数来读取数据块，并将结果追加到file_data向量中
        for extent in extents {
            // 获取extent的起始块号、块数和逻辑块号
            let start_block = extent.ee_start_lo as u64 | ((extent.ee_start_hi as u64) << 32);
            let block_count = extent.ee_len as u64;
            let logical_block = extent.ee_block as u64;

            // 计算extent的物理块号
            let physical_block = start_block + logical_block;

            // 从file中读取extent的所有数据块，并将结果追加到file_data向量中
            for i in 0..block_count {
                let block_num = physical_block + i;
                let block_data = self.read_block(block_num * BLOCK_SIZE);
                file_data.extend(block_data);
            }
        }

        log::info!("file size {:x?}", size);
        let chunk = &file_data[..size];
        buf[..size].copy_from_slice(chunk);
        
        size
    }

    pub fn ext4_find_all_disk_blocks(&self, ext4_file: &mut Ext4File){
        let super_block = self.read_super_block();
        let inode_data = self.ext4_read_inode(ext4_file.inode as u64, &super_block);

        let mut disk_blocks:Vec<u64> = Vec::new();

        // 创建一个空的向量，用于存储文件的所有extent信息
        let mut extents: Vec<Ext4Extent> = Vec::new();

        // 从inode_data中获取文件的所有extent信息，并存储在extents向量中
        self.ext4_find_extent(&inode_data, &mut extents);

        // 遍历extents向量，对每个extent，计算它的物理块号，然后调用read_block函数来读取数据块，并将结果追加到file_data向量中
        for extent in extents {
            // 获取extent的起始块号、块数和逻辑块号
            let start_block = extent.ee_start_lo as u64 | ((extent.ee_start_hi as u64) << 32);
            let block_count = extent.ee_len as u64;
            let logical_block = extent.ee_block as u64;

            // 计算extent的物理块号
            let physical_block = start_block + logical_block;

            // 从file中读取extent的所有数据块，并将结果追加到file_data向量中
            for i in 0..block_count {
                let block_num = physical_block + i;
                disk_blocks.push(block_num);
            }
        }

        
        ext4_file.all_blocks = disk_blocks;
        // ext4_file.extents = extents;
    }

    pub fn ext4_file_iter(&self, ext4_file: &mut Ext4File, buf: &mut [u8]) -> usize{
        
        let mut file_data: Vec<u8> = Vec::new();        
        
        let index = ext4_file.read_index;
        let all_blocks = &ext4_file.all_blocks;
        let block_num = all_blocks[index];
        let block_data = self.read_block(block_num * BLOCK_SIZE);
        file_data.extend(block_data);
        let chunk = &file_data[..4096];
        buf[..4096].copy_from_slice(chunk);
        ext4_file.read_index += 1;

        4096
    }

    pub fn ext4_file_inode_read(&self, ext4_file: &mut Ext4File) {
        let super_block = self.read_super_block();
        let inode_data = self.ext4_read_inode(ext4_file.inode as u64, &super_block);
    
        let size = inode_data.size;

        ext4_file.fsize = size as u64;
        ext4_file.inode_mode = inode_data.mode;
        ext4_file.flags = inode_data.flags;
        ext4_file.blocks = inode_data.blocks;
    }
}


pub fn ext4_path_skip_dot(path: &str) -> &str{
    let path_skip_dot = path.trim_start_matches(".");
    path_skip_dot
}

pub fn ext4_path_skip_slash(path: &str) -> &str{
    let path_skip_slash = path.trim_start_matches("/");
    path_skip_slash
}