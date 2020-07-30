//! ```text
//! pagemap, from the userspace perspective
//! ---------------------------------------
//!
//! pagemap is a new (as of 2.6.25) set of interfaces in the kernel that allow
//! userspace programs to examine the page tables and related information by
//! reading files in /proc.
//!
//! There are four components to pagemap:
//!
//!  * /proc/pid/pagemap.  This file lets a userspace process find out which
//!    physical frame each virtual page is mapped to.  It contains one 64-bit
//!    value for each virtual page, containing the following data (from
//!    fs/proc/task_mmu.c, above pagemap_read):
//!
//!     * Bits 0-54  page frame number (PFN) if present
//!     * Bits 0-4   swap type if swapped
//!     * Bits 5-54  swap offset if swapped
//!     * Bit  55    pte is soft-dirty (see Documentation/vm/soft-dirty.txt)
//!     * Bit  56    page exclusively mapped (since 4.2)
//!     * Bits 57-60 zero
//!     * Bit  61    page is file-page or shared-anon (since 3.5)
//!     * Bit  62    page swapped
//!     * Bit  63    page present
//!
//!    Since Linux 4.0 only users with the CAP_SYS_ADMIN capability can get PFNs.
//!    In 4.0 and 4.1 opens by unprivileged fail with -EPERM.  Starting from
//!    4.2 the PFN field is zeroed if the user does not have CAP_SYS_ADMIN.
//!    Reason: information about PFNs helps in exploiting Rowhammer vulnerability.
//!
//!    If the page is not present but in swap, then the PFN contains an
//!    encoding of the swap file number and the page's offset into the
//!    swap. Unmapped pages return a null PFN. This allows determining
//!    precisely which pages are mapped (or in swap) and comparing mapped
//!    pages between processes.
//!
//!    Efficient users of this interface will use /proc/pid/maps to
//!    determine which areas of memory are actually mapped and llseek to
//!    skip over unmapped regions.
//!
//! Using pagemap to do something useful:
//!
//! The general procedure for using pagemap to find out about a process' memory
//! usage goes like this:
//!
//!  1. Read /proc/pid/maps to determine which parts of the memory space are
//!     mapped to what.
//!  2. Select the maps you are interested in -- all of them, or a particular
//!     library, or the stack or the heap, etc.
//!  3. Open /proc/pid/pagemap and seek to the pages you would like to examine.
//!  4. Read a u64 for each page from pagemap.
//!  5. Open /proc/kpagecount and/or /proc/kpageflags.  For each PFN you just
//!     read, seek to that entry in the file, and read the data you want.
//!
//! For example, to find the "unique set size" (USS), which is the amount of
//! memory that a process is using that is not shared with any other process,
//! you can go through every map in the process, find the PFNs, look those up
//! in kpagecount, and tally up the number of pages that are only referenced
//! once.
//!
//! Other notes:
//!
//! Reading from any of the files will return -EINVAL if you are not starting
//! the read on an 8-byte boundary (e.g., if you sought an odd number of bytes
//! into the file), or if the size of the read is not a multiple of 8 bytes.
//!
//! Before Linux 3.11 pagemap bits 55-60 were used for "page-shift" (which is
//! always 12 at most architectures). Since Linux 3.11 their meaning changes
//! after first clear of soft-dirty bits. Since Linux 4.2 they are used for
//! flags unconditionally.
//! ```

use std::{
    convert::TryFrom,
    fmt,
    fs::File,
    io::{
        BufRead,
        BufReader,
        Read,
        Seek,
        SeekFrom,
        Write,
    },
    mem,
    num::NonZeroU64,
    path::PathBuf,
};

use crate::{
    deps::log::{
        debug,
        info,
        warn,
    },
    error::Error,
    kpageflags::KPageFlags,
    maps::{
        column::{
            AddressRange,
            PathName,
            PermSet,
        },
        MappedRegion,
        Maps,
    },
};
use std::str::FromStr;


