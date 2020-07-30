//! ```text
//!  * /proc/kpagecount.  This file contains a 64-bit count of the number of
//!    times each page is mapped, indexed by PFN.
//!
//!  * /proc/kpageflags.  This file contains a 64-bit set of flags for each
//!    page, indexed by PFN.
//!
//!    The flags are (from fs/proc/page.c, above kpageflags_read):
//!
//!      0. LOCKED
//!      1. ERROR
//!      2. REFERENCED
//!      3. UPTODATE
//!      4. DIRTY
//!      5. LRU
//!      6. ACTIVE
//!      7. SLAB
//!      8. WRITEBACK
//!      9. RECLAIM
//!     10. BUDDY
//!     11. MMAP
//!     12. ANON
//!     13. SWAPCACHE
//!     14. SWAPBACKED
//!     15. COMPOUND_HEAD
//!     16. COMPOUND_TAIL
//!     17. HUGE
//!     18. UNEVICTABLE
//!     19. HWPOISON
//!     20. NOPAGE
//!     21. KSM
//!     22. THP
//!     23. BALLOON
//!     24. ZERO_PAGE
//!     25. IDLE
//!
//!  * /proc/kpagecgroup.  This file contains a 64-bit inode number of the
//!    memory cgroup each page is charged to, indexed by PFN. Only available when
//!    CONFIG_MEMCG is set.
//!
//! Short descriptions to the page flags:
//!
//!  0. LOCKED
//!     page is being locked for exclusive access, eg. by undergoing read/write IO
//!
//!  7. SLAB
//!     page is managed by the SLAB/SLOB/SLUB/SLQB kernel memory allocator
//!     When compound page is used, SLUB/SLQB will only set this flag on the head
//!     page; SLOB will not flag it at all.
//!
//! 10. BUDDY
//!     a free memory block managed by the buddy system allocator
//!     The buddy system organizes free memory in blocks of various orders.
//!     An order N block has 2^N physically contiguous pages, with the BUDDY flag
//!     set for and _only_ for the first page.
//!
//! 15. COMPOUND_HEAD
//! 16. COMPOUND_TAIL
//!     A compound page with order N consists of 2^N physically contiguous pages.
//!     A compound page with order 2 takes the form of "HTTT", where H donates its
//!     head page and T donates its tail page(s).  The major consumers of compound
//!     pages are hugeTLB pages (Documentation/vm/hugetlbpage.txt), the SLUB etc.
//!     memory allocators and various device drivers. However in this interface,
//!     only huge/giga pages are made visible to end users.
//! 17. HUGE
//!     this is an integral part of a HugeTLB page
//!
//! 19. HWPOISON
//!     hardware detected memory corruption on this page: don't touch the data!
//!
//! 20. NOPAGE
//!     no page frame exists at the requested address
//!
//! 21. KSM
//!     identical memory pages dynamically shared between one or more processes
//!
//! 22. THP
//!     contiguous pages which construct transparent hugepages
//!
//! 23. BALLOON
//!     balloon compaction page
//!
//! 24. ZERO_PAGE
//!     zero page for pfn_zero or huge_zero page
//!
//! 25. IDLE
//!     page has not been accessed since it was marked idle (see
//!     Documentation/vm/idle_page_tracking.txt). Note that this flag may be
//!     stale in case the page was accessed via a PTE. To make sure the flag
//!     is up-to-date one has to read /sys/kernel/mm/page_idle/bitmap first.
//!
//!     [IO related page flags]
//!  1. ERROR     IO error occurred
//!  3. UPTODATE  page has up-to-date data
//!               ie. for file backed page: (in-memory data revision >= on-disk one)
//!  4. DIRTY     page has been written to, hence contains new data
//!               ie. for file backed page: (in-memory data revision >  on-disk one)
//!  8. WRITEBACK page is being synced to disk
//!
//!     [LRU related page flags]
//!  5. LRU         page is in one of the LRU lists
//!  6. ACTIVE      page is in the active LRU list
//! 18. UNEVICTABLE page is in the unevictable (non-)LRU list
//!                 It is somehow pinned and not a candidate for LRU page reclaims,
//! 		eg. ramfs pages, shmctl(SHM_LOCK) and mlock() memory segments
//!  2. REFERENCED  page has been referenced since last LRU list enqueue/requeue
//!  9. RECLAIM     page will be reclaimed soon after its pageout IO completed
//! 11. MMAP        a memory mapped page
//! 12. ANON        a memory mapped page that is not part of a file
//! 13. SWAPCACHE   page is mapped to swap space, ie. has an associated swap entry
//! 14. SWAPBACKED  page is backed by swap/RAM
//!
//! The page-types tool in the tools/vm directory can be used to query the
//! above flags.
//! ```
use std::{
    convert::TryFrom,
    fmt,
    io::Read,
};

use crate::{
    deps::{
        derive_more,
        serde,
    },
    error::Error,
};

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
pub struct KPageFlags(u64);


