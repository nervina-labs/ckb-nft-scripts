#![no_std]
#![no_main]
#![feature(asm)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

mod entry;

use ckb_std::default_alloc;

pub use script_utils::error;

ckb_std::entry!(program_entry);
default_alloc!();

fn program_entry() -> i8 {
    match entry::main() {
        Ok(_) => 0,
        Err(err) => err as i8,
    }
}