#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(usize)]
pub enum PageSize {
    Normal = 4 << 10,
    Huge = 2 << 20,
    Giga = 1 << 30,
}


impl FromStr for PageSize {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "normal" => Ok(PageSize::Normal),
            "huge" => Ok(PageSize::Huge),
            "giga" => Ok(PageSize::Giga),
            bad_value => {
                Err(Error::Parse {
                    value:    value.to_string(),
                    typename: std::any::type_name::<PageSize>(),
                    reason:   "value was not one of: normal, huge, giga".to_string(),
                })
            }
        }
    }
}


impl std::default::Default for PageSize {
    fn default() -> Self {
        Self::Normal
    }
}


#[derive(
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::Binary,
    derive_more::LowerHex,
    derive_more::UpperHex,
    serde::Serialize,
    serde::Deserialize,
)]
#[repr(transparent)]
pub struct PageTableEntry(u64);


impl PageTableEntry {
    const PFN_BITS: u32 = 55;
    const PRESENT_BIT: u32 = 63;
    const SOFT_DIRTY_BIT: u32 = 55;

    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    /// Note:
    /// ```text
    ///    Since Linux 4.0 only users with the CAP_SYS_ADMIN capability can get PFNs.
    ///    In 4.0 and 4.1 opens by unprivileged fail with -EPERM.  Starting from
    ///    4.2 the PFN field is zeroed if the user does not have CAP_SYS_ADMIN.
    ///    Reason: information about PFNs helps in exploiting Rowhammer vulnerability.
    /// ```
    pub fn page_frame_number(&self) -> Option<std::num::NonZeroU64> {
        const MASK: u64 = u64::max_value().wrapping_shr(u64::max_value().count_ones() - PageTableEntry::PFN_BITS);
        std::num::NonZeroU64::new(self.0 & MASK)
    }

    pub const fn is_soft_dirty(&self) -> bool {
        const MASK: u64 = 1 << PageTableEntry::SOFT_DIRTY_BIT;
        self.0 & MASK != 0
    }

    pub const fn is_present(&self) -> bool {
        const MASK: u64 = 1 << PageTableEntry::PRESENT_BIT;
        self.0 & MASK != 0
    }
}

impl<'a> TryFrom<&'a mut dyn Read> for PageTableEntry {
    type Error = Error;

    fn try_from(rdr: &'a mut dyn Read) -> Result<Self, Self::Error> {
        crate::io::read_u64(rdr).map(PageTableEntry::new)
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        f.debug_struct("PageTableEntry")
            .field("value", &crate::fmt::Binary(&self.0))
            .field("page_frame_number", &self.page_frame_number())
            .field("soft_dirty", &self.is_soft_dirty())
            .field("present", &self.is_present())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ProcessVMA {
    pid:  usize,
    path: Option<PathBuf>,
    maps: Maps,
}


impl ProcessVMA {
    pub fn this_process() -> Result<Self, Error> {
        let pid = usize::try_from(std::process::id())?;
        Self::with_pid(pid)
    }

    pub fn with_pid(pid: usize) -> Result<Self, Error> {
        debug!("loading ProcessVMA with pid: {}", pid);
        let path = crate::paths::proc_pid_maps_path(Some(pid));
        let maps = crate::io::new_buffered_file_reader(&path, None)
            .map_err(Error::from)
            .and_then(|mut rdr| Maps::try_from(&mut rdr as &mut dyn BufRead))?;

        Ok(Self {
            pid,
            path: Some(path),
            maps,
        })
    }

    pub const fn pid(&self) -> usize {
        self.pid
    }

    pub const fn maps(&self) -> &Maps {
        &self.maps
    }

    pub fn region(
        &self,
        addr: usize,
    ) -> Option<VMARegion<'_>> {
        self.maps.region(addr).map(|region| VMARegion { pid: self.pid, region })
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        *self = Self::with_pid(self.pid)?;
        Ok(())
    }

    /// reset the soft-dirty bits for process with PID
    pub fn clear_refs(&self) -> Result<(), Error> {
        const CLEAR_CMD: &'static str = "4\n";
        debug!("clearing soft-dirty PTE for pid={}", self.pid);

        let path = crate::paths::proc_pid_clear_refs(Some(self.pid));
        debug!("opening file: {:?}", path);
        let mut file = std::fs::OpenOptions::new()
            .read(false)
            .write(true)
            .create(false)
            .append(false)
            .open(path)?;

        file.write_all(CLEAR_CMD.as_bytes())?;

        Ok(())
    }
}


#[derive(Copy, Clone, Debug, serde::Serialize)]
pub struct PageDescriptor<'a> {
    pub addr_range: AddressRange,
    pub offset:     usize,
    pub perms:      &'a PermSet,
    pub pathame:    &'a PathName,
    pub pte:        PageTableEntry,
    pub kpageflags: Option<KPageFlags>,
    pub kpagecount: Option<NonZeroU64>,
}



