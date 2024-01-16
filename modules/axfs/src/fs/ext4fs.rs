use crate::dev::Disk;
use alloc::sync::Arc;
use core::cell::{RefCell, UnsafeCell};
use core::num;
use core::ptr::NonNull;

use axfs_vfs::{VfsDirEntry, VfsError, VfsNodePerm, VfsResult};
use axfs_vfs::{VfsNodeAttr, VfsNodeOps, VfsNodeRef, VfsNodeType, VfsOps};
use axsync::Mutex;

use ext4fs::{BlockDevice, Ext4Fs, BLOCK_SIZE, *};

unsafe impl Send for Ext4FileSystem {}
unsafe impl Sync for Ext4FileSystem {}
unsafe impl Send for Ext4DirWrapper {}
unsafe impl Sync for Ext4DirWrapper {}
unsafe impl Send for Ext4FileWrapper {}
unsafe impl Sync for Ext4FileWrapper {}

pub struct DiskAdapter {
    inner: RefCell<Disk>,
}

impl BlockDevice for DiskAdapter {
    fn block_num(&self) -> usize {
        self.inner.borrow_mut().size() as usize / BLOCK_SIZE as usize
    }
    fn block_size(&self) -> usize {
        BLOCK_SIZE as usize
    }
    fn read_block(&self, offset: usize, buf: &mut [u8]) {
        assert_eq!(buf.len(), self.block_size());

        let disk_block_id = offset / 512 as usize;
        let disk_offset = offset % 512 as usize;

        let mut disk = self.inner.borrow_mut();

        disk.set_position(disk_block_id as u64 * 512 as u64 + disk_offset as u64);

        let mut filled = 0;

        while filled < buf.len() {
            let mut tmp = [0u8; 512];

            disk.read_one(&mut tmp).unwrap();

            let count = buf.len().min(filled + 512 as usize) - filled;

            buf[filled..filled + count].copy_from_slice(&tmp[..count]);

            filled += count;
        }
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        assert!(buf.len() == (BLOCK_SIZE as usize));
        let mut inner = self.inner.borrow_mut();
        let true_block_size = inner.true_block_size();
        let num_block = (BLOCK_SIZE as usize) / true_block_size;

        for i in 0..num_block {
            let pos = block_id * (BLOCK_SIZE as usize) + i * true_block_size;
            inner.set_position(pos as _);
            let res = inner.write_one(&buf[i * true_block_size..(i + 1) * true_block_size]);
            assert_eq!(res.unwrap(), true_block_size);
        }
    }
}

pub struct Ext4FileSystem {
    inner: Arc<ext4fs::Ext4Fs>,
    root_dir: UnsafeCell<Option<VfsNodeRef>>,
}

impl Ext4FileSystem {
    pub fn new(disk: Disk) -> Self {
        log::info!("-----------------ext4fs init-----------------");

        let block_device = Arc::new(DiskAdapter {
            inner: RefCell::new(disk),
        });

        let inner = Arc::new(ext4fs::Ext4Fs::open(block_device));
        Self {
            inner,
            root_dir: UnsafeCell::new(None),
        }
    }

    pub fn init(&self) {
        let root_inode = self.inner.root_inode();
        let fs_ptr: NonNull<Ext4FileSystem> =
            NonNull::new(self as *const _ as *mut Ext4FileSystem).unwrap();
        unsafe { *self.root_dir.get() = Some(Self::new_dir(root_inode, fs_ptr)) }
    }

    fn new_dir(inode: ext4fs::Ext4Inode, fs_ptr: NonNull<Ext4FileSystem>) -> Arc<Ext4DirWrapper> {
        Arc::new(Ext4DirWrapper(inode, fs_ptr))
    }
}

impl VfsOps for Ext4FileSystem {
    fn root_dir(&self) -> VfsNodeRef {
        let root_dir = unsafe { (*self.root_dir.get()).as_ref().unwrap() };
        root_dir.clone()
    }

    fn umount(&self) -> VfsResult {
        todo!()
    }
}

pub struct Ext4DirWrapper(ext4fs::Ext4Inode, NonNull<Ext4FileSystem>);

