#![allow(warnings)]
use std::{
    convert::TryFrom,
    io::{
        BufRead,
        Read,
    },
    path::{
        Path,
        PathBuf,
    },
    str::FromStr,
};

use nix::sys::ptrace::Options;

use crate::deps::{
    beholder::{
        mmapfile::{
            MmapFile,
            MmapOptions,
        },
        pagemaps::{
            PageDescriptor,
            PageSize,
            ProcessVMA,
        },
    },
    log::{
        debug,
        info,
        warn,
    },
    nix::sys::mman::{
        MapFlags,
        ProtFlags,
    },
    structopt::StructOpt,
};

pub mod deps {
    pub(crate) use env_logger;
    pub(crate) use log;
    pub(crate) use nix;
    pub(crate) use structopt;

    pub(crate) use beholder;
}


mod cli {
    pub fn println<T>(
        value: &T,
        verbose: bool,
    ) where
        T: std::fmt::Debug,
    {
        if verbose {
            println!("{:#?}", value);
        } else {
            println!("{:?}", value);
        }
    }

    pub fn parse_hex(number: &str) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(usize::from_str_radix(number, 16)?)
    }
}

macro_rules! panic_on_err {
    () => {
        |err| panic!("[ERROR] {}:{}: {}", err, module_path!(), line!())
    };
}


#[derive(Copy, Clone, Debug, PartialEq)]
enum Data {
    Maps,
    Pages,
}


impl FromStr for Data {
    type Err = crate::deps::beholder::error::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "maps" => Ok(Data::Maps),
            "pages" => Ok(Data::Pages),
            bad_value => {
                Err(crate::deps::beholder::error::Error::Parse {
                    value:    value.to_string(),
                    typename: std::any::type_name::<Data>(),
                    reason:   "value was not one of: maps, regions, pages".to_string(),
                })
            }
        }
    }
}


#[derive(Copy, Clone, Debug, PartialEq)]
enum Assert {
    Off,
    Warn,
    Panic,
}

impl Assert {
    pub fn do_assert(
        &self,
        page: &PageDescriptor,
        expected_value: bool,
    ) {
        match self {
            Assert::Panic => {
                assert_eq!(
                    page.pte.is_soft_dirty(),
                    expected_value,
                    "ASSERTION FAILED: expected page softdirty pte to be '{}' but found '{}'\n{:#?}",
                    expected_value,
                    page.pte.is_soft_dirty(),
                    page,
                );
            }
            Assert::Warn => {
                if page.pte.is_soft_dirty() != expected_value {
                    warn!(
                        "ASSERTION FAILED: expected page softdirty pte to be '{}' but found '{}'\n{:#?}",
                        expected_value,
                        page.pte.is_soft_dirty(),
                        page,
                    );
                }
            }
            Assert::Off => {}
        }
    }
}

impl FromStr for Assert {
    type Err = crate::deps::beholder::error::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "off" => Ok(Assert::Off),
            "warn" => Ok(Assert::Warn),
            "panic" => Ok(Assert::Panic),
            bad_value => {
                Err(crate::deps::beholder::error::Error::Parse {
                    value:    value.to_string(),
                    typename: std::any::type_name::<Assert>(),
                    reason:   "value was not one of: off, warn, panic".to_string(),
                })
            }
        }
    }
}


#[derive(Debug, StructOpt)]
#[structopt(name = "beholder", about = "pagemap parsing")]
struct Args {
    #[structopt(short, long)]
    debug: bool,

    #[structopt(short, long)]
    verbose: bool,

    #[structopt(subcommand)]
    cmd: Command,
}


#[derive(Clone, Debug, StructOpt, PartialEq)]
enum Command {
    DirtyCounts(DirtyCounts),
    Print(Print),
    Demo(Demo),
}


#[derive(Clone, Debug, StructOpt, PartialEq)]
struct Print {
    #[structopt(short, long)]
    pid: Option<usize>,

    #[structopt(short, long, parse(try_from_str = cli::parse_hex))]
    region: Option<usize>,

    #[structopt(short, long)]
    select: Option<Vec<Data>>,

    #[structopt(long)]
    page_size: Option<PageSize>,
}


#[derive(Clone, Debug, StructOpt, PartialEq)]
struct DirtyCounts {
    #[structopt(short, long)]
    pid: Option<usize>,

    #[structopt(short, long, parse(try_from_str = cli::parse_hex))]
    region: Option<usize>,

    #[structopt(long)]
    page_size: Option<PageSize>,
}


#[derive(Clone, Debug, StructOpt, PartialEq)]
struct Demo {
    #[structopt(long, default_value = "/dev/shm/softpte-tracking-demo.mmap", parse(from_os_str))]
    path: PathBuf,

    #[structopt(long, default_value = "3")]
    page_count: usize,

    #[structopt(long, default_value = "3")]
    loops: usize,

    #[structopt(long)]
    page_size: Option<PageSize>,

    #[structopt(long, default_value = "panic")]
    assert: Assert,
}


