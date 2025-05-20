use secrecy::{ExposeSecret, Secret};
use ring::aead::{Aad, AES_256_GCM, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credential {
    pub id: Uuid,
    pub name: String,
    pub token: String,
}

fn get_encryption_key() -> LessSafeKey {
    let key_b64 = std::env::var("SECRET_ENCRYPTION_KEY")
        .expect("SECRET_ENCRYPTION_KEY not set");
    let key_bytes = general_purpose::STANDARD
        .decode(&key_b64)
        .expect("Failed to decode base64 key");
    let unbound = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .expect("Invalid encryption key");
    LessSafeKey::new(unbound)
}

pub async fn store_credential(
    pool: &PgPool,
    name: &str,
    token: Secret<String>,
) -> Result<Uuid, sqlx::Error> {
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes).unwrap();
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let mut plaintext = token.expose_secret().as_bytes().to_vec();
    let key = get_encryption_key();
    let tag = key
        .seal_in_place_separate_tag(nonce, Aad::empty(), &mut plaintext)
        .expect("Encryption failed");

    let encrypted = [plaintext, tag.as_ref().to_vec()].concat();
    let encrypted_b64 = general_purpose::STANDARD.encode(&encrypted);
    let nonce_b64 = general_purpose::STANDARD.encode(&nonce_bytes);

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO credentials (id, name, token, nonce) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)              // bind by value
    .bind(name)
    .bind(encrypted_b64)
    .bind(nonce_b64)
    .execute(pool)
    .await?;

    Ok(id)
}

pub async fn get_credential(pool: &PgPool, id: Uuid) -> Result<Credential, sqlx::Error> {
    let row = sqlx::query(
        "SELECT name, token, nonce FROM credentials WHERE id = $1",
    )
    .bind(id)              // bind by value
    .fetch_one(pool)
    .await?;

    let name: String = row.get("name");
    let encrypted_b64: String = row.get("token");
    let nonce_b64: String = row.get("nonce");

    let encrypted = general_purpose::STANDARD
        .decode(&encrypted_b64)
        .expect("Failed to decode token");
    let nonce_bytes: [u8; 12] = general_purpose::STANDARD
        .decode(&nonce_b64)
        .expect("Failed to decode nonce")
        .as_slice()
        .try_into()
        .expect("Invalid nonce length");
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let mut buffer = encrypted.clone();
    let key = get_encryption_key();
    let decrypted = key
        .open_in_place(nonce, Aad::empty(), &mut buffer)
        .expect("Decryption failed");

    let token = String::from_utf8(decrypted.to_vec())
        .expect("Decrypted token was not valid UTF-8");

    Ok(Credential { id, name, token })
}
