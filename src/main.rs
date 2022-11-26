#![feature(cstr_from_bytes_until_nul)]

use anyhow::*;
use std::env;
use std::fs::File;
use std::io::Read;

mod elf;

use elf::*;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    ensure!(args.len() >= 2, "Usage: rlvd obj.o");

    let file_name = &args[1];
    println!("input: {}", file_name);

    let mut f = File::open(&file_name)?;

    let mut content = vec![];
    f.read_to_end(&mut content)?;

    let elf = ElfFile::new(&content)?;
    println!("elf sections: {}", elf.section_num());
    elf.print();

    Ok(())
}