fn init_process_vma(
    pid: Option<usize>,
    debug: bool,
) -> ProcessVMA {
    let mut vm = match pid {
        Some(pid) => ProcessVMA::with_pid(pid).unwrap_or_else(panic_on_err!()),
        None => ProcessVMA::this_process().unwrap_or_else(panic_on_err!()),
    };

    if debug {
        info!("/proc/{}/maps\n{}", vm.pid(), vm.maps());
    }

    vm
}

fn list_regions(
    vm: &ProcessVMA,
    only_region: Option<usize>,
) -> Vec<usize> {
    if let Some(addr) = only_region {
        vec![addr]
    } else {
        vm.maps().iter().map(|(k, _v)| *k).collect()
    }
}


fn dirty_counts_command(
    args: &Args,
    cmd: &DirtyCounts,
) {
    let (mut dirty, mut clean) = (0, 0);

    let mut vm = init_process_vma(cmd.pid, args.debug);
    let regions = list_regions(&vm, cmd.region);

    for addr in regions.into_iter() {
        let region = vm.region(addr).unwrap();

        let page_iter = region.try_iter(cmd.page_size).unwrap_or_else(panic_on_err!());

        for page_parse_result in page_iter {
            let p = page_parse_result.unwrap_or_else(panic_on_err!());
            if p.pte.is_soft_dirty() {
                dirty += 1;
            } else {
                clean += 1;
            }
        }
    }

    println!("dirty: {}\nclean: {}", dirty, clean);
}


fn print_command(
    args: &Args,
    cmd: &Print,
) {
    let print_maps = cmd
        .select
        .as_ref()
        .map(|only| only.contains(&Data::Maps))
        .unwrap_or(true);
    let print_pages = cmd
        .select
        .as_ref()
        .map(|only| only.contains(&Data::Pages))
        .unwrap_or(true);

    let mut vm = init_process_vma(cmd.pid, args.debug);
    let regions = list_regions(&vm, cmd.region);

    for addr in regions.into_iter() {
        let region = vm
            .region(addr)
            .unwrap_or_else(|| panic!("no such region with starting address {:x}", addr));
        if print_maps {
            cli::println(&region, args.verbose);
        }

        if !print_pages {
            continue;
        }

        let pages_iter = region.try_iter(cmd.page_size).unwrap_or_else(panic_on_err!());
        for page_result in pages_iter {
            let page = page_result.unwrap_or_else(panic_on_err!());
            cli::println(&page, args.verbose);
        }
    }
}

/// Mmap a file. For --loops=n times test the softdirty bits are cleared and set as expected using
/// the behavior defined by --assert=<behavior> to detect a mismatch in expected values.
fn demo_command(
    args: &Args,
    cmd: &Demo,
) {
    let path = &cmd.path;
    let page_size = cmd.page_size.unwrap_or_default() as usize;
    let page_count = cmd.page_count;
    let map_size = page_size * page_count;
    let rounds = 1..=cmd.loops;

    let options = MmapOptions {
        path:           std::borrow::Cow::Borrowed(path),
        base_addr:      0 as *mut _,
        len:            map_size,
        addr_offset:    0,
        remove_on_drop: true,
    };

    let map = MmapFile::new(
        &options,
        ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
        MapFlags::MAP_SHARED | MapFlags::MAP_NORESERVE,
    )
    .unwrap_or_else(panic_on_err!());

    let map_root = map.as_nonnull().as_ptr();

    let mut vm = init_process_vma(None, args.debug);

    // closure to run the assert behavior
    let assert_all_region_softdirty_ptes_are = |expected_value: bool| {
        let region = vm.region(map_root as usize).unwrap_or_else(|| {
            panic!(
                "could not find region corresponding \
                to memory mapped file, address={:p}",
                map_root
            )
        });

        let pages_iter = region.try_iter(cmd.page_size).unwrap_or_else(panic_on_err!());

        for page_result in pages_iter {
            let page = page_result.unwrap_or_else(panic_on_err!());
            cmd.assert.do_assert(&page, expected_value);
        }
    };

    println!("begin demo ({} rounds)", rounds.end());

    let mut chars = b"abcdefghijklmnopqrstuvwxyz".iter().copied().cycle();

    for round in rounds.clone() {
        println!("start round: {} of {}", round, rounds.end());
        vm.clear_refs();
        assert_all_region_softdirty_ptes_are(false);

        let mut page_ptr = map_root;
        for page_num in 0..cmd.page_count {
            println!("{:p} [# {:0>3}]: write 'x'", page_ptr, page_num);
            unsafe {
                *page_ptr = chars.next().unwrap();
                page_ptr = page_ptr.add(page_size);
            };
        }

        assert_all_region_softdirty_ptes_are(true);
        println!("end round: {} of {}", round, rounds.end());
    }

    println!("end demo...")
}


fn main() {
    let args = Args::from_args();
    if args.debug {
        crate::deps::env_logger::builder()
            .filter_level(crate::deps::log::LevelFilter::Debug)
            .init();
    } else {
        crate::deps::env_logger::builder()
            .filter_level(crate::deps::log::LevelFilter::Warn)
            .init();
    }

    debug!("program arguments: {:#?}", args);


    match &args.cmd {
        Command::DirtyCounts(cmd) => dirty_counts_command(&args, cmd),
        Command::Print(cmd) => print_command(&args, cmd),
        Command::Demo(cmd) => demo_command(&args, cmd),
    }
}
