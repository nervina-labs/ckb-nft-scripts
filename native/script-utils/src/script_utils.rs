extern crate alloc;

#[path = "../../contracts/script-utils/src/class.rs"]
mod class;
#[path = "../../contracts/script-utils/src/error.rs"]
mod error;
#[path = "../../contracts/script-utils/src/helper.rs"]
mod helper;
#[path = "../../contracts/script-utils/src/issuer.rs"]
mod issuer;
#[path = "../../contracts/script-utils/src/nft.rs"]
mod nft;
#[path = "../../contracts/script-utils/src/lib.rs"]
mod lib;

fn main() {
    if let Err(err) = entry::main() {
        std::process::exit(err as i32);
    }
}
