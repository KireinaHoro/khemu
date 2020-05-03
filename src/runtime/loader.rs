use crate::guest::arm64::Arm64GuestContext;
use crate::guest::Disassembler;
use crate::host::HostContext;
use crate::ir::storage::HostStorage;
use crate::runtime::GuestMap;
use log::info;

use goblin::elf;
use goblin::elf::header::*;
use goblin::elf::program_header::pt_to_str;

macro_rules! dispatch_machine {
    ( $buffer:expr, $binary:expr => $( $em:ident: $handler:expr; )* ) => {
        match $binary.header.e_machine {
            $(
                $em => $handler($buffer, $binary, true),
            )*
            _ => Err(format!(
                "unsupported architecture: {}",
                machine_to_str($binary.header.e_machine)
            )),
        }
    }
}

fn load_aarch64<'a, R: HostStorage>(
    buffer: &'a [u8],
    binary: &elf::Elf,
    is_exec: bool,
) -> Result<Arm64GuestContext<'a, R>, String> {
    if !is_exec {
        return Err("loading shared libraries are not supported yet".to_owned());
    } else if binary.header.e_type != ET_EXEC && binary.header.e_type != ET_DYN {
        return Err(format!(
            "requested to load executable (EXEC or DYN) but ELF type is {}",
            et_to_str(binary.header.e_type)
        ));
    }

    if let Some(_) = binary.dynamic {
        return Err("dynamically linked executable not supported yet".to_owned());
    }

    let mut map = GuestMap::new();
    for ph in &binary.program_headers {
        if ph.p_type == elf::program_header::PT_LOAD {
            info!(
                "{}: reading {:#x} bytes for {:#x}",
                pt_to_str(ph.p_type),
                ph.p_memsz,
                ph.p_vaddr
            );
            let data = &buffer[ph.p_offset as usize..(ph.p_offset + ph.p_filesz) as usize];
            let f: &[u8];
            if ph.is_write() || ph.p_memsz > ph.p_filesz {
                let mut copied = vec![0u8; ph.p_memsz as usize];
                copied[..ph.p_filesz as usize].copy_from_slice(&data);
                f = Vec::leak(copied);
            } else {
                f = data;
            }
            map.insert(ph.p_vaddr as usize, f);
        }
    }

    info!("Entry point: {:#x}", binary.entry);

    Ok(Arm64GuestContext::<R>::new(map, binary.entry as usize))
}

pub fn load_program<'a, R: 'a + HostStorage>(
    buffer: &'a [u8],
) -> Result<Box<dyn Disassembler<R> + 'a>, String> {
    let binary = match elf::Elf::parse(buffer) {
        Ok(b) => b,
        Err(e) => return Err(format!("failed to parse ELF: {}", e)),
    };

    Ok(Box::new(dispatch_machine! { buffer, &binary =>
        EM_AARCH64: load_aarch64::<R>;
    }?))
}
