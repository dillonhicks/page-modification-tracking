//! Types for `/proc/[pid]/maps`.
//!
//! ```text
//!  /proc/[pid]/maps
//!               A file containing the currently mapped memory regions and
//!               their access permissions.  See mmap(2) for some further infor‐
//!               mation about memory mappings.
//!
//!               Permission to access this file is governed by a ptrace access
//!               mode PTRACE_MODE_READ_FSCREDS check; see ptrace(2).
//!
//!               The format of the file is:
//!
//!     address           perms offset  dev   inode       pathname
//!     00400000-00452000 r-xp 00000000 08:02 173521      /usr/bin/dbus-daemon
//!     00651000-00652000 r--p 00051000 08:02 173521      /usr/bin/dbus-daemon
//!     00652000-00655000 rw-p 00052000 08:02 173521      /usr/bin/dbus-daemon
//!     00e03000-00e24000 rw-p 00000000 00:00 0           [heap]
//!     00e24000-011f7000 rw-p 00000000 00:00 0           [heap]
//!     ...
//!     35b1800000-35b1820000 r-xp 00000000 08:02 135522  /usr/lib64/ld-2.15.so
//!     35b1a1f000-35b1a20000 r--p 0001f000 08:02 135522  /usr/lib64/ld-2.15.so
//!     35b1a20000-35b1a21000 rw-p 00020000 08:02 135522  /usr/lib64/ld-2.15.so
//!     35b1a21000-35b1a22000 rw-p 00000000 00:00 0
//!     35b1c00000-35b1dac000 r-xp 00000000 08:02 135870  /usr/lib64/libc-2.15.so
//!     35b1dac000-35b1fac000 ---p 001ac000 08:02 135870  /usr/lib64/libc-2.15.so
//!     35b1fac000-35b1fb0000 r--p 001ac000 08:02 135870  /usr/lib64/libc-2.15.so
//!     35b1fb0000-35b1fb2000 rw-p 001b0000 08:02 135870  /usr/lib64/libc-2.15.so
//!     ...
//!     f2c6ff8c000-7f2c7078c000 rw-p 00000000 00:00 0    [stack:986]
//!     ...
//!     7fffb2c0d000-7fffb2c2e000 rw-p 00000000 00:00 0   [stack]
//!     7fffb2d48000-7fffb2d49000 r-xp 00000000 00:00 0   [vdso]
//!
//!               The address field is the address space in the process that the
//!               mapping occupies.  The perms field is a set of permissions:
//!
//!                   r = read
//!                   w = write
//!                   x = execute
//!                   s = shared
//!                   p = private (copy on write)
//!
//!               The offset field is the offset into the file/whatever; dev is
//!               the device (major:minor); inode is the inode on that device.
//!               0 indicates that no inode is associated with the memory
//!               region, as would be the case with BSS (uninitialized data).
//!
//!               The pathname field will usually be the file that is backing
//!               the mapping.  For ELF files, you can easily coordinate with
//!               the offset field by looking at the Offset field in the ELF
//!               program headers (readelf -l).
//!
//!               There are additional helpful pseudo-paths:
//!
//!               [stack]
//!                      The initial process's (also known as the main thread's)
//!                      stack.
//!
//!               [stack:<tid>] (from Linux 3.4 to 4.4)
//!                      A thread's stack (where the <tid> is a thread ID).  It
//!                      corresponds to the /proc/[pid]/task/[tid]/ path.  This
//!                      field was removed in Linux 4.5, since providing this
//!                      information for a process with large numbers of threads
//!                      is expensive.
//!
//!               [vdso] The virtual dynamically linked shared object.  See
//!                      vdso(7).
//!
//!               [heap] The process's heap.
//!
//!               If the pathname field is blank, this is an anonymous mapping
//!               as obtained via mmap(2).  There is no easy way to coordinate
//!               this back to a process's source, short of running it through
//!               gdb(1), strace(1), or similar.
//!
//!               pathname is shown unescaped except for newline characters,
//!               which are replaced with an octal escape sequence.  As a
//!               result, it is not possible to determine whether the original
//!               pathname contained a newline character or the literal \e012
//!               character sequence.
//!
//!               If the mapping is file-backed and the file has been deleted,
//!               the string " (deleted)" is appended to the pathname.  Note
//!               that this is ambiguous too.
//!
//!               Under Linux 2.0, there is no field giving pathname.
//! ```
pub mod column;

use self::column::{
    AddressRange,
    Device,
    Inode,
    Offset,
    PathName,
    PermSet,
};
use crate::{
    deps::{serde, log::warn},
    error::Error,
};
use std::{
    convert::TryFrom,
    fmt,
    io::BufRead,
    path::Path,
};

