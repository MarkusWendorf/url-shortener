use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::entities::User;

pub fn create_user(connection: &mut Connection, email: &str, password: &str) -> Result<User, rusqlite::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        _ => return Err(rusqlite::Error::QueryReturnedNoRows),
    };

    connection.query_row(
        "INSERT INTO users (email, pw_hash) VALUES (?1, ?2) RETURNING id, email",
        [email, &hash],
        |row| {
            let id: u64 = row.get(0)?;
            let email: String = row.get(1)?;

            Ok(User { id, email })
        },
    )
}

pub fn verify_password(connection: &mut Connection, email: &str, password: &str) -> Result<User, rusqlite::Error> {
    connection.query_row(
        "SELECT pw_hash, id, email FROM users WHERE email = ?1",
        [email],
        |row| {
            let hash: String = row.get(0)?;

            let argon = Argon2::default();
            if let Ok(parsed_hash) = PasswordHash::new(&hash)
                && argon.verify_password(password.as_bytes(), &parsed_hash).is_ok()
            {
                let id: u64 = row.get(1)?;
                let email: String = row.get(2)?;

                return Ok(User { email, id });
            }

            Err(rusqlite::Error::QueryReturnedNoRows)
        },
    )
}

pub fn create_session(connection: &mut Connection, user_id: u64) -> Result<(String, OffsetDateTime), rusqlite::Error> {
    let session_id = Uuid::now_v7().to_string();
    let expires_at = OffsetDateTime::now_utc() + Duration::days(1);

    connection
        .execute(
            "INSERT INTO sessions(session_id, user_id, expires_at) VALUES (?1, ?2, ?3)",
            params![&session_id, &user_id, expires_at.unix_timestamp()],
        )
        .map(|_| (session_id, expires_at))
}
