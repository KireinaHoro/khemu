extern crate log;

use crate::guest::*;
use crate::host::dump_ir::DumpIRHostContext;
use crate::host::HostContext;
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::{env, fs};

use log::*;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

pub type GuestMap = Rc<RefCell<BTreeMap<usize, Vec<u8>>>>;

pub trait GuestMapMethods: Sized {
    fn new() -> Self;
    fn get_region(&self, loc: usize) -> (usize, Ref<Vec<u8>>);
}
impl GuestMapMethods for GuestMap {
    fn new() -> Self {
        Rc::new(RefCell::new(BTreeMap::new()))
    }
    fn get_region(&self, loc: usize) -> (usize, Ref<Vec<u8>>) {
        let mut start = 0;
        let ret = Ref::map(self.borrow(), |map| {
            let kv = map.range(..=loc).next_back();
            if let Some((&k, v)) = kv {
                if k + v.len() > loc {
                    start = k;
                    return v;
                } else {
                    panic!("unmapped guest address {}", loc);
                }
            } else {
                panic!("unmapped guest address {}", loc);
            }
        });
        (start, ret)
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

pub fn do_work() -> Result<(), String> {
    let elf = read_elf()?;
    let (mut disassembler, entry_point) = loader::load_program(elf)?;
    let mut host = DumpIRHostContext::new(disassembler.get_guest_map());
    let mut start_positions = VecDeque::new();

    start_positions.push_back(entry_point as usize);

    let mut ret = None;
    println!("IR generated:");
    while let Some(start_pos) = start_positions.pop_front() {
        match disassembler.disas_block(start_pos, 4096) {
            DisasException::Unexpected(s) => {
                ret = Some(s);
                host.emit_block(disassembler.get_tb());
                break;
            }
            e => {
                let tb = disassembler.get_tb();
                info!("TB @ {:#x} finished with \"{}\"", tb.start_pc, e);

                // find blocks that can be found statically
                match e {
                    DisasException::Branch(Some(taken), Some(not_taken)) => {
                        // both destinations are known
                        start_positions.push_back(taken);
                        start_positions.push_back(not_taken);
                    }
                    DisasException::LimitReached(dest)
                    | DisasException::Branch(Some(dest), None)
                    | DisasException::Branch(None, Some(dest)) => {
                        // only one destination is known
                        start_positions.push_back(dest);
                    }
                    _ => {
                        // none of the jump targets are known
                        // bail out, wait for actual LOOKUP trap
                    }
                }

                // emit backend instructions
                host.emit_block(tb);

                // TODO(jsteward) run generated instructions
                // TODO(jsteward) handle trap to add new targets to `start_positions`
            }
        }
    }

    Err(ret.unwrap())
}
