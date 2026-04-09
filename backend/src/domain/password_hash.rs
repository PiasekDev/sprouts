use argon2::{
	Argon2, PasswordHasher, PasswordVerifier,
	password_hash::{Error, PasswordHash as ParsedPasswordHash, SaltString, rand_core::OsRng},
};
use nutype::nutype;

use crate::domain::plain_password::PlainPassword;

#[nutype(derive(AsRef))]
pub struct PasswordHash(String);

impl PasswordHash {
	pub fn hash(password: &PlainPassword) -> Result<Self, Error> {
		let salt = SaltString::generate(&mut OsRng);

		Argon2::default()
			.hash_password(password.as_ref().as_bytes(), &salt)
			.map(|password_hash| Self::new(password_hash.to_string()))
	}

	pub fn verify(&self, password: &PlainPassword) -> Result<bool, Error> {
		let password_hash = ParsedPasswordHash::new(self.as_ref())?;

		match Argon2::default().verify_password(password.as_ref().as_bytes(), &password_hash) {
			Ok(()) => Ok(true),
			Err(Error::Password) => Ok(false),
			Err(error) => Err(error),
		}
	}
}