impl VfsNodeOps for Ext4DirWrapper {
    fn lookup(self: Arc<Self>, path: &str) -> VfsResult<VfsNodeRef> {
        let mp = Ext4MountPoint::new("/");
        let mut ext4_file = Ext4File::new(mp);

        unsafe {
            let fs = self.1.as_ref();
            fs.inner.ext4_generic_open(&mut ext4_file, path);
            fs.inner.ext4_file_inode_read(&mut ext4_file);
            fs.inner.ext4_find_all_disk_blocks(&mut ext4_file);
        }

        let fs_ptr: NonNull<Ext4FileSystem> = self.1;

        let file_wrapepr = Arc::new(Ext4FileWrapper(Mutex::new(ext4_file), fs_ptr));

        Ok(file_wrapepr)
    }
}

pub struct Ext4FileWrapper(Mutex<ext4fs::Ext4File>, NonNull<Ext4FileSystem>);

impl VfsNodeOps for Ext4FileWrapper {
    fn lookup(self: Arc<Self>, path: &str) -> VfsResult<VfsNodeRef> {
        log::info!("-------Ext4FileWrapper-----lookup path {:?}", path);
        let mp = Ext4MountPoint::new("/");
        let mut ext4_file = Ext4File::new(mp);

        unsafe {
            let fs = self.1.as_ref();
            fs.inner.ext4_generic_open(&mut ext4_file, path);
            fs.inner.ext4_file_inode_read(&mut ext4_file);
        }

        let fs_ptr: NonNull<Ext4FileSystem> = self.1;

        let file_wrapepr = Arc::new(Ext4FileWrapper(Mutex::new(ext4_file), fs_ptr));

        Ok(file_wrapepr)
    }

    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        let ext4_file = self.0.lock();

        let inode_mode = ext4_file.inode_mode;
        let flags = ext4_file.flags;
        let size = ext4_file.fsize;
        let blocks = ext4_file.blocks;
        let (ty, perm) = map_imode(inode_mode as u16);

        drop(ext4_file);
        Ok(VfsNodeAttr::new(perm, ty, size as _, blocks as _))
    }

    fn read_dir(&self, start_idx: usize, dirents: &mut [VfsDirEntry]) -> VfsResult<usize> {
        let ext4_file = self.0.lock();

        let mut len = 0;

        unsafe {
            let fs = self.1.as_ref();

            let entries = fs
                .inner
                .read_dir_entry(ext4_file.inode as u64, &fs.inner.super_block);
            len = entries.len();

            let mut iter = entries.into_iter().skip(2).skip(start_idx);
            for (i, out_entry) in dirents.iter_mut().enumerate() {
                let x: Option<Ext4DirEntry> = iter.next();

                match x {
                    Some(ext4direntry) => {
                        let name = ext4direntry.name;
                        let name_len = ext4direntry.name_len;
                        let file_type = ext4direntry.file_type;

                        let (ty, _) = map_dir_imode(file_type as u16);
                        let name = get_name(name, name_len as usize).unwrap();

                        *out_entry = VfsDirEntry::new(name.as_str(), ty);
                    }
                    _ => return Ok(i),
                }
            }
        }

        drop(ext4_file);

        return Ok(len);
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let mut ext4_file = self.0.lock();

        unsafe {
            let fs = self.1.as_ref();
            if offset < ext4_file.fsize {
                let len: usize = fs.inner.ext4_file_iter(&mut ext4_file, buf);
                Ok(len)
            } else {
                return Ok(0);
            }
        }
    }
}