const PATHNAME_DISPLAY_RIGHT_PADDING: usize = 73;


/// ```text
///                                   +-- MappedRegion
///                                   |
/// +---------------------------------+---------------------------------------------------------------------+
/// V                                                                                                       V
/// 7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MappedRegion {
    addr_range: AddressRange,
    perms:      PermSet,
    offset:     Offset,
    device:     Device,
    inode:      Inode,
    pathname:   PathName,
    extra:      Vec<String>,
}


impl MappedRegion {
    pub const fn addr_range(&self) -> &AddressRange {
        &self.addr_range
    }

    pub const fn perms(&self) -> &PermSet {
        &self.perms
    }

    pub const fn offset(&self) -> Offset {
        self.offset
    }

    pub const fn device(&self) -> &Device {
        &self.device
    }

    pub const fn inode(&self) -> Inode {
        self.inode
    }

    pub const fn pathname(&self) -> &PathName {
        &self.pathname
    }

    pub fn extra(&self) -> &[String] {
        self.extra.as_slice()
    }
}


impl fmt::Display for MappedRegion {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let s = format!(
            "{} {} {} {} {}",
            self.addr_range, self.perms, self.offset, self.device, self.inode
        );
        s.fmt(f)?;
        match &self.pathname {
            PathName::Empty => Ok(()),
            _path => {
                let pad = PATHNAME_DISPLAY_RIGHT_PADDING.checked_sub(s.len()).unwrap_or(0);
                let pad_ws = unsafe { String::from_utf8_unchecked(vec![b' '; pad]) };
                pad_ws.fmt(f)?;
                self.pathname.fmt(f)
            }
        }
    }
}


impl<'a> TryFrom<&'a str> for MappedRegion {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<MappedRegion>(),
                reason:   "blank string".to_string(),
            });
        }

        let mut iter = trimmed.split_ascii_whitespace();

        let addr_range = AddressRange::try_from(iter.next().unwrap_or(""))?;
        let perms = PermSet::try_from(iter.next().unwrap_or(""))?;
        let offset = Offset::try_from(iter.next().unwrap_or(""))?;
        let device = Device::try_from(iter.next().unwrap_or(""))?;
        let inode = Inode::try_from(iter.next().unwrap_or(""))?;
        let pathname = PathName::try_from(iter.next().unwrap_or(""))?;
        // extra garbage we couldn't parse
        let extra = iter.map(str::to_string).collect::<Vec<_>>();

        if !extra.is_empty() {
            warn!(
                "unexpected extra fields were encountered while parsing this line - line={:?}; extra={:?}",
                value, extra
            );
        }

        Ok(MappedRegion {
            addr_range,
            perms,
            offset,
            device,
            inode,
            pathname,
            extra,
        })
    }
}


/// This is the whole file
#[derive(Debug, Clone, PartialEq)]
pub struct Maps {
    /// Index of start address to the MappedRegion entry. The BTreeMap keeps the
    /// the collection ordered by address, like the original /proc/pid/maps file.
    map:            std::collections::BTreeMap<usize, MappedRegion>,
    /// Reverse index on PathName to find all of the mapped regions matching a
    /// file.
    pathname_index: std::collections::HashMap<PathName, Vec<AddressRange>>,
}


impl Maps {
    fn new() -> Self {
        Self {
            map:            std::collections::BTreeMap::new(),
            pathname_index: std::collections::HashMap::new(),
        }
    }

    fn insert(
        &mut self,
        entry: MappedRegion,
    ) {
        let addr_range = entry.addr_range;
        let pathname = entry.pathname.clone();
        self.map.insert(addr_range.start(), entry);
        self.pathname_index.entry(pathname).or_default().push(addr_range);
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, usize, MappedRegion> {
        self.map.iter()
    }

    pub fn primary_index(&self) -> &std::collections::BTreeMap<usize, MappedRegion> {
        &self.map
    }

    /// Get the reference to a mapped region corresponding to the given address,
    /// if it exists.
    pub fn region(
        &self,
        address: usize,
    ) -> Option<&MappedRegion> {
        match self.map.get(&address) {
            Some(m) => Some(m),
            None => self.map.values().find(|m| m.addr_range().contains(address)),
        }
    }

    /// Get the slice of mapped regions corresponding to the given pathname,
    /// if any exist.
    pub fn addrs_for_pathname<P>(
        &self,
        path: P,
    ) -> Option<&[AddressRange]>
    where
        PathName: TryFrom<P>,
    {
        PathName::try_from(path).ok()
            .and_then(|p| self.pathname_index.get(&p))
            .map(|addrs| addrs.as_slice())
    }
}


impl<'a> TryFrom<&'a str> for Maps {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut pagemap = Maps::new();