macro_rules! warn_once {
        ($name:ident; $($arg:tt)+) => {{
            use $crate::deps::lazy_static::lazy_static;
            use $crate::deps::log::warn;

            lazy_static! {
                static ref $name: ::std::sync::Once = ::std::sync::Once::new();
            }

            (&*($name)).call_once(|| {
                warn!("[WARN_ONCE] {}", format_args!($($arg)*))
            })
       }};
}


#[derive(Debug)]
pub struct VMARegion<'a> {
    pid:    usize,
    region: &'a MappedRegion,
}

impl<'a> VMARegion<'a> {
    pub const LEVEL_SIZE: usize = 512;
    pub const PAGESIZE: usize = PageSize::Normal as usize;

    pub fn try_iter(
        &self,
        page_size_override: Option<PageSize>,
    ) -> Result<Iter, Error> {
        let pagemaps_reader = self.open_pagemaps()?;
        let kpageflags_reader = self.open_kpageflags()?;
        let kpagecount_reader = self.open_kpagecount()?;

        info!("created iterator for mapped region: {} ", self.region.addr_range());

        Ok(Iter {
            addr_range: *(self.region.addr_range()),
            page_count: 0,
            current_addr: self.region.addr_range().start(),
            page_size_override,
            pagemaps_reader,
            kpageflags_reader,
            kpagecount_reader,
            region: self.region,
        })
    }

    fn open_pagemaps(&self) -> Result<BufReader<File>, Error> {
        let path = crate::paths::proc_pid_pagemaps_path(Some(self.pid));
        let offset_bytes = (self.region.addr_range().start() / VMARegion::PAGESIZE) * mem::size_of::<PageTableEntry>();
        let offset_btyes = u64::try_from(offset_bytes).map(NonZeroU64::new)?;
        Ok(crate::io::new_buffered_file_reader(&path, offset_btyes)?)
    }

    fn open_kpageflags(&self) -> Result<Option<BufReader<File>>, Error> {
        let kpageflags_path = crate::paths::proc_kpageflags_path();
        let open_kpageflags_result = crate::io::new_buffered_file_reader(kpageflags_path, None);
        match open_kpageflags_result {
            Ok(reader) => Ok(Some(reader)),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                warn_once!(PROC_KPAGEFLAGS_PERMISSION_DENIED;
                    "some functionality disabled, unable to read {:?}, reason: {:?}",
                    kpageflags_path,
                    err
                );
                Ok(None)
            }
            Err(err) => Err(err)?,
        }
    }

    fn open_kpagecount(&self) -> Result<Option<BufReader<File>>, Error> {
        let kpagecount_path = crate::paths::proc_kpagecount_path();
        match crate::io::new_buffered_file_reader(kpagecount_path, None) {
            Ok(reader) => Ok(Some(reader)),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                warn_once!(PROC_KPAGECOUNT_PERMISSION_DENIED;
                    "some functionality disabled, unable to read {:?}, reason: {:?}",
                    kpagecount_path,
                    err
                );
                Ok(None)
            }
            Err(err) => Err(err)?,
        }
    }
}


