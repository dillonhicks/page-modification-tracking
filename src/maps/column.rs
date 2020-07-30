//! Types representing the columns of the `/proc/pid/maps` file:
//!
//! Each type is presented in left to right order in which it appears in the file:
//!
//! * [`maps::columns::AddressRange`]
//! * [`maps::columns::PermSet`]
//! * [`maps::columns::Perm`]
//! * [`maps::columns::Offset`]
//! * [`maps::columns::Device`]
//! * [`maps::columns::Inode`]
//! * [`maps::columns::PathName`]
//!
//! **Example /proc/pid/maps file contents**
//! ```text
//! 55c723b94000-55c723c98000 r-xp 00000000 fd:01 49545223                   /bin/bash
//! 55c72450b000-55c72499f000 rw-p 00000000 00:00 0                          [heap]
//! 7fcb2de8c000-7fcb2e691000 r--s 00000000 fd:01 50213063                   /var/lib/sss/mc/passwd
//! 7fcb2e691000-7fcb2e699000 r-xp 00000000 fd:01 10223639                   /lib/x86_64-linux-gnu/libnss_sss.so.2
//! 7fcb3045e000-7fcb30465000 r--s 00000000 fd:01 32511450                   /usr/lib/x86_64-linux-gnu/gconv/gconv-modules.cache
//! 7fcb30465000-7fcb30466000 r--p 00027000 fd:01 10224447                   /lib/x86_64-linux-gnu/ld-2.27.so
//! 7fcb30466000-7fcb30467000 rw-p 00028000 fd:01 10224447                   /lib/x86_64-linux-gnu/ld-2.27.so
//! 7fcb30467000-7fcb30468000 rw-p 00000000 00:00 0
//! 7ffe1fa2a000-7ffe1fa4b000 rw-p 00000000 00:00 0                          [stack]
//! 7ffe1fbec000-7ffe1fbef000 r--p 00000000 00:00 0                          [vvar]
//! 7ffe1fbef000-7ffe1fbf0000 r-xp 00000000 00:00 0                          [vdso]
//! ffffffffff600000-ffffffffff601000 --xp 00000000 00:00 0                  [vsyscall]
//! ```
use std::{
    convert::TryFrom,
    fmt,
    iter::IntoIterator,
    string::ToString,
};

use crate::{
    deps::{
        derive_more,
        serde,
    },
    error::Error,
};

/// ```text
/// +-----------------------+----- AddressRange
/// V                       V
/// 7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
/// ```
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AddressRange {
    start: usize,
    end:   usize,
}

impl AddressRange {
    const SEPARATOR: char = '-';

    pub const fn new(
        start: usize,
        end: usize,
    ) -> Self {
        AddressRange { start, end }
    }

    pub fn contains(
        &self,
        n: usize,
    ) -> bool {
        (n >= self.start) && (n < self.end)
    }

    pub const fn start(&self) -> usize {
        self.start
    }

    pub const fn end(&self) -> usize {
        self.end
    }

    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    pub const fn offset_from(
        &self,
        low_addr: usize,
    ) -> usize {
        self.start - low_addr
    }
}


impl fmt::Debug for AddressRange {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        f.debug_struct("AddressRange")
            .field("start", &crate::fmt::Hex(&self.start))
            .field("end", &crate::fmt::Hex(&self.end))
            .field("size", &self.len())
            .finish()
    }
}


impl<'a> TryFrom<&'a str> for AddressRange {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<AddressRange>(),
                reason:   "blank string".to_string(),
            });
        } else if trimmed.len() < 3 {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<AddressRange>(),
                reason:   "address range string was shorter than the minimum number of characters (3)".to_string(),
            });
        }

        let parts = trimmed
            .splitn(2, AddressRange::SEPARATOR)
            .map(|s| usize::from_str_radix(s, 16))
            .collect::<Vec<_>>();

        if parts.len() != 2 {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<AddressRange>(),
                reason:   format!(
                    "address range string was not in the form XX{}YY, parts={:?}",
                    AddressRange::SEPARATOR,
                    parts
                ),
            });
        } else if parts.iter().any(Result::is_err) {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<AddressRange>(),
                reason:   format!("part of address range string was not a number {:?}", parts),
            });
        }

        let mut parts_iter = parts.into_iter().map(Result::<usize, _>::unwrap);

        let low = parts_iter.next().unwrap();
        let high = parts_iter.next().unwrap();

        Ok(AddressRange {
            start: low,
            end:   high,
        })
    }
}

impl fmt::Display for AddressRange {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{:0>8x}{}{:0>8x}", self.start, AddressRange::SEPARATOR, self.end)
    }
}

/// ```text
///                           +--+-- PermSet
///                           V  V
/// 7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct PermSet(Vec<Perm>);

impl<'a> TryFrom<&'a str> for PermSet {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<PermSet>(),
                reason:   "blank string".to_string(),
            });
        }

        let mut set = Vec::with_capacity(4);
        for ch in trimmed.chars() {
            set.push(Perm::try_from(ch)?);
        }

        Ok(PermSet(set.into_iter().collect()))
    }
}

impl fmt::Display for PermSet {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        for perm in self.0.iter() {
            perm.fmt(f)?;
        }
        Ok(())
    }
}


/// ```text
///                           +----- Perm::Read
///                           V
/// 7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Perm {
    Read,
    Write,
    Execute,
    Private,
    Shared,
    Nil,
}


