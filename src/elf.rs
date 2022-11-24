use anyhow::*;
use core::mem;
use static_assertions::assert_eq_align;

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELF_HEADER_SIZE: usize = core::mem::size_of::<ElfHeader>();
const SECTION_HEADER_SIZE: usize = core::mem::size_of::<SectionHeader>();

#[derive(Debug)]
#[repr(C)]
pub struct ElfHeader {
    ident: [u8; 16],
    ty: u16,
    machine: u16,
    version: u32,
    entry: u64,
    ph_off: u64,
    sh_off: u64,
    flags: u32,
    eh_size: u16,
    ph_ent_size: u16,
    ph_num: u16,
    sh_ent_size: u16,
    sh_num: u16,
    sh_strndx: u16,
}
unsafe impl zero::Pod for ElfHeader {}

#[derive(Debug)]
#[repr(C)]
struct SectionHeader {
    name: u32,
    ty: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addr_align: u64,
}
unsafe impl zero::Pod for SectionHeader {}

/// Zero-Copy elf file
pub struct ElfFile<'input> {
    input: &'input [u8],
    elf_header: &'input ElfHeader,
    section_headers: &'input [SectionHeader],
}

impl<'input> ElfFile<'input> {
    pub fn new(input: &'input [u8]) -> Result<ElfFile<'input>> {
        ensure!(input.starts_with(ELF_MAGIC), "input isn't an ELF file");
        let elf_header: &ElfHeader = zero::read(input);

        let sh_off = elf_header.sh_off as usize;
        let sh_num = match dbg!(elf_header.sh_num) {
            0 => {
                ensure!(input.len() >= sh_off + SECTION_HEADER_SIZE);
                zero::read::<SectionHeader>(&input[sh_off..]).size as usize
            }
            sh_num => sh_num as usize,
        };
        dbg!(sh_off);
        dbg!(sh_num);
        let sh_len = sh_num * SECTION_HEADER_SIZE;
        dbg!(sh_len);

        ensure!(input.len() >= sh_off + sh_len);
        let section_headers = zero::read_array::<SectionHeader>(&input[sh_off..sh_off + sh_len]);
        Ok(Self {
            input,
            elf_header,
            section_headers,
        })
    }

    pub fn section_num(&self) -> usize {
        self.section_headers.len()
    }
}
