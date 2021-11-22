#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(asm)]

use ckb_std::default_alloc;

mod claim_mint;
mod claim_transfer;
mod entry;
mod update_info;
mod withdraw_transfer;

ckb_std::entry!(program_entry);
default_alloc!();

fn program_entry() -> i8 {
    match entry::main() {
        Ok(_) => 0,
        Err(err) => err as i8,
    }
}
