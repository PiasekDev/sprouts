use argon2::{
	Argon2, PasswordHasher,
	password_hash::{Error, SaltString, rand_core::OsRng},
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
}
