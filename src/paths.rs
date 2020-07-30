use std::{
    fs::File,
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom,
    },
    mem,
    path::{
        Path,
        PathBuf,
    },
};

use crate::error::Error;

fn pid_to_path(pid: Option<usize>) -> String {
    pid.as_ref().map(ToString::to_string).unwrap_or(String::from("self"))
}


pub fn proc_pid_maps_path(pid: Option<usize>) -> PathBuf {
    Path::new("/").join("proc").join(pid_to_path(pid)).join("maps")
}

pub fn proc_pid_pagemaps_path(pid: Option<usize>) -> PathBuf {
    Path::new("/").join("proc").join(pid_to_path(pid)).join("pagemap")
}


pub fn proc_pid_clear_refs(pid: Option<usize>) -> PathBuf {
    Path::new("/").join("proc").join(pid_to_path(pid)).join("clear_refs")
}


pub fn proc_kpageflags_path() -> &'static Path {
    Path::new("/proc/kpageflags")
}


pub fn proc_kpagecount_path() -> &'static Path {
    Path::new("/proc/kpagecount")
}
