extern crate alloc;

#[path = "../../../contracts/nft-type/src/entry.rs"]
mod entry;
#[path = "../../../contracts/nft-type/src/validator.rs"]
mod validator;

fn main() {
    if let Err(err) = entry::main() {
        std::process::exit(err as i32);
    }
}