impl KPageFlags {
    const ACTIVE_BIT: u32 = 6;
    const ANON_BIT: u32 = 12;
    const BALLOON_BIT: u32 = 23;
    const BUDDY_BIT: u32 = 10;
    const COMPOUND_HEAD_BIT: u32 = 15;
    const COMPOUND_TAIL_BIT: u32 = 16;
    const DIRTY_BIT: u32 = 4;
    const ERROR_BIT: u32 = 1;
    const HUGE_BIT: u32 = 17;
    const HWPOISON_BIT: u32 = 19;
    const IDLE_BIT: u32 = 25;
    const KSM_BIT: u32 = 21;
    const LOCKED_BIT: u32 = 0;
    const LRU_BIT: u32 = 5;
    const MMAP_BIT: u32 = 11;
    const NOPAGE_BIT: u32 = 20;
    const RECLAIM_BIT: u32 = 9;
    const REFERENCED_BIT: u32 = 2;
    const SLAB_BIT: u32 = 7;
    const SWAPBACKED_BIT: u32 = 14;
    const SWAPCACHE_BIT: u32 = 13;
    const THP_BIT: u32 = 22;
    const UNEVICTABLE_BIT: u32 = 18;
    const UPTODATE_BIT: u32 = 3;
    const WRITEBACK_BIT: u32 = 8;
    const ZERO_PAGE_BIT: u32 = 24;

    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    pub const fn locked(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::LOCKED_BIT;
        self.0 & MASK != 0
    }

    pub const fn error(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::ERROR_BIT;
        self.0 & MASK != 0
    }

    pub const fn referenced(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::REFERENCED_BIT;
        self.0 & MASK != 0
    }

    pub const fn uptodate(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::UPTODATE_BIT;
        self.0 & MASK != 0
    }

    pub const fn dirty(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::DIRTY_BIT;
        self.0 & MASK != 0
    }

    pub const fn lru(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::LRU_BIT;
        self.0 & MASK != 0
    }

    pub const fn active(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::ACTIVE_BIT;
        self.0 & MASK != 0
    }

    pub const fn slab(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::SLAB_BIT;
        self.0 & MASK != 0
    }

    pub const fn writeback(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::WRITEBACK_BIT;
        self.0 & MASK != 0
    }

    pub const fn reclaim(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::RECLAIM_BIT;
        self.0 & MASK != 0
    }

    pub const fn buddy(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::BUDDY_BIT;
        self.0 & MASK != 0
    }

    pub const fn mmap(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::MMAP_BIT;
        self.0 & MASK != 0
    }

    pub const fn anon(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::ANON_BIT;
        self.0 & MASK != 0
    }

    pub const fn swapcache(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::SWAPCACHE_BIT;
        self.0 & MASK != 0
    }

    pub const fn swapbacked(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::SWAPBACKED_BIT;
        self.0 & MASK != 0
    }

    pub const fn compound_head(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::COMPOUND_HEAD_BIT;
        self.0 & MASK != 0
    }

    pub const fn compound_tail(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::COMPOUND_TAIL_BIT;
        self.0 & MASK != 0
    }

    pub const fn huge(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::HUGE_BIT;
        self.0 & MASK != 0
    }

    pub const fn unevictable(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::UNEVICTABLE_BIT;
        self.0 & MASK != 0
    }

    pub const fn hwpoison(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::HWPOISON_BIT;
        self.0 & MASK != 0
    }

    pub const fn nopage(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::NOPAGE_BIT;
        self.0 & MASK != 0
    }

    pub const fn ksm(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::KSM_BIT;
        self.0 & MASK != 0
    }

    pub const fn thp(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::THP_BIT;
        self.0 & MASK != 0
    }

    pub const fn balloon(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::BALLOON_BIT;
        self.0 & MASK != 0
    }

    pub const fn zero_page(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::ZERO_PAGE_BIT;
        self.0 & MASK != 0
    }

    pub const fn idle(&self) -> bool {
        const MASK: u64 = 1u64 << KPageFlags::IDLE_BIT;
        self.0 & MASK != 0
    }
}


impl<'a> TryFrom<&'a mut dyn Read> for KPageFlags {
    type Error = Error;

    fn try_from(rdr: &'a mut dyn Read) -> Result<Self, Self::Error> {
        crate::io::read_u64(rdr).map(KPageFlags::new)
    }
}


impl fmt::Debug for KPageFlags {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let mut bits = Vec::with_capacity(8);
        if self.locked() {
            bits.push("LOCKED");
        }
        if self.error() {
            bits.push("ERROR");
        }
        if self.referenced() {
            bits.push("REFERENCED");
        }
        if self.uptodate() {
            bits.push("UPTODATE");
        }
        if self.dirty() {
            bits.push("DIRTY");
        }
        if self.lru() {
            bits.push("LRU");
        }
        if self.active() {
            bits.push("ACTIVE");
        }
        if self.slab() {
            bits.push("SLAB");
        }
        if self.writeback() {
            bits.push("WRITEBACK");
        }
        if self.reclaim() {
            bits.push("RECLAIM");
        }
        if self.buddy() {
            bits.push("BUDDY");
        }
        if self.mmap() {
            bits.push("MMAP");
        }
        if self.anon() {
            bits.push("ANON");
        }
        if self.swapcache() {
            bits.push("SWAPCACHE");
        }
        if self.swapbacked() {
            bits.push("SWAPBACKED");
        }
        if self.compound_head() {
            bits.push("COMPOUND_HEAD");
        }
        if self.compound_tail() {
            bits.push("COMPOUND_TAIL");
        }
        if self.huge() {
            bits.push("HUGE");
        }
        if self.unevictable() {
            bits.push("UNEVICTABLE");
        }
        if self.hwpoison() {
            bits.push("HWPOISON");
        }
        if self.nopage() {
            bits.push("NOPAGE");
        }
        if self.ksm() {
            bits.push("KSM");
        }
        if self.thp() {
            bits.push("THP");
        }
        if self.balloon() {
            bits.push("BALLOON");
        }
        if self.zero_page() {
            bits.push("ZERO_PAGE");
        }
        if self.idle() {
            bits.push("IDLE");
        }

        f.debug_struct("KPageFlags")
            .field("value", &crate::fmt::Binary(&self.0))
            .field("bits", &bits.as_slice())
            .finish()
    }
}
