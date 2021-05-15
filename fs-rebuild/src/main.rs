use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use anyhow::Result;
use bytes::Buf;
use clap::{App, AppSettings, Arg, SubCommand};
use tar::{Archive, Builder};
use walkdir::WalkDir;
use zstd::stream::read::Decoder;
use zstd::stream::write::Encoder;

fn fetch_chunk(output: PathBuf, host: String, idx: usize) -> Result<()> {
    let resp = reqwest::blocking::get(&format!("{}/{}.tar.zst", host, idx))?;

    let bytes = resp.bytes()?;
    let decoder = Decoder::new(bytes.reader())?;

    let mut archive = Archive::new(decoder);
    archive.unpack(output)?;

    Ok(())
}

fn rebuild(output: &Path, host: &str, count: usize) -> Result<()> {
    let mut threads = vec![];

    for idx in 1..(count + 1) {
        let host = host.to_string();
        let output = output.to_path_buf();
        threads.push(thread::spawn(move || fetch_chunk(output, host, idx)));
    }

    for handle in threads {
        if let Err(error) = handle.join().unwrap() {
            return Err(error);
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileMeta {
    path: PathBuf,
    size: u64,
}

impl FileMeta {
    fn new(path: PathBuf, size: u64) -> Self {
        Self { path, size }
    }
}

impl Ord for FileMeta {
    fn cmp(&self, other: &Self) -> Ordering {
        self.size.cmp(&other.size)
    }
}

impl PartialOrd for FileMeta {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.size.cmp(&other.size))
    }
}

struct OutputChunk(Vec<FileMeta>);

impl OutputChunk {
    fn size(&self) -> u64 {
        self.0.iter().map(|entry| entry.size).sum()
    }

    fn write(&self, prefix: &Path, output: &Path, idx: usize) -> Result<()> {
        let tar_file = fs::File::create(output.join(&format!("{}.tar", idx)))?;
        let compressed = Encoder::new(tar_file, 0)?;
        let mut archive = Builder::new(compressed);

        println!("writing to {}", &format!("{}.tar.zst", idx));
        println!("writing {} chunks", self.0.len());

        for meta in self.0.iter() {
            let mut file = fs::File::open(&meta.path)?;
            let path = meta.path.strip_prefix(prefix)?;
            archive.append_file(path, &mut file)?;
        }
        Ok(())
    }
}

struct OutputChunks {
    prefix: PathBuf,
    chunks: Vec<OutputChunk>,
}

impl OutputChunks {
    fn new(prefix: PathBuf, count: usize) -> Self {
        let mut chunks = Vec::new();
        for _ in 0..count {
            chunks.push(OutputChunk(vec![]))
        }
        OutputChunks { prefix, chunks }
    }

    fn push(&mut self, meta: FileMeta) {
        let mut min_index = 0;
        let mut min_value = u64::MAX;

        for (idx, chunk) in self.chunks.iter().enumerate() {
            let size = chunk.size();
            if size < min_value {
                min_index = idx;
                min_value = size
            }
        }

        self.chunks[min_index].0.push(meta)
    }

    fn write(&self, output: &Path) -> Result<()> {
        for (idx, chunk) in self.chunks.iter().enumerate() {
            chunk.write(&self.prefix, output, idx + 1)?;
        }
        Ok(())
    }
}

fn build_output_chunks(input: &Path, count: usize) -> Result<OutputChunks> {
    let mut meta_heap = BinaryHeap::new();

    println!("reading from {:?}", input);
    for entry_result in WalkDir::new(input) {
        let entry = entry_result?;
        let meta = entry.metadata()?;
        if meta.is_file() {
            meta_heap.push(FileMeta::new(entry.path().to_path_buf(), meta.len()))
        }
    }

    let mut chunks = OutputChunks::new(input.to_path_buf(), count);
    for meta in meta_heap {
        chunks.push(meta)
    }

    Ok(chunks)
}

fn split(input: &Path, output: &Path, count: usize) -> Result<()> {
    let chunks = build_output_chunks(input, count)?;
    chunks.write(output)
}

fn main() -> Result<()> {
    let matches = App::new("fs-rebuild")
        .arg(
            Arg::with_name("chunks")
                .short("c")
                .long("chunks")
                .default_value("4")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("split")
                .about("split directory into chunks")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("rebuild")
                .about("rebuild chunks into directory")
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("host")
                        .short("h")
                        .long("host")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    let chunks_arg = matches.value_of("chunks").unwrap();
    let chunks_count = chunks_arg.parse::<usize>()?;

    if let Some(split_matches) = matches.subcommand_matches("split") {
        let input = split_matches.value_of("input").unwrap();
        let output = split_matches.value_of("output").unwrap();
        return split(Path::new(input), Path::new(output), chunks_count);
    }

    if let Some(rebuild_matches) = matches.subcommand_matches("rebuild") {
        let output = rebuild_matches.value_of("output").unwrap();
        let host = rebuild_matches.value_of("host").unwrap();
        return rebuild(Path::new(output), host, chunks_count);
    }

    Ok(())
}