        for line in value.lines() {
            let entry = MappedRegion::try_from(line)?;
            pagemap.insert(entry);
        }

        Ok(pagemap)
    }
}

impl<'a> TryFrom<&'a mut dyn BufRead> for Maps {
    type Error = Error;

    fn try_from(reader: &'a mut dyn BufRead) -> Result<Self, Self::Error> {
        let mut pagemap = Maps::new();

        for line in reader.lines().map(|r| r.unwrap()) {
            let entry = MappedRegion::try_from(line.as_str())?;
            pagemap.insert(entry);
        }

        Ok(pagemap)
    }
}


impl<'a> TryFrom<&'a Path> for Maps {
    type Error = Error;

    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        let mut reader = crate::io::new_buffered_file_reader(path, None)?;
        Maps::try_from(&mut reader as &mut dyn BufRead)
    }
}


impl fmt::Display for Maps {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        for value in self.map.values() {
            writeln!(f, "{}", value)?;
        }
        Ok(())
    }
}


#[test]
fn test_parse() {
    const EXAMPLE_PROC_MAPS: &'static str = r#"00400000-004c0000 r-xp 00000000 103:01 270237                            /usr/bin/zsh
006bf000-006c0000 r--p 000bf000 103:01 270237                            /usr/bin/zsh
006c0000-006c7000 rw-p 000c0000 103:01 270237                            /usr/bin/zsh
006c7000-006da000 rw-p 00000000 00:00 0
00e08000-01135000 rw-p 00000000 00:00 0                                  [heap]
7fa281d2e000-7fa281d3e000 r-xp 00000000 103:01 270247                    /usr/lib64/zsh/5.5.1/zsh/computil.so
7fa281d3e000-7fa281f3d000 ---p 00010000 103:01 270247                    /usr/lib64/zsh/5.5.1/zsh/computil.so
7fa281f3d000-7fa281f3e000 r--p 0000f000 103:01 270247                    /usr/lib64/zsh/5.5.1/zsh/computil.so
7fa281f3e000-7fa281f3f000 rw-p 00010000 103:01 270247                    /usr/lib64/zsh/5.5.1/zsh/computil.so
7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
7fa281f42000-7fa282141000 ---p 00003000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
7fa282141000-7fa282142000 r--p 00002000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
7fa282142000-7fa282143000 rw-p 00003000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
7fa282143000-7fa282145000 r-xp 00000000 103:01 270272                    /usr/lib64/zsh/5.5.1/zsh/terminfo.so
7fa282145000-7fa282344000 ---p 00002000 103:01 270272                    /usr/lib64/zsh/5.5.1/zsh/terminfo.so
7fa282344000-7fa282345000 r--p 00001000 103:01 270272                    /usr/lib64/zsh/5.5.1/zsh/terminfo.so
7fa282345000-7fa282346000 rw-p 00002000 103:01 270272                    /usr/lib64/zsh/5.5.1/zsh/terminfo.so
7fa282346000-7fa282348000 r-xp 00000000 103:01 270255                    /usr/lib64/zsh/5.5.1/zsh/langinfo.so
7fa282348000-7fa282547000 ---p 00002000 103:01 270255                    /usr/lib64/zsh/5.5.1/zsh/langinfo.so
7fa282547000-7fa282548000 r--p 00001000 103:01 270255                    /usr/lib64/zsh/5.5.1/zsh/langinfo.so
7fa282548000-7fa282549000 rw-p 00002000 103:01 270255                    /usr/lib64/zsh/5.5.1/zsh/langinfo.so
7fa282549000-7fa282557000 r-xp 00000000 103:01 270246                    /usr/lib64/zsh/5.5.1/zsh/complist.so
7fa282557000-7fa282757000 ---p 0000e000 103:01 270246                    /usr/lib64/zsh/5.5.1/zsh/complist.so
7fa282757000-7fa282758000 r--p 0000e000 103:01 270246                    /usr/lib64/zsh/5.5.1/zsh/complist.so
7fa282758000-7fa282759000 rw-p 0000f000 103:01 270246                    /usr/lib64/zsh/5.5.1/zsh/complist.so
7fa282759000-7fa282761000 r-xp 00000000 103:01 270279                    /usr/lib64/zsh/5.5.1/zsh/zutil.so
7fa282761000-7fa282960000 ---p 00008000 103:01 270279                    /usr/lib64/zsh/5.5.1/zsh/zutil.so
7fa282960000-7fa282961000 r--p 00007000 103:01 270279                    /usr/lib64/zsh/5.5.1/zsh/zutil.so
7fa282961000-7fa282962000 rw-p 00008000 103:01 270279                    /usr/lib64/zsh/5.5.1/zsh/zutil.so
7fa282962000-7fa282985000 r-xp 00000000 103:01 270245                    /usr/lib64/zsh/5.5.1/zsh/complete.so
7fa282985000-7fa282b85000 ---p 00023000 103:01 270245                    /usr/lib64/zsh/5.5.1/zsh/complete.so
7fa282b85000-7fa282b86000 r--p 00023000 103:01 270245                    /usr/lib64/zsh/5.5.1/zsh/complete.so
7fa282b86000-7fa282b87000 rw-p 00024000 103:01 270245                    /usr/lib64/zsh/5.5.1/zsh/complete.so
7fa282b87000-7fa282b88000 rw-p 00000000 00:00 0
7fa282b88000-7fa282b92000 r-xp 00000000 103:01 270264                    /usr/lib64/zsh/5.5.1/zsh/parameter.so
7fa282b92000-7fa282d91000 ---p 0000a000 103:01 270264                    /usr/lib64/zsh/5.5.1/zsh/parameter.so
7fa282d91000-7fa282d92000 r--p 00009000 103:01 270264                    /usr/lib64/zsh/5.5.1/zsh/parameter.so
7fa282d92000-7fa282d93000 rw-p 0000a000 103:01 270264                    /usr/lib64/zsh/5.5.1/zsh/parameter.so
7fa282d93000-7fa282ddb000 r-xp 00000000 103:01 270274                    /usr/lib64/zsh/5.5.1/zsh/zle.so
7fa282ddb000-7fa282fda000 ---p 00048000 103:01 270274                    /usr/lib64/zsh/5.5.1/zsh/zle.so
7fa282fda000-7fa282fdc000 r--p 00047000 103:01 270274                    /usr/lib64/zsh/5.5.1/zsh/zle.so
7fa282fdc000-7fa282fe4000 rw-p 00049000 103:01 270274                    /usr/lib64/zsh/5.5.1/zsh/zle.so
7fa282fe4000-7fa289bb3000 r--p 00000000 103:01 276804                    /usr/lib/locale/locale-archive
7fa289bb3000-7fa289bcb000 r-xp 00000000 103:01 282043                    /usr/lib64/libpthread-2.26.so
7fa289bcb000-7fa289dcb000 ---p 00018000 103:01 282043                    /usr/lib64/libpthread-2.26.so
7fa289dcb000-7fa289dcc000 r--p 00018000 103:01 282043                    /usr/lib64/libpthread-2.26.so
7fa289dcc000-7fa289dcd000 rw-p 00019000 103:01 282043                    /usr/lib64/libpthread-2.26.so
7fa289dcd000-7fa289dd1000 rw-p 00000000 00:00 0
7fa289dd1000-7fa289f72000 r-xp 00000000 103:01 264810                    /usr/lib64/libc-2.26.so
7fa289f72000-7fa28a172000 ---p 001a1000 103:01 264810                    /usr/lib64/libc-2.26.so
7fa28a172000-7fa28a176000 r--p 001a1000 103:01 264810                    /usr/lib64/libc-2.26.so
7fa28a176000-7fa28a178000 rw-p 001a5000 103:01 264810                    /usr/lib64/libc-2.26.so
7fa28a178000-7fa28a17c000 rw-p 00000000 00:00 0
7fa28a17c000-7fa28a2bb000 r-xp 00000000 103:01 264817                    /usr/lib64/libm-2.26.so
7fa28a2bb000-7fa28a4ba000 ---p 0013f000 103:01 264817                    /usr/lib64/libm-2.26.so
7fa28a4ba000-7fa28a4bb000 r--p 0013e000 103:01 264817                    /usr/lib64/libm-2.26.so
7fa28a4bb000-7fa28a4bc000 rw-p 0013f000 103:01 264817                    /usr/lib64/libm-2.26.so
7fa28a4bc000-7fa28a4c3000 r-xp 00000000 103:01 289012                    /usr/lib64/librt-2.26.so
7fa28a4c3000-7fa28a6c2000 ---p 00007000 103:01 289012                    /usr/lib64/librt-2.26.so
7fa28a6c2000-7fa28a6c3000 r--p 00006000 103:01 289012                    /usr/lib64/librt-2.26.so
7fa28a6c3000-7fa28a6c4000 rw-p 00007000 103:01 289012                    /usr/lib64/librt-2.26.so
7fa28a6c4000-7fa28a6eb000 r-xp 00000000 103:01 265142                    /usr/lib64/libtinfo.so.6.0
7fa28a6eb000-7fa28a8ea000 ---p 00027000 103:01 265142                    /usr/lib64/libtinfo.so.6.0
7fa28a8ea000-7fa28a8ee000 r--p 00026000 103:01 265142                    /usr/lib64/libtinfo.so.6.0
7fa28a8ee000-7fa28a8ef000 rw-p 0002a000 103:01 265142                    /usr/lib64/libtinfo.so.6.0
7fa28a8ef000-7fa28a924000 r-xp 00000000 103:01 265134                    /usr/lib64/libncursesw.so.6.0
7fa28a924000-7fa28ab24000 ---p 00035000 103:01 265134                    /usr/lib64/libncursesw.so.6.0
7fa28ab24000-7fa28ab25000 r--p 00035000 103:01 265134                    /usr/lib64/libncursesw.so.6.0
7fa28ab25000-7fa28ab26000 rw-p 00036000 103:01 265134                    /usr/lib64/libncursesw.so.6.0
7fa28ab26000-7fa28ab29000 r-xp 00000000 103:01 264815                    /usr/lib64/libdl-2.26.so
7fa28ab29000-7fa28ad28000 ---p 00003000 103:01 264815                    /usr/lib64/libdl-2.26.so
7fa28ad28000-7fa28ad29000 r--p 00002000 103:01 264815                    /usr/lib64/libdl-2.26.so
7fa28ad29000-7fa28ad2a000 rw-p 00003000 103:01 264815                    /usr/lib64/libdl-2.26.so
7fa28ad2a000-7fa28ad8d000 r-xp 00000000 103:01 265311                    /usr/lib64/libpcre.so.1.2.0
7fa28ad8d000-7fa28af8c000 ---p 00063000 103:01 265311                    /usr/lib64/libpcre.so.1.2.0
7fa28af8c000-7fa28af8d000 r--p 00062000 103:01 265311                    /usr/lib64/libpcre.so.1.2.0
7fa28af8d000-7fa28af8e000 rw-p 00063000 103:01 265311                    /usr/lib64/libpcre.so.1.2.0
7fa28af8e000-7fa28af9a000 r-xp 00000000 103:01 266388                    /usr/lib64/libgdbm.so.4.0.0
7fa28af9a000-7fa28b199000 ---p 0000c000 103:01 266388                    /usr/lib64/libgdbm.so.4.0.0
7fa28b199000-7fa28b19a000 r--p 0000b000 103:01 266388                    /usr/lib64/libgdbm.so.4.0.0
7fa28b19a000-7fa28b19b000 rw-p 0000c000 103:01 266388                    /usr/lib64/libgdbm.so.4.0.0
7fa28b19b000-7fa28b1bf000 r-xp 00000000 103:01 264698                    /usr/lib64/ld-2.26.so
7fa28b36e000-7fa28b3a3000 r--s 00000000 103:01 132098                    /var/db/nscd/passwd
7fa28b3a3000-7fa28b3a9000 rw-p 00000000 00:00 0
7fa28b3af000-7fa28b3b6000 r--s 00000000 103:01 265116                    /usr/lib64/gconv/gconv-modules.cache
7fa28b3ba000-7fa28b3be000 rw-p 00000000 00:00 0
7fa28b3be000-7fa28b3bf000 r--p 00023000 103:01 264698                    /usr/lib64/ld-2.26.so
7fa28b3bf000-7fa28b3c0000 rw-p 00024000 103:01 264698                    /usr/lib64/ld-2.26.so
7fa28b3c0000-7fa28b3c1000 rw-p 00000000 00:00 0
7ffce82d7000-7ffce831f000 rw-p 00000000 00:00 0                          [stack]
7ffce83c1000-7ffce83c4000 r--p 00000000 00:00 0                          [vvar]
7ffce83c4000-7ffce83c6000 r-xp 00000000 00:00 0                          [vdso]
ffffffffff600000-ffffffffff601000 r-xp 00000000 00:00 0                  [vsyscall]
"#;

    let pagemap = Maps::try_from(EXAMPLE_PROC_MAPS).unwrap();

    eprintln!("{:#?}", pagemap);
    println!("{}", pagemap);
    assert_eq!(pagemap.map.len(), EXAMPLE_PROC_MAPS.lines().count());
    assert_eq!(&format!("{}", pagemap), EXAMPLE_PROC_MAPS);
}