impl Perm {
    const ALL: [Perm; 6] = [
        Perm::Read,
        Perm::Write,
        Perm::Execute,
        Perm::Private,
        Perm::Shared,
        Perm::Nil,
    ];

    fn chars() -> &'static [char] {
        use crate::deps::lazy_static::lazy_static;
        lazy_static! {
            static ref PERM_CHARS: Vec<char> = Perm::ALL.into_iter().map(Perm::to_char).collect::<_>();
        }

        (&*PERM_CHARS).as_slice()
    }

    pub fn to_char(&self) -> char {
        use Perm::*;

        match self {
            Read => 'r',
            Write => 'w',
            Execute => 'x',
            Private => 'p',
            Shared => 's',
            Nil => '-',
        }
    }

}


impl TryFrom<char> for Perm {
    type Error = Error;

    fn try_from(ch: char) -> Result<Self, Self::Error> {
        use Perm::*;

        let perm = match ch.to_ascii_lowercase() {
            'r' => Read,
            'w' => Write,
            'x' => Execute,
            's' => Shared,
            'p' => Private,
            '-' => Nil,
            unknown_ch => {
                return Err(Error::Parse {
                    value:    unknown_ch.to_string(),
                    typename: std::any::type_name::<Perm>(),
                    reason:   format!(
                        "character was not one of \"{:?}\"",
                        Perm::chars()
                    ),
                });
            }
        };

        Ok(perm)
    }
}


impl<'a> TryFrom<&'a str> for Perm {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Perm>(),
                reason:   "blank string".to_string(),
            });
        } else if trimmed.len() != 1 {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Perm>(),
                reason:   "string was longer than one character".to_string(),
            });
        }

        let ch = trimmed.chars().next().unwrap();
        TryFrom::try_from(ch)
    }
}


impl fmt::Display for Perm {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        use Perm::*;
        self.to_char().fmt(f)
    }
}


/// ```text
///                                 +----- Offset
///                                 V
/// 7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
/// ```
#[derive(
    Copy,
    Clone,
    Debug,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Hash,
    derive_more::From,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Offset(usize);

impl<'a> TryFrom<&'a str> for Offset {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Offset>(),
                reason:   "blank string".to_string(),
            });
        }

        Ok(Offset(usize::from_str_radix(trimmed, 16).map_err(|_err| {
            Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Offset>(),
                reason:   "Offset string was not valid base 16 usize".to_string(),
            }
        })?))
    }
}


impl fmt::Display for Offset {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{:0>8x}", self.0)
    }
}

/// ```text
///                                           +----- Device
///                                           V
/// 7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Device {
    major: usize,
    minor: usize,
}

impl Device {
    const SEPARATOR: char = ':';
}


impl<'a> TryFrom<&'a str> for Device {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Device>(),
                reason:   "blank string".to_string(),
            });
        } else if trimmed.len() < 3 {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Device>(),
                reason:   "device string was shorter than the minimum number of characters (3)".to_string(),
            });
        }

        let parts = trimmed
            .splitn(2, Device::SEPARATOR)
            .map(|s| usize::from_str_radix(s, 16))
            .collect::<Vec<_>>();

        if parts.len() != 2 {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Device>(),
                reason:   format!("device string was not in the form XX{}YY", Device::SEPARATOR),
            });
        } else if parts.iter().any(Result::is_err) {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Device>(),
                reason:   format!("part of device string was not a number {:?}", parts),
            });
        }

        let mut parts_iter = parts.into_iter().map(Result::<usize, _>::unwrap);

        let major = parts_iter.next().unwrap();
        let minor = parts_iter.next().unwrap();

        Ok(Device { major, minor })
    }
}

impl fmt::Display for Device {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        write!(f, "{:0>2x}{}{:0>2x}", self.major, Device::SEPARATOR, self.minor)
    }
}


/// ```text
///                                                  +----- Inode
///                                                  V
/// 7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
/// ```
#[derive(
    Copy,
    Clone,
    Debug,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Hash,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Inode(usize);


impl<'a> TryFrom<&'a str> for Inode {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Inode>(),
                reason:   "blank string".to_string(),
            });
        }

        Ok(Inode(trimmed.parse::<usize>().map_err(|_err| {
            Error::Parse {
                value:    value.to_string(),
                typename: std::any::type_name::<Inode>(),
                reason:   "Inode string was not valid base 10 usize".to_string(),
            }
        })?))
    }
}


/// ```text
///                                                                             +----- PathName::Real(..)
///                                                                             V
/// 7fa281f3f000-7fa281f42000 r-xp 00000000 103:01 270269                    /usr/lib64/zsh/5.5.1/zsh/stat.so
///
///                                                                             +----- PathName::Pseudo(..)
///                                                                             V
/// 7ffce82d7000-7ffce831f000 rw-p 00000000 00:00 0                          [stack]
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum PathName {
    Empty,
    Pseudo(String),
    Real(String),
}


impl PathName {
    pub fn as_str(&self) -> &str {
        use PathName::*;
        match self {
            Empty => "",
            Real(s) | Pseudo(s) => s.as_str(),
        }
    }
}


impl<'a> TryFrom<&'a str> for PathName {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        let path = if trimmed.is_empty() {
            PathName::Empty
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            PathName::Pseudo(trimmed.to_string())
        } else {
            PathName::Real(trimmed.to_string())
        };

        Ok(path)
    }
}

impl fmt::Display for PathName {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
