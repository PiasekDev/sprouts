use nutype::nutype;
use thiserror::Error;

use crate::api::support::validation::ProblemSpec;

#[nutype(
	new_unchecked,
	sanitize(trim, lowercase),
	validate(with = validate_username, error = UsernameError),
	derive(Debug, Clone, PartialEq, Eq, AsRef, Display),
)]
pub struct Username(String);

const MIN_USERNAME_LENGTH: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UsernameError {
	#[error("must have at least {MIN_USERNAME_LENGTH} characters")]
	TooShort,
}

impl ProblemSpec for UsernameError {
	fn code(&self) -> &'static str {
		match self {
			Self::TooShort => "too_short",
		}
	}
}

fn validate_username(username: &str) -> Result<(), UsernameError> {
	if username.chars().count() < MIN_USERNAME_LENGTH {
		return Err(UsernameError::TooShort);
	}

	Ok(())
}
