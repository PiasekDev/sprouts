use nutype::nutype;
use thiserror::Error;

use crate::api::support::validation::ProblemSpec;

const MIN_PASSWORD_LENGTH: usize = 8;

#[nutype(
	validate(with = validate_plain_password, error = PlainPasswordError),
	derive(AsRef),
)]
pub struct PlainPassword(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlainPasswordError {
	#[error("must have at least {MIN_PASSWORD_LENGTH} characters")]
	TooShort,
}

impl ProblemSpec for PlainPasswordError {
	fn code(&self) -> &'static str {
		match self {
			Self::TooShort => "too_short",
		}
	}
}

fn validate_plain_password(password: &str) -> Result<(), PlainPasswordError> {
	if password.chars().count() < MIN_PASSWORD_LENGTH {
		return Err(PlainPasswordError::TooShort);
	}

	Ok(())
}
