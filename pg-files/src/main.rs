mod api;
mod base;
mod storage;

use std::fs::{self, File, Metadata};
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::Result;
use clap::{self, value_t, App, AppSettings, Arg, SubCommand};
use postgres::{self, Client, Config, NoTls};
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;
use walkdir::WalkDir;

use crate::base::Path;
use crate::storage::Query;

const CHUNK_SIZE: usize = 50;

type PathAndMeta<'a> = (PathBuf, Metadata);

fn read_path(prefix: &str, path: &std::path::Path, meta: &Metadata) -> Result<Option<Path>> {
    if !meta.is_file() {
        // FIXME: Support directories
        return Ok(None);
    }

    let mut bytes = vec![0; meta.len() as usize];
    File::open(path)?.read(&mut bytes)?;

    Ok(Some(Path::new(
        path.strip_prefix(prefix)?.to_path_buf(),
        bytes,
        meta.permissions().mode(),
    )))
}

fn write_chunk(
    config: Config,
    schema: &str,
    project: u32,
    dir: &str,
    version: u64,
    chunk: &[PathAndMeta],
) -> Result<()> {
    let client = config.connect(NoTls)?;
    let mut conn = storage::Connection::new(client, schema, project);
    for (path, metadata) in chunk {
        if let Some(path) = read_path(dir, path, metadata)? {
            storage::write(&mut conn, path, version)?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let matches = App::new("pg-files")
        .arg(
            Arg::with_name("connect")
                .short("c")
                .long("connect")
                .takes_value(true)
                .default_value("host=127.0.0.1 user=postgres"),
        )
        .arg(
            Arg::with_name("schema")
                .short("s")
                .long("schema")
                .takes_value(true)
                .required(true),
        )
        .subcommand(
            SubCommand::with_name("setup").about("teardown and setup the schema, types and tables"),
        )
        .subcommand(
            SubCommand::with_name("init_project")
                .about("write all files from a directory as version 1")
                .arg(
                    Arg::with_name("project")
                        .short("p")
                        .long("project")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("dir")
                        .short("d")
                        .long("dir")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("update_project")
                .about("update changed files to a new version")
                .arg(
                    Arg::with_name("project")
                        .short("p")
                        .long("project")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("version")
                        .short("v")
                        .long("version")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("dir")
                        .short("d")
                        .long("dir")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("diff")
                        .long("diff")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("query")
                .about("query existing files")
                .arg(
                    Arg::with_name("project")
                        .short("p")
                        .long("project")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("version")
                        .short("v")
                        .long("version")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("mode")
                        .short("m")
                        .long("mode")
                        .takes_value(true)
                        .possible_values(&["read", "list"])
                        .required(true),
                )
                .arg(
                    Arg::with_name("path")
                        .long("path")
                        .takes_value(true)
                        .default_value(""),
                )
                .arg(Arg::with_name("content").short("c").long("content")),
        )
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    let config = matches.value_of("connect").unwrap().parse::<Config>()?;
    let client = Client::connect(matches.value_of("connect").unwrap(), NoTls)?;
    let schema = matches.value_of("schema").unwrap();

    if let Some(_) = matches.subcommand_matches("setup") {
        let mut conn = storage::Connection::new(client, schema, 0);
        storage::teardown_schema(&mut conn)?;
        storage::setup_schema(&mut conn)?;
    } else if let Some(init_matches) = matches.subcommand_matches("init_project") {
        let project = value_t!(init_matches, "project", u32)?;
        let dir = init_matches.value_of("dir").unwrap();

        let entries = WalkDir::new(dir)
            .into_iter()
            .map(|entry| {
                let entry = entry?;
                Ok((entry.path().to_path_buf(), entry.metadata()?))
            })
            .collect::<Result<Vec<PathAndMeta>>>()?;

        entries.par_chunks(CHUNK_SIZE).try_for_each(|chunk| {
            write_chunk(config.clone(), schema, project, dir, 1, chunk)
        })?;
    } else if let Some(update_matches) = matches.subcommand_matches("update_project") {
        let project = value_t!(update_matches, "project", u32)?;
        let version = value_t!(update_matches, "version", u64)?;
        let dir = update_matches.value_of("dir").unwrap();
        let diff = update_matches.value_of("diff").unwrap();

        let lines = BufReader::new(File::open(diff)?)
            .lines()
            .map(|line| {
                let path = PathBuf::from(line?);
                let meta = fs::metadata(&path)?;
                Ok((path, meta))
            })
            .collect::<Result<Vec<PathAndMeta>>>()?;

        lines.par_chunks(CHUNK_SIZE).try_for_each(|chunk| {
            write_chunk(config.clone(), schema, project, dir, version, chunk)
        })?;
    } else if let Some(query_matches) = matches.subcommand_matches("query") {
        let project = value_t!(query_matches, "project", u32)?;
        let mode = query_matches.value_of("mode").unwrap();
        let path = PathBuf::from(query_matches.value_of("path").unwrap());
        let content = query_matches.is_present("content");

        let query = match mode {
            "read" => match query_matches.value_of("version") {
                Some(version) => {
                    let version: u64 = version.parse()?;
                    Query::Read(path, content, version)
                }
                None => Query::ReadLatest(path, content),
            },
            "list" => match query_matches.value_of("version") {
                Some(version) => {
                    let version: u64 = version.parse()?;
                    Query::List(path, content, version)
                }
                None => Query::ListLatest(path, content),
            },
            _ => unreachable!(),
        };

        let mut conn = storage::Connection::new(client, schema, project);

        for path in query.execute(&mut conn)? {
            println!("{:?}", path);
        }
    }

    Ok(())
}
