use std::path::PathBuf;

use anyhow::Result;
use postgres::row::Row;
use postgres::Client;

use crate::base::{Hash, Path};

pub struct Connection {
    client: Client,
    schema: String,
    project: u32,
}

impl Connection {
    pub fn new<S: Into<String>>(client: Client, schema: S, project: u32) -> Self {
        Self {
            client,
            schema: schema.into(),
            project,
        }
    }

    fn paths_table(&self) -> String {
        format!("{}.paths", self.schema)
    }

    fn contents_table(&self) -> String {
        format!("{}.contents", self.schema)
    }
}

struct RawPath {
    version: i64,
    path: String,
    hash: Hash,
    mode: i32,
}

impl RawPath {
    fn write(&self, conn: &mut Connection) -> Result<()> {
        let paths_table = conn.paths_table();
        let (uuid_1, uuid_2) = self.hash.uuids();
        let mut transaction = conn.client.transaction()?;

        transaction.execute(
            format!(
                "
                UPDATE {} SET stop_version = $1
                    WHERE project = $2
                      AND path = $3
                      AND stop_version IS NULL;
                ",
                paths_table
            )
            .as_str(),
            &[&self.version, &(conn.project as i32), &self.path],
        )?;

        transaction.execute(
            format!(
                "
                INSERT INTO {} (project, start_version, stop_version, path, hash, mode)
                    VALUES ($1, $2, NULL, $3, ($4, $5), $6);
                ",
                paths_table
            )
            .as_str(),
            &[
                &(conn.project as i32),
                &(self.version as i64),
                &self.path,
                &uuid_1,
                &uuid_2,
                &(self.mode as i32),
            ],
        )?;

        Ok(transaction.commit()?)
    }
}

struct RawContent {
    hash: Hash,
    bytes: Vec<u8>,
}

impl RawContent {
    fn write(&self, conn: &mut Connection) -> Result<()> {
        let (uuid_1, uuid_2) = self.hash.uuids();

        conn.client.execute(
            format!(
                "INSERT INTO {} (hash, bytes) VALUES (($1, $2), $3);",
                conn.contents_table()
            )
            .as_str(),
            &[&uuid_1, &uuid_2, &self.bytes],
        )?;

        Ok(())
    }
}

pub enum Query {
    ListLatest(PathBuf, bool),
    ReadLatest(PathBuf, bool),
    List(PathBuf, bool, u64),
    Read(PathBuf, bool, u64),
}

impl Query {
    pub fn execute(&self, conn: &mut Connection) -> Result<Vec<Path>> {
        let rows = match self {
            Query::ListLatest(path, with_contents) => {
                Self::list_latest(conn, &path.to_string_lossy(), *with_contents)
            }
            Query::ReadLatest(path, with_contents) => {
                Self::read_latest(conn, &path.to_string_lossy(), *with_contents)
            }
            Query::List(path, with_contents, version) => {
                Self::list_version(conn, &path.to_string_lossy(), *with_contents, *version)
            }
            Query::Read(path, with_contents, version) => {
                Self::read_version(conn, &path.to_string_lossy(), *with_contents, *version)
            }
        }?;

        let paths = rows
            .into_iter()
            .map(|row| {
                let path: &str = row.get(0);
                let mode: i32 = row.get(1);
                let bytes = if row.len() == 3 { row.get(2) } else { vec![] };
                Path::new(PathBuf::from(path), bytes, mode as u32)
            })
            .collect();

        Ok(paths)
    }

    fn read_latest(conn: &mut Connection, path: &str, with_contents: bool) -> Result<Vec<Row>> {
        let (select, join) = Self::join_clause(conn, with_contents);
        let query = format!(
            "
            SELECT p.path, p.mode {}
            FROM {} p
            {}
            WHERE p.project = $1
              AND p.stop_version IS NULL
              AND p.path = $2;
            ",
            select,
            conn.paths_table(),
            join
        );

        Ok(conn
            .client
            .query(query.as_str(), &[&(conn.project as i32), &path])?)
    }

