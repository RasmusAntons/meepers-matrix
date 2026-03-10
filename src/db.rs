use anyhow::{Error, Result};
use include_dir::{Dir, include_dir};
use lazy_static::lazy_static;
use rusqlite::Connection;
use rusqlite::Error::QueryReturnedNoRows;
use rusqlite_migration::Migrations;
use std::path::Path;
use std::sync::LazyLock;
use std::{env, fs};

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");
static MIGRATIONS: LazyLock<Migrations<'static>> =
    LazyLock::new(|| Migrations::from_directory(&MIGRATIONS_DIR).unwrap());

lazy_static! {
    static ref DB_PATH_STR: String = env::var("DB_DIR").unwrap_or("./data".to_string());
}

pub struct Db {
    pub conn: Connection,
}

impl Db {
    pub fn connect_and_migrate() -> Result<Self> {
        let db_path = Path::new(DB_PATH_STR.as_str());
        fs::create_dir_all(db_path)?;
        let mut conn = Connection::open(db_path.join("db.sqlite"))?;
        MIGRATIONS.to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn connect() -> Result<Self> {
        let db_path = Path::new(DB_PATH_STR.as_str());
        let mut conn = Connection::open(db_path.join("db.sqlite"))?;
        Ok(Self { conn })
    }

    pub fn close(self) -> Result<()> {
        match self.conn.close() {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::msg("failed to close database")),
        }
    }

    pub fn get_config<T>(self, key: &String) -> std::result::Result<T, Error>
    where
        String: Into<T>,
    {
        let value = match self
            .conn
            .query_one("SELECT value FROM config WHERE key = :key", &[(":key", &key)], |r| {
                r.get::<_, String>(0)
            }) {
            Ok(value) => value,
            Err(QueryReturnedNoRows) => return Err(Error::msg("config key does not exist")),
            Err(e) => return Err(Error::from(e)),
        };
        Ok(value.into())
    }
}
