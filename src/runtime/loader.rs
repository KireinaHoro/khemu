// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use crate::guest::arm64::Arm64GuestContext;
use crate::guest::Disassembler;
use crate::ir::storage::HostStorage;
use crate::runtime::*;
use log::info;

use goblin::elf;
use goblin::elf::header::*;
use goblin::elf::program_header::pt_to_str;
use std::rc::Rc;

use std::cell::RefCell;
use std::ops::IndexMut;

/// Loads a guest ELF and creates the frontend context, also known as the disassembler.
pub fn load_program<R: HostStorage>(
    buffer: Vec<u8>,
    handler: TrapHandler,
) -> Result<(impl Disassembler<R>, u64), String> {
    let binary: elf::Elf = match elf::Elf::parse(&buffer) {
        Ok(b) => b,
        Err(e) => return Err(format!("failed to parse ELF: {}", e)),
    };

    match binary.header.e_machine {
        EM_AARCH64 => {
            if binary.header.e_type != ET_EXEC && binary.header.e_type != ET_DYN {
                return Err(format!(
                    "requested to load executable (EXEC or DYN) but ELF type is {}",
                    et_to_str(binary.header.e_type)
                ));
            }

            if let Some(_) = binary.dynamic {
                return Err("dynamically linked executable not supported yet".to_owned());
            }

            // mmap for guest virtual
            let guest_map = map_virtual()?;
            info!(
                "Created guest address space at {:#x}",
                guest_map.as_ptr() as usize
            );

            for ph in &binary.program_headers {
                if ph.p_type == elf::program_header::PT_LOAD {
                    let len = ph.p_filesz as usize;
                    let file_off = ph.p_offset as usize;
                    let virt = ph.p_vaddr as usize;

                    info!(
                        "{}: reading {:#x} bytes for {:#x}",
                        pt_to_str(ph.p_type),
                        len,
                        virt
                    );

                    // memsz may be larger than filesz, in which case the rest is zero-filled
                    let data = &buffer[file_off..file_off + len];
                    guest_map
                        .borrow_mut()
                        .index_mut(virt..virt + len)
                        .copy_from_slice(data);
                }
            }

            info!("Entry point: {:#x}", binary.entry);

            R::HostContext::init(Rc::clone(&guest_map), handler);

            Ok((Arm64GuestContext::<R>::new(guest_map), binary.entry))
        }
        _ => Err(format!(
            "unsupported architecture {}",
            elf::header::machine_to_str(binary.header.e_machine)
        )),
    }
}
