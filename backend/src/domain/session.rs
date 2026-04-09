use argon2::password_hash::rand_core::{OsRng, RngCore};
use nutype::nutype;
use sha2::{Digest, Sha256};

#[nutype(derive(Debug, AsRef))]
pub struct SessionToken(String);

#[nutype(derive(Debug, AsRef))]
pub struct SessionTokenHash(String);

const SESSION_TOKEN_SIZE_BYTES: usize = 32;

impl SessionToken {
	pub fn generate() -> Self {
		let mut token = [0; SESSION_TOKEN_SIZE_BYTES];
		OsRng.fill_bytes(&mut token);

		Self::new(hex::encode(token))
	}

	pub fn hash(&self) -> SessionTokenHash {
		let digest = Sha256::digest(self.as_ref().as_bytes());
		SessionTokenHash::new(hex::encode(digest))
	}
}
