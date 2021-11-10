use ckb_testtool::ckb_error::Error;
use ckb_testtool::ckb_types::bytes::Bytes;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[cfg(test)]
mod issuer_tests;

#[cfg(test)]
mod class_tests;

#[cfg(test)]
mod nft_tests;

#[cfg(test)]
mod compact_registry_tests;

#[cfg(test)]
mod compact_nft_mint_tests;

#[cfg(test)]
mod compact_transfer_withdraw_tests;

#[cfg(test)]
mod compact_transfer_claim_tests;

const TEST_ENV_VAR: &str = "CAPSULE_TEST_ENV";

pub enum TestEnv {
    Debug,
    Release,
}

impl FromStr for TestEnv {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(TestEnv::Debug),
            "release" => Ok(TestEnv::Release),
            _ => Err("no match"),
        }
    }
}

pub struct Loader(PathBuf);

impl Default for Loader {
    fn default() -> Self {
        let test_env = match env::var(TEST_ENV_VAR) {
            Ok(val) => val.parse().expect("test env"),
            Err(_) => TestEnv::Debug,
        };
        Self::with_test_env(test_env)
    }
}

impl Loader {
    fn with_test_env(env: TestEnv) -> Self {
        let load_prefix = match env {
            TestEnv::Debug => "debug",
            TestEnv::Release => "release",
        };
        let dir = env::current_dir().unwrap();
        let mut base_path = PathBuf::new();
        base_path.push(dir);
        base_path.push("..");
        base_path.push("build");
        base_path.push(load_prefix);
        Loader(base_path)
    }

    pub fn load_binary(&self, name: &str) -> Bytes {
        let mut path = self.0.clone();
        path.push(name);
        fs::read(path).expect("binary").into()
    }
}

pub fn assert_script_error(err: Error, err_code: i8) {
    let error_string = err.to_string();
    assert!(
        error_string.contains(format!("error code {} ", err_code).as_str()),
        "error_string: {}, expected_error_code: {}",
        error_string,
        err_code
    );
}

pub fn assert_script_errors(err: Error, err_codes: &[i8]) {
    let error_string = err.to_string();
    let mut result = false;
    let mut err_code_ = 0i8;
    for err_code in err_codes {
        if error_string.contains(format!("error code {} ", err_code).as_str()) {
            result = true;
            err_code_ = *err_code;
        }
    }
    assert!(
        result,
        "error_string: {}, expected_error_code: {}",
        error_string, err_code_
    );
}

pub const TYPE: u8 = 1;
pub const CLASS_TYPE_CODE_HASH: [u8; 32] = [
    9, 91, 140, 11, 78, 81, 164, 95, 149, 58, 205, 31, 205, 30, 57, 72, 159, 38, 117, 180, 188,
    148, 231, 175, 39, 187, 56, 149, 135, 144, 227, 252,
];

pub const BYTE32_ZEROS: [u8; 32] = [0u8; 32];
pub const BYTE22_ZEROS: [u8; 22] = [0u8; 22];
pub const BYTE4_ZEROS: [u8; 4] = [0u8; 4];
pub const BYTE3_ZEROS: [u8; 3] = [0u8; 3];

pub const OWNED_SMT_TYPE: u8 = 1u8;
pub const WITHDRAWAL_SMT_TYPE: u8 = 2u8;
pub const CLAIMED_SMT_TYPE: u8 = 3u8;
