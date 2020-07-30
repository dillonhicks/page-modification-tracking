#![allow(warnings)]
#![cfg_attr(feature = "nightly", feature(backtrace))]

pub(crate) mod deps {
    pub use derive_more;
    pub use lazy_static;
    pub use libc;
    pub use log;
    pub use nix;
    pub use serde;
    pub use thiserror;
}

mod fmt;
mod io;

pub mod error;
pub mod kpageflags;
pub mod maps;
pub mod mmapfile;
pub mod pagemaps;
pub mod paths;
