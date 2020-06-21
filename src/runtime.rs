extern crate log;

use crate::guest::*;
use crate::host::{HostBlock, HostContext};
use crate::ir::op::TrapOp;

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

pub type TrapHandler = fn(u64, u64);
static mut START_POSITIONS: Option<VecDeque<usize>> = None;

fn trap_handler<C: HostContext + 'static>(cause: u64, val: u64) {
    let trap_op = TrapOp::from_bits(cause).unwrap();

    C::get().handle_trap();

    match trap_op {
        TrapOp::LOOKUP_TB => {
            info!("Lookup TB: continuing at {:#x}", val);
            // insert target right after pending
            unsafe {
                let waiting = START_POSITIONS.as_mut().unwrap().pop_front().unwrap();
                START_POSITIONS.as_mut().unwrap().push_front(val as usize);
                START_POSITIONS.as_mut().unwrap().push_front(waiting);
            }
        }
        _ => unimplemented!(),
    }
}

pub fn do_work<C: HostContext + 'static>() -> Result<(), String> {
    let elf = read_elf()?;

    let (mut disassembler, entry_point) = loader::load_program(elf, trap_handler::<C>)?;
    let mut blk_cache: HashMap<_, C::BlockType> = HashMap::new();

    unsafe {
        START_POSITIONS = Some(VecDeque::new());
        START_POSITIONS
            .as_mut()
            .unwrap()
            .push_back(entry_point as usize);
    }

    let mut ret = None;
    unsafe {
        while let Some(&start_pos) = START_POSITIONS.as_mut().unwrap().front() {
            match blk_cache.get(&start_pos) {
                // found block, execute
                Some(blk) => {
                    info!("Executing host block for guest {:#x}", start_pos);
                    unsafe {
                        blk.execute();
                    }
                    START_POSITIONS.as_mut().unwrap().pop_front();
                }
                // not found, translate and insert
                None => {
                    let name = format!("func_{}", start_pos);
                    C::get().push_block(&name, true);

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
                                    let waiting =
                                        START_POSITIONS.as_mut().unwrap().pop_front().unwrap();
                                    START_POSITIONS.as_mut().unwrap().push_front(dest);
                                    START_POSITIONS.as_mut().unwrap().push_front(waiting);
                                }
                                DisasException::Branch(Some(taken), Some(not_taken)) => {
                                    // both destinations are known
                                    // TODO(jsteward) modify to fit proper translation branch prediction
                                    START_POSITIONS.as_mut().unwrap().push_back(taken);
                                    START_POSITIONS.as_mut().unwrap().push_back(not_taken);
                                }
                                DisasException::Branch(Some(dest), None)
                                | DisasException::Branch(None, Some(dest)) => {
                                    // only one destination is known
                                    // TODO(jsteward) modify to fit proper translation branch prediction
                                    START_POSITIONS.as_mut().unwrap().push_back(dest);
                                }
                                _ => {
                                    // none of the jump targets are known
                                    // bail out, wait for actual LOOKUP trap
                                }
                            }

                            // emit backend instructions
                            let blk = C::get().emit_block(
                                tb,
                                &name,
                                disassembler.get_tracking(),
                                Some(e),
                            );

                            // record in cache, run it next round
                            blk_cache.insert(start_pos, blk);
                            continue;
                        }
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
