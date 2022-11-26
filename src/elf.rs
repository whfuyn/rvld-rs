use core::mem;
use static_assertions::assert_eq_align;
use core::ffi::CStr;
use bitflags::bitflags;
use anyhow::{
    anyhow, ensure, Error,
};

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELF_HEADER_SIZE: usize = mem::size_of::<ElfHeader>();
const SECTION_HEADER_SIZE: usize = mem::size_of::<SectionHeader>();
/// Section header name's max index. It means index stored elsewhere.
const SHN_XINDEX: u16 = 0xffff;

bitflags! {
    struct SectionHeaderTy: u32 {
        const NULL = 0;
        const PROGBITS = 1;
        const SYMTAB = 2;
        const STRTAB = 3;
        // TODO
    }
}

// Reference:
// https://refspecs.linuxbase.org/elf/gabi4+/ch4.eheader.html
// https://refspecs.linuxbase.org/elf/gabi4+/ch4.intro.html#data_representation
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
    addralign: u64,
    entsize: u64,
}
unsafe impl zero::Pod for SectionHeader {}

/// Zero-Copy ELF file
pub struct ElfFile<'input> {
    input: &'input [u8],
    elf_header: &'input ElfHeader,

    sec_headers: &'input [SectionHeader],
    sh_strtab: StrTable<'input>,

    sym_table: SymTable<'input>,
}

impl<'input> ElfFile<'input> {
    pub fn new(input: &'input [u8]) -> Result<ElfFile<'input>, anyhow::Error> {
        ensure!(input.starts_with(ELF_MAGIC), "input isn't an ELF file");
        let elf_header: &ElfHeader = zero::read(input);

        let sh_off = elf_header.sh_off as usize;
        let first_sh = {
            ensure!(input.len() >= sh_off + SECTION_HEADER_SIZE);
            zero::read::<SectionHeader>(&input[sh_off..])
        };

        let sh_num = match elf_header.sh_num {
            0 => first_sh.size as usize,
            sh_num => sh_num as usize,
        };
        let sh_len = sh_num * SECTION_HEADER_SIZE;

        ensure!(input.len() >= sh_off + sh_len);
        let sec_headers = zero::read_array::<SectionHeader>(&input[sh_off..sh_off + sh_len]);

        let sh_strtab = {
            let sh_strndx = match elf_header.sh_strndx {
                SHN_XINDEX => first_sh.link as usize,
                sh_strndx => sh_strndx as usize,
            };
            let sh_strtab_sh = sec_headers.get(sh_strndx)
                .ok_or_else(|| anyhow!("sh_strtab not found"))?;
            StrTable{ strs: get_section_bytes(input, sh_strtab_sh)? }
        };
        let sym_table = {
            let symtab_sh = sec_headers
                .iter()
                .find(|sh| sh.ty == SectionHeaderTy::SYMTAB.bits())
                .ok_or_else(|| anyhow!("symbol table not found"))?;
            let symbols: &[Symbol] = {
                let bytes = get_section_bytes(input, symtab_sh)?;
                zero::read_array(bytes)
            };
            let sym_strtab = {
                let strs = sec_headers
                    .get(dbg!(symtab_sh.link) as usize)
                    .map(|sh| get_section_bytes(input, sh))
                    .transpose()
                    .ok()
                    .flatten()
                    .ok_or_else(|| anyhow!("symbol table's str table not found"))?;
                StrTable { strs }
            };
            
            SymTable {
                header: symtab_sh,
                symbols,
                sym_strtab,
            }
        };
        Ok(Self {
            input,
            elf_header,
            sec_headers,
            sh_strtab,
            sym_table,
        })
    }

    pub fn section_num(&self) -> usize {
        self.sec_headers.len()
    }

    pub fn print(&self) {
        for (i, sh) in self.sec_headers.iter().enumerate() {
            println!("#{} {}", i, self.sh_strtab.get_str(sh.name as usize).unwrap().to_str().unwrap());
        }
        
        self.sym_table.print();
    }
}

fn get_section_bytes<'input>(input: &'input [u8], sh: &SectionHeader) -> Result<&'input [u8], anyhow::Error> {
    // TODO: handle SHT_NOBITS case
    let b = sh.offset as usize;
    let e = b + sh.size as usize;
    input.get(b..e).ok_or_else(|| anyhow!("get section bytes out of range"))
}

#[repr(C)]
struct Symbol {
    name: u32,
    info: u8,
    other: u8,
    shndx: u16,
    value: u64,
    size: u64,
}
unsafe impl zero::Pod for Symbol {}

struct SymTable<'input> {
    header: &'input SectionHeader,
    symbols: &'input [Symbol],
    sym_strtab: StrTable<'input>,
}

impl<'input> SymTable<'input> {
    pub fn print(&self) {
        for sym in self.symbols {
            println!("{}", self.sym_strtab.get_str(sym.name as usize).unwrap().to_str().unwrap());
        }
    }
}

struct StrTable<'input> {
    strs: &'input [u8],
}

impl<'input> StrTable<'input> {
    pub fn get_str(&self, index: usize) -> Result<&CStr, anyhow::Error> {
        let bytes = self.strs.get(index..).ok_or_else(|| anyhow!("index out of range"))?;
        CStr::from_bytes_until_nul(bytes).map_err(|_| anyhow!("str at {} is invalid", index))
    }
}
