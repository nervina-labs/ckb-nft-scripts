#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(asm)]

mod entry;
mod validator;

use ckb_std::default_alloc;

ckb_std::entry!(program_entry);
default_alloc!();

fn program_entry() -> i8 {
    match entry::main() {
        Ok(_) => 0,
        Err(err) => err as i8,
    }
}
