use nutype::nutype;
use thiserror::Error;

use crate::api::support::validation::ProblemSpec;

#[nutype(
	sanitize(with = sanitize_join_code),
	validate(with = validate_join_code, error = JoinCodeError),
	derive(Debug, Clone, PartialEq, Eq, AsRef, Display),
)]
pub struct JoinCode(String);

const JOIN_CODE_LENGTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JoinCodeError {
	#[error("must contain exactly {JOIN_CODE_LENGTH} characters")]
	InvalidLength,

	#[error("must contain only hexadecimal characters")]
	InvalidFormat,
}

impl ProblemSpec for JoinCodeError {
	fn code(&self) -> &str {
		match self {
			Self::InvalidLength => "invalid_length",
			Self::InvalidFormat => "invalid_format",
		}
	}
}

fn sanitize_join_code(join_code: String) -> String {
	join_code.trim().to_ascii_uppercase()
}

fn validate_join_code(join_code: &str) -> Result<(), JoinCodeError> {
	if join_code.len() != JOIN_CODE_LENGTH {
		return Err(JoinCodeError::InvalidLength);
	}

	if !join_code
		.chars()
		.all(|character| character.is_ascii_hexdigit())
	{
		return Err(JoinCodeError::InvalidFormat);
	}

	Ok(())
}
