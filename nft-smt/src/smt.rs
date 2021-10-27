pub use blake2b_ref::{Blake2b, Blake2bBuilder};
use sparse_merkle_tree::{traits::Hasher, SparseMerkleTree};

// re-exports
pub use sparse_merkle_tree::{default_store::DefaultStore, CompiledMerkleProof, MerkleProof, H256};

pub type SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;

const BLAKE2B_KEY: &[u8] = &[];
const BLAKE2B_LEN: usize = 32;
const PERSONALIZATION: &[u8] = b"ckb-default-hash";

pub struct Blake2bHasher(Blake2b);

impl Default for Blake2bHasher {
    fn default() -> Self {
        let blake2b = Blake2bBuilder::new(BLAKE2B_LEN)
            .personal(PERSONALIZATION)
            .key(BLAKE2B_KEY)
            .build();
        Blake2bHasher(blake2b)
    }
}

impl Hasher for Blake2bHasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }

    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b][..]);
    }

    fn finish(self) -> H256 {
        let mut hash = [0u8; 32];
        self.0.finalize(&mut hash);
        hash.into()
    }
}

pub fn new_blake2b() -> Blake2b {
    Blake2bBuilder::new(32)
        .personal(PERSONALIZATION)
        .key(BLAKE2B_KEY)
        .build()
}

pub fn blake2b_256<T: AsRef<[u8]>>(s: T) -> [u8; 32] {
    let mut result = [0u8; 32];
    let mut blake2b = new_blake2b();
    blake2b.update(s.as_ref());
    blake2b.finalize(&mut result);
    result
}
