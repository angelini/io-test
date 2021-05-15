use std::convert::TryInto;
use std::path::PathBuf;

use anyhow::Result;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn uuids(&self) -> (Uuid, Uuid) {
        (
            Uuid::from_bytes(self.0[0..16].try_into().unwrap()),
            Uuid::from_bytes(self.0[0..16].try_into().unwrap()),
        )
    }
}

#[derive(Debug)]
pub struct Path {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub mode: u32,
}

impl Path {
    pub fn new(path: PathBuf, bytes: Vec<u8>, mode: u32) -> Self {
        Self {
            path,
            bytes,
            mode,
        }
    }

    pub fn hash_bytes(&self) -> Result<Hash> {
        let hash: [u8; 32] = Sha256::digest(&self.bytes).try_into()?;
        Ok(Hash(hash))
    }
}
