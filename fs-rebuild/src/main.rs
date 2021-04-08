use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::thread;

use anyhow::Result;
use bytes::Buf;
use tar::Archive;
use thiserror::Error;
use zstd::stream::read::Decoder;

const SERVER: &str = "http://127.0.0.1:8080/";
const CHUNK_COUNT: usize = 4;

#[derive(Error, Debug)]
pub enum RebuildError {
    #[error("IO: {0}")]
    Io(#[from] io::Error),
}

fn fetch_chunk(output: PathBuf, idx: usize) -> Result<()> {
    let t0 = Instant::now();

    let resp = reqwest::blocking::get(&format!("{}/{}.tar.zst", SERVER, idx))?;
    let t1 = Instant::now();

    let bytes = resp.bytes()?;
    let decoder = Decoder::new(bytes.reader())?;
    let t2 = Instant::now();

    let mut archive = Archive::new(decoder);
    archive.unpack(output)?;
    let t3 = Instant::now();

    println!("t1({}): {:?}", idx, t1 - t0);
    println!("t2({}): {:?}", idx, t2 - t1);
    println!("t3({}): {:?}", idx, t3 - t2);

    Ok(())
}

fn main() -> Result<()> {
    let output = Path::new("/mnt/data");
    let mut threads = vec![];

    for idx in 1..(CHUNK_COUNT + 1) {
        threads.push(thread::spawn(move || {
            fetch_chunk(output.to_path_buf(), idx)
        }));
    }

    for handle in threads {
        if let Err(error) = handle.join().unwrap() {
            return Err(error)
        }
    }

    Ok(())
}