fn map_dir_imode(imode: u16) -> (VfsNodeType, VfsNodePerm) {
    let diren_type = imode;
    let type_code = ext4fs::DirEntryType::from_bits(diren_type as u8).unwrap();
    let ty = match type_code {
        DirEntryType::REG_FILE => VfsNodeType::File,
        DirEntryType::DIR => VfsNodeType::Dir,
        DirEntryType::CHRDEV => VfsNodeType::CharDevice,
        DirEntryType::BLKDEV => VfsNodeType::BlockDevice,
        DirEntryType::FIFO => VfsNodeType::Fifo,
        DirEntryType::SOCK => VfsNodeType::Socket,
        DirEntryType::SYMLINK => VfsNodeType::SymLink,
        _ => {
            log::info!("{:x?}", imode);
            VfsNodeType::File
        }
    };

    let perm = ext4fs::FileMode::from_bits_truncate(imode);
    let mut vfs_perm = VfsNodePerm::from_bits_truncate(0);

    if perm.contains(ext4fs::FileMode::S_IXOTH) {
        vfs_perm |= VfsNodePerm::OTHER_EXEC;
    }
    if perm.contains(ext4fs::FileMode::S_IWOTH) {
        vfs_perm |= VfsNodePerm::OTHER_WRITE;
    }
    if perm.contains(ext4fs::FileMode::S_IROTH) {
        vfs_perm |= VfsNodePerm::OTHER_READ;
    }

    if perm.contains(ext4fs::FileMode::S_IXGRP) {
        vfs_perm |= VfsNodePerm::GROUP_EXEC;
    }
    if perm.contains(ext4fs::FileMode::S_IWGRP) {
        vfs_perm |= VfsNodePerm::GROUP_WRITE;
    }
    if perm.contains(ext4fs::FileMode::S_IRGRP) {
        vfs_perm |= VfsNodePerm::GROUP_READ;
    }

    if perm.contains(ext4fs::FileMode::S_IXUSR) {
        vfs_perm |= VfsNodePerm::OWNER_EXEC;
    }
    if perm.contains(ext4fs::FileMode::S_IWUSR) {
        vfs_perm |= VfsNodePerm::OWNER_WRITE;
    }
    if perm.contains(ext4fs::FileMode::S_IRUSR) {
        vfs_perm |= VfsNodePerm::OWNER_READ;
    }

    (ty, vfs_perm)
}

fn map_imode(imode: u16) -> (VfsNodeType, VfsNodePerm) {
    let diren_type = imode & 0xf000;
    let type_code = ext4fs::FileMode::from_bits(diren_type).unwrap();
    let ty = match type_code {
        ext4fs::FileMode::S_IFREG => VfsNodeType::File,
        ext4fs::FileMode::S_IFDIR => VfsNodeType::Dir,
        ext4fs::FileMode::S_IFCHR => VfsNodeType::CharDevice,
        ext4fs::FileMode::S_IFBLK => VfsNodeType::BlockDevice,
        ext4fs::FileMode::S_IFIFO => VfsNodeType::Fifo,
        ext4fs::FileMode::S_IFSOCK => VfsNodeType::Socket,
        ext4fs::FileMode::S_IFLNK => VfsNodeType::SymLink,
        _ => {
            log::info!("{:x?}", imode);
            VfsNodeType::File
        }
    };

    let perm = ext4fs::FileMode::from_bits_truncate(imode);
    let mut vfs_perm = VfsNodePerm::from_bits_truncate(0);

    if perm.contains(ext4fs::FileMode::S_IXOTH) {
        vfs_perm |= VfsNodePerm::OTHER_EXEC;
    }
    if perm.contains(ext4fs::FileMode::S_IWOTH) {
        vfs_perm |= VfsNodePerm::OTHER_WRITE;
    }
    if perm.contains(ext4fs::FileMode::S_IROTH) {
        vfs_perm |= VfsNodePerm::OTHER_READ;
    }

    if perm.contains(ext4fs::FileMode::S_IXGRP) {
        vfs_perm |= VfsNodePerm::GROUP_EXEC;
    }
    if perm.contains(ext4fs::FileMode::S_IWGRP) {
        vfs_perm |= VfsNodePerm::GROUP_WRITE;
    }
    if perm.contains(ext4fs::FileMode::S_IRGRP) {
        vfs_perm |= VfsNodePerm::GROUP_READ;
    }

    if perm.contains(ext4fs::FileMode::S_IXUSR) {
        vfs_perm |= VfsNodePerm::OWNER_EXEC;
    }
    if perm.contains(ext4fs::FileMode::S_IWUSR) {
        vfs_perm |= VfsNodePerm::OWNER_WRITE;
    }
    if perm.contains(ext4fs::FileMode::S_IRUSR) {
        vfs_perm |= VfsNodePerm::OWNER_READ;
    }

    (ty, vfs_perm)
}


