extern crate alloc;

#[path = "../../contracts/nft-type/src/entry.rs"]
mod entry;

fn main() {
    if let Err(err) = entry::main() {
        std::process::exit(err as i32);
    }
}