pub struct Iter<'a> {
    addr_range:         AddressRange,
    page_count:         usize,
    current_addr:       usize,
    page_size_override: Option<PageSize>,
    pagemaps_reader:    BufReader<File>,
    kpageflags_reader:  Option<BufReader<File>>,
    kpagecount_reader:  Option<BufReader<File>>,
    region:             &'a MappedRegion,
}

impl<'a> Iter<'a> {
    fn kpageflags_for_pte(
        &mut self,
        pte: &PageTableEntry,
    ) -> Result<Option<KPageFlags>, Error> {
        const KPAGEFLAGS_SIZE: u64 = mem::size_of::<KPageFlags>() as u64;

        // to read the kpageflags, the reader needs to have permissions to read
        // the PFN bits of the PTE to locate the entry in kpageflags
        match (pte.page_frame_number(), self.kpageflags_reader.as_mut()) {
            (Some(pfn), Some(mut reader)) => {
                let offset = pfn.get() * KPAGEFLAGS_SIZE;
                reader.seek(SeekFrom::Start(offset))?;
                let reader: &mut dyn Read = reader;
                Ok(Some(KPageFlags::try_from(reader)?))
            }
            // occurs when functionality is disabled due to permissions
            _ => Ok(None),
        }
    }

    fn kpagecount_for_pte(
        &mut self,
        pte: &PageTableEntry,
    ) -> Result<Option<NonZeroU64>, Error> {
        const KPAGECOUNT_SIZE: u64 = mem::size_of::<u64>() as u64;

        match (pte.page_frame_number(), self.kpagecount_reader.as_mut()) {
            (Some(pfn), Some(mut reader)) => {
                let offset = pfn.get() * KPAGECOUNT_SIZE;
                reader.seek(SeekFrom::Start(offset))?;

                let reader: &mut dyn Read = reader;
                crate::io::read_u64(reader).map(NonZeroU64::new)
            }
            // occurs when functionality is disabled due to permissions
            _ => Ok(None),
        }
    }

    fn next_page_descriptor(&mut self) -> Result<Option<PageDescriptor<'a>>, Error> {
        if !self.addr_range.contains(self.current_addr) {
            return Ok(None);
        }

        let low = self.current_addr;
        let rdr: &mut dyn Read = &mut self.pagemaps_reader;

        let pte = match PageTableEntry::try_from(rdr) {
            Ok(ok) => ok,
            Err(err) => {
                warn!("{:?}", err);
                return Ok(None);
            }
        };

        let kpageflags = self.kpageflags_for_pte(&pte)?;
        let kpagecount = self.kpagecount_for_pte(&pte)?;
        let is_hugepage = kpageflags.as_ref().map(KPageFlags::huge).unwrap_or(false);

        let page_size = if let Some(size) = self.page_size_override {
            size as usize
        } else if is_hugepage {
            VMARegion::PAGESIZE * VMARegion::LEVEL_SIZE
        } else {
            VMARegion::PAGESIZE
        };


        self.current_addr = (self.current_addr).checked_add(page_size).unwrap_or_else(|| {
            panic!(
                "bad math: {} + {} would overflow type {}",
                low,
                page_size,
                std::any::type_name::<usize>(),
            )
        });

        Ok(Some(PageDescriptor {
            addr_range: AddressRange::new(low, self.current_addr),
            offset: 0,
            perms: self.region.perms(),
            pathame: self.region.pathname(),
            pte,
            kpageflags,
            kpagecount,
        }))
    }
}


impl<'a> Iterator for Iter<'a> {
    type Item = Result<PageDescriptor<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_page_descriptor().transpose()
    }
}

