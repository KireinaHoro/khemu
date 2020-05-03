extern crate log;

use std::collections::BTreeMap;
use std::path::Path;
use std::{env, fs};

pub trait GetRegion<'a> {
    fn get_region(&self, loc: usize) -> Option<(usize, &'a [u8])>;
}

pub type GuestMap<'a> = BTreeMap<usize, &'a [u8]>;

impl<'a> GetRegion<'a> for GuestMap<'a> {
    fn get_region(&self, loc: usize) -> Option<(usize, &'a [u8])> {
        let kv = self.range(..=loc).next_back();
        if let Some((&k, &v)) = kv {
            if k + v.len() > loc {
                return Some((k, v));
            }
        }

        None
    }
}

pub fn read_elf() -> Result<Vec<u8>, String> {
    let args: Vec<_> = env::args().collect();
    let prog_path;
    match args.len() {
        2 => prog_path = Path::new(args[1].as_str()),
        _ => return Err(format!("usage: {} <ELF name>", args[0])),
    };

    match fs::read(prog_path) {
        Ok(b) => Ok(b),
        Err(e) => Err(format!("failed to read {}: {}", prog_path.display(), e)),
    }
}

pub mod loader;
