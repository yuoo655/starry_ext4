use bitflags::bitflags;
use core::mem::size_of;
use alloc::vec::*;
use alloc::string::*;
// use alloc::alloc::Global;
use alloc::vec::IntoIter;

// 定义超级块结构体，参考 https://www.nongnu.org/ext2-doc/ext2.html#SUPERBLOCK
#[repr(C)]
#[derive(Debug)]
pub struct Ext4SuperBlock {
    pub inodes_count: u32,
    pub blocks_count: u32,
    pub r_blocks_count: u32,
    pub free_blocks_count: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub log_frag_size: u32,
    pub blocks_per_group: u32,
    pub frags_per_group: u32,
    pub inodes_per_group: u32,
    pub mtime: u32,
    pub wtime: u32,
    pub mnt_count: u16,
    pub max_mnt_count: u16,
    pub magic: u16,
    pub state: u16,
    pub errors: u16,
    pub minor_rev_level: u16,
    pub lastcheck: u32,
    pub checkinterval: u32,
    pub creator_os: u32,
    pub rev_level: u32,
    pub def_resuid: u16,
    pub def_resgid: u16,
    // 以下字段仅适用于ext2 rev 1或更高版本
    pub first_ino: u32,
    pub inode_size: u16,
    pub block_group_nr: u16,
    pub feature_compat: u32,
    pub feature_incompat: u32,
    pub feature_ro_compat: u32,
    pub uuid: [u8; 16],
    pub volume_name: [u8; 16],
    pub last_mounted: [u8; 64],
    pub algo_bitmap: u32,
    // 以下字段仅适用于ext3
    pub prealloc_blocks: u8,
    pub prealloc_dir_blocks: u8,
    pub reserved_gdt_blocks: u16,
    pub journal_uuid: [u8; 16],
    pub journal_inum: u32,
    pub journal_dev: u32,
    pub last_orphan: u32,
    pub hash_seed: [u32; 4],
    pub def_hash_version: u8,
    pub journal_backup_type: u8,
	pub desc_size: u16,
    pub default_mount_options: u32,
    pub first_meta_bg: u32,
    pub mkfs_time: u32,
    pub journal_blocks: [u32; 17],
    // 以下字段仅适用于ext4
    pub blocks_count_hi: u32,
    pub r_blocks_count_hi: u32,
    pub free_blocks_count_hi: u32,
    pub min_extra_isize: u16,
    pub want_extra_isize: u16,
    pub flags: u32,
    pub raid_stride: u16,
    pub mmp_interval: u16,
    pub mmp_block: u64,
    pub raid_stripe_width: u32,
    pub log_groups_per_flex: u8,
    pub checksum_type: u8,
    pub reserved_pad: u16,
    pub kbytes_written: u64,
    pub snapshot_inum: u32,
    pub snapshot_id: u32,
    pub snapshot_r_blocks_count: u64,
    pub snapshot_list: u32,
    pub error_count: u32,
    pub first_error_time: u32,
    pub first_error_ino: u32,
    pub first_error_block: u64,
    pub first_error_func: [u8; 32],
    pub first_error_line: u32,
    pub last_error_time: u32,
    pub last_error_ino: u32,
    pub last_error_line: u32,
    pub last_error_block: u64,
    pub last_error_func: [u8; 32],
    pub mount_opts: [u8; 64],
    pub usr_quota_inum: u32,
    pub grp_quota_inum: u32,
    pub overhead_blocks: u32,
    pub backup_bgs: [u32; 2],
    pub encrypt_algos: [u8; 4],
    pub encrypt_pw_salt: [u8; 16],
    pub lpf_ino: u32,
    pub prj_quota_inum: u32,
    pub checksum_seed: u32,
    pub padding2: [u8; 100],
    pub checksum: u32,
}

// 定义inode结构体，参考 https://www.nongnu.org/ext2-doc/ext2.html#INODES
#[repr(C)]
#[derive(Debug)]
pub struct Ext4Inode {
    pub mode: u16,
    pub uid: u16,
    pub size: u32,
    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub dtime: u32,
    pub gid: u16,
    pub links_count: u16,
    pub blocks: u32,
    pub flags: u32,
    pub osd1: u32,
    pub block: [u32; 15],
    pub generation: u32,
    pub file_acl: u32,
    pub dir_acl: u32,
    pub faddr: u32,
    pub osd2: [u8; 12],
}