    fn read_version(
        conn: &mut Connection,
        path: &str,
        with_contents: bool,
        version: u64,
    ) -> Result<Vec<Row>> {
        let (select, join) = Self::join_clause(conn, with_contents);
        let query = format!(
            "
            SELECT p.path, p.mode {}
            FROM {} p
            {}
            WHERE p.project = $1
              AND p.start_version <= $2
              AND (p.stop_version IS NULL OR p.stop_version > $2)
              AND p.path = $3;
            ",
            select,
            conn.paths_table(),
            join
        );

        Ok(conn.client.query(
            query.as_str(),
            &[&(conn.project as i32), &(version as i64), &path],
        )?)
    }

    fn list_latest(conn: &mut Connection, path: &str, with_contents: bool) -> Result<Vec<Row>> {
        let (select, join) = Self::join_clause(conn, with_contents);
        let path_matcher = format!("{}%", path);
        let query = format!(
            "
            SELECT p.path, p.mode {}
            FROM {} p
            {}
            WHERE p.project = $1
              AND p.stop_version IS NULL
              AND p.path LIKE $2;
            ",
            select,
            conn.paths_table(),
            join
        );

        Ok(conn
            .client
            .query(query.as_str(), &[&(conn.project as i32), &path_matcher])?)
    }

    fn list_version(
        conn: &mut Connection,
        path: &str,
        with_contents: bool,
        version: u64,
    ) -> Result<Vec<Row>> {
        let (select, join) = Self::join_clause(conn, with_contents);
        let path_matcher = format!("{}%", path);
        let query = format!(
            "
            SELECT p.path, p.mode {}
            FROM {} p
            {}
            WHERE p.project = $1
              AND p.start_version <= $2
              AND (p.stop_version IS NULL OR p.stop_version > $2)
              AND p.path LIKE $3;
            ",
            select,
            conn.paths_table(),
            join
        );

        Ok(conn.client.query(
            query.as_str(),
            &[&(conn.project as i32), &(version as i64), &path_matcher],
        )?)
    }

    fn join_clause(conn: &Connection, with_contents: bool) -> (String, String) {
        if with_contents {
            (
                ", c.bytes".to_owned(),
                format!(
                    "LEFT JOIN {} c
                            ON c.hash = p.hash",
                    conn.contents_table()
                ),
            )
        } else {
            ("".to_owned(), "".to_owned())
        }
    }
}

pub fn setup_schema(conn: &mut Connection) -> Result<()> {
    conn.client
        .execute(format!("CREATE SCHEMA {};", &conn.schema).as_str(), &[])?;

    conn.client
        .execute("CREATE TYPE hash AS (d1 uuid, d2 uuid);", &[])?;

    conn.client.execute(
        format!(
            "
            CREATE TABLE {} (
                project       integer,
                start_version bigint,
                stop_version  bigint,
                path          text,
                hash          hash,
                mode          integer
            );
            ",
            conn.paths_table()
        )
        .as_str(),
        &[],
    )?;

    conn.client.execute(
        format!(
            "
            CREATE TABLE {} (
                hash  hash,
                bytes bytea
            );
            ",
            conn.contents_table()
        )
        .as_str(),
        &[],
    )?;

    Ok(())
}

pub fn teardown_schema(conn: &mut Connection) -> Result<()> {
    conn.client.execute(
        format!("DROP SCHEMA IF EXISTS {} CASCADE;", &conn.schema).as_str(),
        &[],
    )?;
    conn.client.execute("DROP TYPE IF EXISTS hash;", &[])?;
    Ok(())
}

pub fn write(conn: &mut Connection, path: Path, version: u64) -> Result<()> {
    let hash = path.hash_bytes()?;

    let raw_path = RawPath {
        version: version as i64,
        path: path.path.to_string_lossy().into_owned(),
        hash: hash,
        mode: path.mode as i32,
    };

    let raw_contents = RawContent {
        hash: hash,
        bytes: path.bytes,
    };

    raw_contents.write(conn)?;
    raw_path.write(conn)
}
