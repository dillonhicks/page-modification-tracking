use crate::deps::{
    log::{
        debug,
        error,
        info,
        warn,
    },
    nix::sys::mman::{
        mmap,
        munmap,
        MapFlags,
        ProtFlags,
    },
};
use std::{
    borrow::Cow,
    fs::{
        File,
        OpenOptions,
    },
    mem::ManuallyDrop,
    os::unix::io::AsRawFd,
    path::{
        Path,
        PathBuf,
    },
    pin::Pin,
    ptr::NonNull,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};


#[derive(Debug, Clone)]
pub struct MmapOptions<'a> {
    pub path:           Cow<'a, Path>,
    pub base_addr:      *mut std::ffi::c_void,
    pub len:            crate::deps::libc::size_t,
    pub addr_offset:    crate::deps::libc::off_t,
    pub remove_on_drop: bool,
}

impl<'a> MmapOptions<'a> {
    fn owned<'b>(opts: &MmapOptions<'b>) -> MmapOptions<'static> {
        MmapOptions {
            path:           PathBuf::from(opts.path.clone()).into(),
            base_addr:      opts.base_addr,
            len:            opts.len,
            addr_offset:    opts.addr_offset,
            remove_on_drop: opts.remove_on_drop,
        }
    }
}

unsafe impl Send for MmapOptions<'static> {}
unsafe impl Sync for MmapOptions<'static> {}

struct MmapFileInner {
    pub path:      PathBuf,
    pub file:      File,
    // do not drop this vec as it is created from the
    // span of the mmap
    pub buf:       ManuallyDrop<Pin<Vec<u8>>>,
    pub opts:      MmapOptions<'static>,
    pub flags:     MapFlags,
    pub prot:      ProtFlags,
    pub is_mapped: AtomicBool,
}

impl MmapFileInner {
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    unsafe fn unmap_memory(&self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            munmap(self.buf.as_ptr() as *mut u8 as *mut _, self.len()).map_err(|e| {
                error!(
                    "[{}] an unhandled error occurred during the call to munmap({:?}, {}) \
                     unmap memory mapped file at: {:?}  {:?}",
                    MmapFile::TAG,
                    self.buf.as_ptr(),
                    self.len(),
                    self.path,
                    e
                );
                e
            })?;
        }



        self.is_mapped.store(false, Ordering::SeqCst);
        Ok(())
    }
}

impl Drop for MmapFileInner {
    fn drop(&mut self) {
        debug!("[{}::drop] unmapping file {:?}", MmapFile::TAG, self.path);

        unsafe {
            if self.is_mapped.load(Ordering::SeqCst) {
                self.unmap_memory()
                    .unwrap_or_else(|err| warn!("[{}] unable to drop MmapFile due to error {}", MmapFile::TAG, err));
            } else {
                debug!("[{}] mapped file was already unmapped", MmapFile::TAG);
            }
        }

        if self.opts.remove_on_drop {
            std::fs::remove_file(&self.path)
                .unwrap_or_else(|err| warn!("[{}] could not delete mmap file {:?}", MmapFile::TAG, self.path));
        }
    }
}


#[derive(Clone)]
pub struct MmapFile {
    inner: std::sync::Arc<MmapFileInner>,
}


impl MmapFile {
    pub const TAG: Cow<'static, str> = Cow::Borrowed("MmapFile");

    pub fn new<'a>(
        opts: &MmapOptions<'a>,
        prot: ProtFlags,
        flags: MapFlags,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        debug!(
            "[{}] creating new file backed memory mapping with options: {:?}; flags: {:?}; permissions: {:?}",
            Self::TAG,
            opts,
            flags,
            prot
        );

        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(opts.path.as_ref())?;

        file.set_len(opts.len as u64)?;
        file.sync_all()?;

        let file_ptr: *mut std::ffi::c_void = unsafe {
            mmap(
                opts.base_addr,
                opts.len,
                prot,
                flags,
                file.as_raw_fd(),
                opts.addr_offset,
            )?
        };

        if !(MapFlags::MAP_FIXED & flags).is_empty() {
            assert_eq!(
                file_ptr, opts.base_addr,
                "call to mmap did not create a MAP_FIXED mapping \
                 at the requested base address"
            );
        }

        let buf = unsafe { ManuallyDrop::new(Pin::new(Vec::from_raw_parts(file_ptr as *mut u8, opts.len, opts.len))) };

        Ok(Self {
            inner: std::sync::Arc::new(MmapFileInner {
                path: PathBuf::from(opts.path.as_ref()),
                file,
                buf,
                opts: MmapOptions::owned(opts),
                flags,
                prot,
                is_mapped: AtomicBool::new(true),
            }),
        })
    }

    pub fn fixed_with_options<'a>(opts: &MmapOptions<'a>) -> Result<Self, Box<dyn std::error::Error>> {
        let flags = MapFlags::MAP_SHARED | MapFlags::MAP_FIXED | MapFlags::MAP_NORESERVE;
        let prot = ProtFlags::PROT_READ | ProtFlags::PROT_WRITE;
        Self::new(opts, prot, flags)
    }

    pub fn with_options<'a>(opts: &MmapOptions<'a>) -> Result<Self, Box<dyn std::error::Error>> {
        let flags = MapFlags::MAP_SHARED | MapFlags::MAP_NORESERVE;
        let prot = ProtFlags::PROT_READ | ProtFlags::PROT_WRITE;
        Self::new(opts, prot, flags)
    }

    pub fn len(&self) -> usize {
        self.inner.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.buf.is_empty()
    }

    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    #[inline(always)]
    pub fn as_nonnull(&self) -> NonNull<u8> {
        let array = self.as_ref();
        unsafe { NonNull::new_unchecked((&array[0]) as *const u8 as *mut u8) }
    }
}


impl std::fmt::Debug for MmapFile {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct(Self::TAG.as_ref())
            .field("path", &self.inner.path)
            .field("ptr", &self.inner.buf.as_ptr())
            .field("len", &self.len())
            .finish()
    }
}

impl std::convert::AsRef<[u8]> for MmapFile {
    fn as_ref(&self) -> &[u8] {
        &self.inner.buf
    }
}

impl std::convert::AsMut<[u8]> for MmapFile {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.inner.buf.as_ptr() as *mut u8, self.len()) }
    }
}