/**@brief   Mount point descriptor.*/
pub struct Ext4MountPoint {
    /**@brief   Mount done flag.*/
    pub mounted: bool,
    /**@brief   Mount point name (@ref ext4_mount)*/
    pub mount_name: [char; 33],

    pub mount_name_string: String,
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct GroupDesc {
    /// Lower 32-bits of location of block bitmap.
    pub bg_block_bitmap_lo: u32,

    /// Lower 32-bits of location of inode bitmap.
    pub bg_inode_bitmap_lo: u32,

    /// Lower 32-bits of location of inode table.
    pub bg_inode_table_lo: u32,

    /// Lower 16-bits of free block count.
    pub bg_free_blocks_count_lo: u16,

    /// Lower 16-bits of free inode count.
    pub bg_free_inodes_count_lo: u16,

    /// Lower 16-bits of directory count.
    pub bg_used_dirs_count_lo: u16,

    /// Block group flags
    pub bg_flags: GroupFlags,

    /// Lower 32-bits of location of snapshot exclusion bitmap.
    pub bg_exclude_bitmap_lo: u32,

    /// Lower 16-bits of the block bitmap checksum.
    pub bg_block_bitmap_csum_lo: u16,

    /// Lower 16-bits of the inode bitmap checksum.
    pub bg_inode_bitmap_csum_lo: u16,

    /// Lower 16-bits of unused inode count.
    /// If set, we needn’t scan past the (sb.s_inodes_per_group - gdt.bg_itable_unused) th
    /// entry in the inode table for this group.
    pub bg_itable_unused_lo: u16,

    /// Group descriptor checksum;
    /// crc16(sb_uuid+group_num+bg_desc) if the RO_COMPAT_GDT_CSUM feature is set,
    /// or crc32c(sb_uuid+group_num+bg_desc) & 0xFFFF if the RO_COMPAT_METADATA_CSUM feature is set.
    /// The bg_checksum field in bg_desc is skipped when calculating crc16 checksum,
    /// and set to zero if crc32c checksum is used.
    pub bg_checksum: u16,

    /// Upper 32-bits of location of block bitmap.
    pub bg_block_bitmap_hi: u32,

    /// Upper 32-bits of location of inodes bitmap.
    pub bg_inode_bitmap_hi: u32,

    /// Upper 32-bits of location of inodes table.
    pub bg_inode_table_hi: u32,

    /// Upper 16-bits of free block count.
    pub bg_free_blocks_count_hi: u16,

    /// Upper 16-bits of free inode count.
    pub bg_free_inodes_count_hi: u16,

    /// Upper 16-bits of directory count.
    pub bg_used_dirs_count_hi: u16,

    /// Upper 16-bits of unused inode count.
    pub bg_itable_unused_hi: u16,

    /// Upper 32-bits of location of snapshot exclusion bitmap.
    pub bg_exclude_bitmap_hi: u32,

    /// Upper 16-bits of the block bitmap checksum.
    pub bg_block_bitmap_csum_hi: u16,

    /// Upper 16-bits of the inode bitmap checksum.
    pub bg_inode_bitmap_csum_hi: u16,

    /// Padding to 64 bytes.
    pub bg_reserved: u32,
}

// 定义目录项结构体，参考 https://www.nongnu.org/ext2-doc/ext2.html#DIRECTORY-ENTRIES
#[repr(C)]
#[derive(Debug)]
pub struct Ext4DirEntry {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    pub name: [u8; 255],
}


#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentHeader {
    /// Magic number, 0xF30A.
    pub eh_magic: u16,

    /// Number of valid entries following the header.
    pub eh_entries: u16,

    /// Maximum number of entries that could follow the header.
    pub eh_max: u16,

    /// Depth of this extent node in the extent tree.
    /// 0 = this extent node points to data blocks;
    /// otherwise, this extent node points to other extent nodes.
    /// The extent tree can be at most 5 levels deep:
    /// a logical block number can be at most 2^32,
    /// and the smallest n that satisfies 4*(((blocksize - 12)/12)^n) >= 2^32 is 5.
    pub eh_depth: u16,

    /// Generation of the tree. (Used by Lustre, but not standard ext4).
    pub eh_generation: u32,
}


