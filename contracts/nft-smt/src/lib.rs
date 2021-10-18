#![cfg_attr(not(feature = "std"), no_std)]

pub mod registry;

pub use registry::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        pub use ckb_types::{self, molecule};
        pub use std::vec;
        pub use std::borrow::ToOwned;
    } else  if #[cfg(feature = "no-std")] {
        pub use ckb_std::ckb_types;
        pub use molecule;
        extern crate alloc;
        pub use alloc::vec;
        pub use alloc::borrow::ToOwned;
    }
}

