use crate::guest::arm64::Arm64GuestContext;
use crate::guest::Disassembler;
use crate::ir::storage::HostStorage;
use crate::runtime::*;
use log::info;

use goblin::elf;
use goblin::elf::header::*;
use goblin::elf::program_header::pt_to_str;
use std::rc::Rc;

pub fn load_program<R: HostStorage>(
    buffer: Vec<u8>,
    handler: Box<dyn FnMut(u64, u64)>,
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

            let mut map: GuestMap = GuestMapMethods::new();
            for ph in &binary.program_headers {
                if ph.p_type == elf::program_header::PT_LOAD {
                    info!(
                        "{}: reading {:#x} bytes for {:#x}",
                        pt_to_str(ph.p_type),
                        ph.p_memsz,
                        ph.p_vaddr
                    );
                    let data = &buffer[ph.p_offset as usize..(ph.p_offset + ph.p_filesz) as usize];
                    let f;
                    if ph.is_write() || ph.p_memsz > ph.p_filesz {
                        let mut copied = vec![0u8; ph.p_memsz as usize];
                        copied[..ph.p_filesz as usize].copy_from_slice(&data);
                        f = copied;
                    } else {
                        f = data.to_owned();
                    }
                    map.borrow_mut().insert(ph.p_vaddr as usize, f);
                }
            }

            info!("Entry point: {:#x}", binary.entry);

            R::HostContext::init(Rc::clone(&map), handler);

            Ok((Arm64GuestContext::<R>::new(map), binary.entry))
        }
        _ => Err(format!(
            "unsupported architecture {}",
            elf::header::machine_to_str(binary.header.e_machine)
        )),
    }
}