#[derive(Debug, Default)]
#[repr(C)]
pub struct Ext4ExtentIndex {
    /// This index node covers file blocks from ‘block’ onward.
    pub ei_block: u32,

    /// Lower 32-bits of the block number of the extent node that is
    /// the next level lower in the tree. The tree node pointed to
    /// can be either another internal node or a leaf node, described below.
    pub ei_leaf_lo: u32,

    /// Upper 16-bits of the previous field.
    pub ei_leaf_hi: u16,

    pub ei_unused: u16,
}




#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4Extent {
    /// First file block number that this extent covers.
    pub ee_block: u32,

    /// Number of blocks covered by extent.
    /// If the value of this field is <= 32768, the extent is initialized.
    /// If the value of the field is > 32768, the extent is uninitialized
    /// and the actual extent length is ee_len - 32768.
    /// Therefore, the maximum length of a initialized extent is 32768 blocks,
    /// and the maximum length of an uninitialized extent is 32767.
    pub ee_len: u16,

    /// Upper 16-bits of the block number to which this extent points.
    pub ee_start_hi: u16,

    /// Lower 32-bits of the block number to which this extent points.
    pub ee_start_lo: u32,
}


#[derive(Default, Debug)]
// A single block descriptor
pub struct Ext4Block {
    // Logical block ID
    pub lb_id: u64,
    // Buffer
    // buf: Ext4Buf,
    // Data buffer
    pub data: Vec<u8>,
}

/**
 * Linked list directory entry structure
 */
pub struct Ext4DirEn {
    pub inode: u32,     // I-node for the entry
    pub entry_len: u16, // Distance to the next directory entry
    pub name_len: u8,   // Lower 8 bits of name length
    pub name_length_high: u8, // Internal fields
    pub name: [u8; 255],      // Entry name
}


/// 文件描述符
pub struct Ext4File {
    /// 挂载点句柄
    pub mp: Ext4MountPoint,
    /// 文件 inode id
    pub inode: u32,
    /// 打开标志
    pub flags: u32,
    /// 文件大小
    pub fsize: u64,
    /// 实际文件位置
    pub fpos: u64,
    
    pub inode_mode: u16,

    pub blocks: u32,

    pub read_index: usize,

    pub all_blocks: Vec<u64>,
}



pub  struct Ext4DirSearchResult {
    // block: Ext4Block,
    pub dentry: Ext4DirEn,
}


#[derive(Debug)]
pub struct Ext4ExtentPath {
    // Physical block number
    pub p_block: ext4_fsblk_t,
    // Single block descriptor
    pub block: Ext4Block,
    // Depth of this extent node
    pub depth: u16,
    // Max depth of the extent tree
    pub maxdepth: i32,
    // Pointer to the extent header
    pub header: *const Ext4ExtentHeader,
    // Pointer to the index in the current node
    pub index: Ext4ExtentIndex,
    // Pointer to the extent in the current node
    pub extent: *const Ext4Extent,
}



bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct GroupFlags: u16 {
        /// inode table and bitmap are not initialized
        const INODE_UNINIT = 0x1;
        /// block bitmap is not initialized
        const BLOCK_UNINIT = 0x2;
        /// inode table is zeroed
        const INODE_ZEROED = 0x4;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct FileMode: u16 {
        // S_IFMT是文件类型的掩码
        const S_IFMT   = 0xF000; // 文件类型的位域
        // S_IFSOCK是套接字类型
        const S_IFSOCK = 0xC000; // 套接字
        // S_IFLNK是符号链接类型
        const S_IFLNK  = 0xA000; // 符号链接
        // S_IFREG是普通文件类型
        const S_IFREG  = 0x8000; // 普通文件
        // S_IFBLK是块设备类型
        const S_IFBLK  = 0x6000; // 块设备
        // S_IFDIR是目录类型
        const S_IFDIR  = 0x4000; // 目录
        // S_IFCHR是字符设备类型
        const S_IFCHR  = 0x2000; // 字符设备
        // S_IFIFO是FIFO类型
        const S_IFIFO  = 0x1000; // FIFO
        // S_ISUID是设置用户ID位
        const S_ISUID  = 0x0800; // 设置用户ID位
        // S_ISGID是设置组ID位
        const S_ISGID  = 0x0400; // 设置组ID位
        // S_ISVTX是粘滞位
        const S_ISVTX  = 0x0200; // 粘滞位
        // S_IRWXU是用户权限的掩码
        const S_IRWXU  = 0x01E0; // 用户权限的位域
        // S_IRUSR是用户可读权限
        const S_IRUSR  = 0x0100; // 用户可读
        // S_IWUSR是用户可写权限
        const S_IWUSR  = 0x0080; // 用户可写
        // S_IXUSR是用户可执行权限
        const S_IXUSR  = 0x0040; // 用户可执行
        // S_IRWXG是组权限的掩码
        const S_IRWXG  = 0x001C; // 组权限的位域
        // S_IRGRP是组可读权限
        const S_IRGRP  = 0x0010; // 组可读
        // S_IWGRP是组可写权限
        const S_IWGRP  = 0x0008; // 组可写
        // S_IXGRP是组可执行权限
        const S_IXGRP  = 0x0004; // 组可执行
        // S_IRWXO是其他用户权限的掩码
        const S_IRWXO  = 0x0007; // 其他用户权限的位域
        // S_IROTH是其他用户可读权限
        const S_IROTH  = 0x0004; // 其他用户可读
        // S_IWOTH是其他用户可写权限
        const S_IWOTH  = 0x0002; // 其他用户可写
        // S_IXOTH是其他用户可执行权限
        const S_IXOTH  = 0x0001; // 其他用户可执行
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct IFlags: u32 {
        // 定义每个flag的名字和值，使用16进制字面量
        // EXT4_EA_INODE_FL是扩展属性的inode标志
        const EXT4_EA_INODE_FL = 0x0020_0000; // 扩展属性的inode
        // EXT4_HUGE_FILE_FL是大文件标志
        const EXT4_HUGE_FILE_FL = 0x0040_0000; // 大文件
        // EXT4_EXTENTS_FL是使用extents的标志
        const EXT4_EXTENTS_FL = 0x0080_0000; // 使用extents
        // EXT4_EOFBLOCKS_FL是有额外的块的标志
        const EXT4_EOFBLOCKS_FL = 0x0200_0000; // 有额外的块
        // EXT4_SNAPFILE_FL是快照文件的标志
        const EXT4_SNAPFILE_FL = 0x0400_0000; // 快照文件
        // EXT4_SNAPFILE_DELETED_FL是快照文件已删除的标志
        const EXT4_SNAPFILE_DELETED_FL = 0x0800_0000; // 快照文件已删除
        // EXT4_SNAPFILE_SHRUNK_FL是快照文件已缩小的标志
        const EXT4_SNAPFILE_SHRUNK_FL = 0x1000_0000; // 快照文件已缩小
        // EXT4_INLINE_DATA_FL是内联数据的标志
        const EXT4_INLINE_DATA_FL = 0x2000_0000; // 内联数据
        // EXT4_PROJINHERIT_FL是继承项目的标志
        const EXT4_PROJINHERIT_FL = 0x4000_0000; // 继承项目
        // EXT4_RESERVED_FL是保留的标志
        const EXT4_RESERVED_FL = 0x8000_0000; // 保留
    }
}


bitflags! {
    #[derive(PartialEq, Debug)]
    struct InodeMode: u16 {
        const S_IFSOCK = 0xC000;
        const S_IFLNK = 0xA000;
        const S_IFREG = 0x8000;
        const S_IFBLK = 0x6000;
        const S_IFDIR = 0x4000;
        const S_IFCHR = 0x2000;
        const S_IFIFO = 0x1000;
    }
}

bitflags! {
    #[derive(PartialEq, Eq, Debug)]
    pub struct DirEntryType: u8 {
        const UNKNOWN = 0;
        const REG_FILE = 1;
        const DIR = 2;
        const CHRDEV = 3;
        const BLKDEV = 4;
        const FIFO = 5;
        const SOCK = 6;
        const SYMLINK = 7;
    }
}



pub const BASE_OFFSET: u64 = 1024; // 超级块的偏移量
pub const BLOCK_SIZE: u64 = 4096; // 块大小
pub const INODE_SIZE: u64 = 128; // inode大小
pub const ROOT_INODE: u64 = 2; // 根目录的inode号
pub type ext4_lblk_t = u32;
pub type ext4_fsblk_t = u64;



impl Default for Ext4SuperBlock{
    fn default() -> Self {
        Self{
            ..Default::default()
        }
    }
}

impl Ext4ExtentHeader {
    pub fn from_bytes_u32(bytes: &[u32]) -> Ext4ExtentHeader {
        let size = size_of::<Self>();
        let src = bytes.as_ptr() as *const Self;
        let mut dst = Self {
            eh_magic: 0,
            eh_entries: 0,
            eh_max: 0,
            eh_depth: 0,
            eh_generation: 0,
        };
        let ptr = &mut dst as *mut Ext4ExtentHeader as *mut Ext4ExtentHeader;
        unsafe { core::ptr::copy_nonoverlapping(src, ptr, 1) };
        dst
    }

}

impl Ext4DirEntry{
    pub fn from_bytes_offset(bytes: &[u8], offset: usize) -> Ext4DirEntry {
        let new_bytes = &bytes[offset..];
        let size = size_of::<Self>();
        let src = new_bytes.as_ptr() as *const Self;
        let mut dst = Self {
            inode: 0,
            rec_len: 0,
            name_len: 0,
            file_type: 0,
            name: [0; 255],
        };
        let ptr = &mut dst as *mut Ext4DirEntry as *mut Ext4DirEntry;
        unsafe { core::ptr::copy_nonoverlapping(src, ptr, 1) };
        dst
    }
}

impl Ext4ExtentIndex{
    pub fn from_bytes_u32(bytes: &[u32]) -> Ext4ExtentIndex {
        let size = size_of::<Self>();
        let src = bytes.as_ptr() as *const Self;
        let mut dst = Self {
            ei_block: 0,
            ei_leaf_lo: 0,
            ei_leaf_hi: 0,
            ei_unused: 0,
        };
        let ptr = &mut dst as *mut Ext4ExtentIndex as *mut Ext4ExtentIndex;
        unsafe { core::ptr::copy_nonoverlapping(src, ptr, 1) };
        dst
    }
}

impl Ext4Extent{
    pub fn from_bytes_u32(bytes: &[u32]) -> Ext4Extent {
        let size = size_of::<Self>();
        let src = bytes.as_ptr() as *const Self;
        let mut dst = Self {
            ee_block: 0,
            ee_len: 0,
            ee_start_hi: 0,
            ee_start_lo: 0,
        };
        let ptr = &mut dst as *mut Ext4Extent as *mut Ext4Extent;
        unsafe { core::ptr::copy_nonoverlapping(src, ptr, 1) };
        dst
    }
}

impl Default for Ext4ExtentPath {
    fn default() -> Self {
        Self {
            p_block: 0,
            block: Ext4Block::default(),
            depth: 0,
            maxdepth: 0,
            header: core::ptr::null_mut(),
            index: Ext4ExtentIndex::default(),
            extent: core::ptr::null_mut(),
        }
    }
}


impl Ext4MountPoint {
    pub fn new(name: &str) -> Self {
        let name_string = name.to_string();
        let mut arr: [char; 33] = ['0'; 33];
        for (i, c) in name.chars().enumerate() {
            if i >= arr.len() {
                break;
            }
            arr[i] = c;
        }
        Ext4MountPoint {
            mounted: true,
            mount_name: arr,
            mount_name_string: name_string,
        }
    }
}

impl Default for Ext4DirEn {
    fn default() -> Self {
        Self {
            inode: 0,     // I-node for the entry
            entry_len: 0, // Distance to the next directory entry
            name_len: 0,  // Lower 8 bits of name length
            name_length_high: 0,
            name: [0u8; 255],
        }
    }


}
impl Default for Ext4DirSearchResult {
    fn default() -> Self {
        Self {
            dentry: Ext4DirEn::default(),
        }
    }
}


impl Ext4File{
    pub fn new(mp: Ext4MountPoint) -> Self{

        Self{
            mp: mp,
            inode: 0,
            flags: 0,
            fsize: 0,
            fpos: 0,
            inode_mode:0,
            blocks:0,
            read_index: 0,
            all_blocks: Vec::new(),
        }
    }
}
pub trait Ext4Traits {
    fn read_block(&self, offset: u64) ->Vec<u8>;
}