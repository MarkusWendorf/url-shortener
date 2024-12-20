use heed::types::Str;
use heed::{Database, EnvFlags, EnvOpenOptions};
use heed::{MdbError, PutFlags};

use super::storage::{Error, Storage};

pub struct LmdbStorage {
    db: heed::Database<Str, Str>,
    env: heed::Env,
}

impl LmdbStorage {
    pub fn new() -> LmdbStorage {
        let path = std::path::Path::new("data");
        std::fs::create_dir_all(&path).unwrap();

        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(40000 * 1024 * 1024)
                .max_readers(64)
                .flags(EnvFlags::WRITE_MAP | EnvFlags::MAP_ASYNC)
                .open(path)
                .unwrap()
        };

        let mut tx = env.write_txn().unwrap();
        let db: Database<Str, Str> = env.create_database(&mut tx, None).unwrap();
        tx.commit().unwrap();

        LmdbStorage { db, env }
    }
}

impl Storage for LmdbStorage {
    fn get(&self, key: &str) -> Option<String> {
        if let Ok(tx) = self.env.read_txn()
            && let Ok(value) = self.db.get(&tx, &key)
        {
            return value.map(|v| v.to_owned());
        }

        None
    }

    fn set(&self, key: &str, value: &str) -> Result<(), Error> {
        let mut tx = self.env.write_txn().map_err(|_| Error::GenericError)?;

        let insert = self
            .db
            .put_with_flags(&mut tx, PutFlags::NO_OVERWRITE, &key, &value);

        return match insert {
            Err(err) => match err {
                heed::Error::Mdb(MdbError::KeyExist) => Err(Error::DuplicateKey),
                _ => Err(Error::GenericError),
            },
            _ => tx.commit().map_err(|_| Error::GenericError),
        };
    }

    fn key_count(&self) -> u64 {
        match self.env.read_txn() {
            Ok(tx) => self.db.len(&tx).unwrap_or_default(),
            _ => 0,
        }
    }
}
