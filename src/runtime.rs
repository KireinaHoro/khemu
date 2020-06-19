extern crate log;

use crate::guest::*;
use crate::host::{HostBlock, HostContext};

use log::*;
use memmap::{MmapMut, MmapOptions};

use std::cell::{Ref, RefCell};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::Path;
use std::rc::Rc;
use std::{env, fs};

pub const DEFAULT_TB_SIZE: usize = 4096;

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

// 2GB guest virtual space
const GUEST_SIZE: usize = 0x2000_0000;

pub type GuestMap = Rc<RefCell<MmapMut>>;

// returns the base of map
pub fn map_virtual() -> Result<GuestMap, String> {
    MmapOptions::new()
        .len(GUEST_SIZE)
        .map_anon()
        .map(|x| Rc::new(RefCell::new(x)))
        .map_err(|e| format!("failed to map guest virtual space: {}", e))
}

pub mod loader;

pub fn do_work<C: HostContext + 'static>() -> Result<(), String> {
    let elf = read_elf()?;

    // trap handler
    // TODO(jsteward) lookup new start_positions when LOOKUP_TB is requested
    let handler = Box::new(|cause, val| {
        warn!("Trap: cause={:#x} val={:#x}", cause, val);
    });

    let (mut disassembler, entry_point) = loader::load_program(elf, handler)?;
    let mut start_positions = VecDeque::new();
    let mut blk_cache: HashMap<_, C::BlockType> = HashMap::new();

    start_positions.push_back(entry_point as usize);

    let mut ret = None;
    while let Some(&start_pos) = start_positions.front() {
        match blk_cache.get(&start_pos) {
            // found block, execute
            Some(blk) => {
                info!("Executing host block for guest {:#x}", start_pos);
                blk.execute();
                start_positions.pop_front();
            }
            // not found, translate and insert
            None => {
                let result = disassembler.disas_block(start_pos, DEFAULT_TB_SIZE);
                let tb = disassembler.get_tb();
                match result {
                    DisasException::Unexpected(s) => {
                        error!("Ending TB @ {:#x} with error: {}", tb.start_pc, s);
                        ret = Some(s);
                        break;
                    }
                    e => {
                        info!("Ending TB @ {:#x} with reason: {}", tb.start_pc, e);
                        // find blocks that can be found statically
                        match e {
                            DisasException::Continue(dest) => {
                                // Size exceeded or unconditional jump
                                // insert target right after pending
                                let waiting = start_positions.pop_front().unwrap();
                                start_positions.push_front(dest);
                                start_positions.push_front(waiting);
                            }
                            DisasException::Branch(Some(taken), Some(not_taken)) => {
                                // both destinations are known
                                // TODO(jsteward) modify to fit proper translation branch prediction
                                start_positions.push_back(taken);
                                start_positions.push_back(not_taken);
                            }
                            DisasException::Branch(Some(dest), None)
                            | DisasException::Branch(None, Some(dest)) => {
                                // only one destination is known
                                // TODO(jsteward) modify to fit proper translation branch prediction
                                start_positions.push_back(dest);
                            }
                            _ => {
                                // none of the jump targets are known
                                // bail out, wait for actual LOOKUP trap
                            }
                        }

                        // emit backend instructions
                        let blk = C::get().emit_block(tb, disassembler.get_tracking(), Some(e));

                        // record in cache, run it next round
                        blk_cache.insert(start_pos, blk);
                        continue;
                    }
                }
            }
        }
    }

    match ret {
        Some(r) => Err(r),
        None => Ok(()),
    }
}
