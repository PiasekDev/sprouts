use std::borrow::Cow;
use std::error::Error as StdError;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("invariant violated: {message}")]
pub struct InvariantViolationError {
	message: Cow<'static, str>,
	source: Box<dyn StdError + Send + Sync + 'static>,
}

impl InvariantViolationError {
	pub fn new(
		message: impl Into<Cow<'static, str>>,
		source: impl StdError + Send + Sync + 'static,
	) -> Self {
		Self {
			message: message.into(),
			source: Box::new(source),
		}
	}
}

#[derive(Debug, Error)]
#[error("expected a value to be present")]
pub struct MissingInvariantValue;

pub trait InvariantExt<T> {
	fn invariant<M>(self, message: M) -> Result<T, InvariantViolationError>
	where
		M: Into<Cow<'static, str>>;
}

impl<T> InvariantExt<T> for Option<T> {
	fn invariant<M>(self, message: M) -> Result<T, InvariantViolationError>
	where
		M: Into<Cow<'static, str>>,
	{
		self.ok_or_else(|| InvariantViolationError::new(message, MissingInvariantValue))
	}
}

impl<T, E> InvariantExt<T> for Result<T, E>
where
	E: StdError + Send + Sync + 'static,
{
	fn invariant<M>(self, message: M) -> Result<T, InvariantViolationError>
	where
		M: Into<Cow<'static, str>>,
	{
		self.map_err(|source| InvariantViolationError::new(message, source))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use color_eyre::eyre::Report;
	use std::{error::Error, io};

	#[test]
	fn option_invariant_wraps_missing_value() {
		let error = Option::<()>::None
			.invariant("value should be present")
			.unwrap_err();

		assert_eq!(
			error.to_string(),
			"invariant violated: value should be present"
		);
		assert_eq!(
			error.source().unwrap().to_string(),
			"expected a value to be present"
		);
	}

	#[test]
	fn result_invariant_wraps_original_error() {
		let error = Err::<(), _>(io::Error::other("boom"))
			.invariant("operation should have succeeded")
			.unwrap_err();

		assert_eq!(
			error.to_string(),
			"invariant violated: operation should have succeeded"
		);
		assert_eq!(error.source().unwrap().to_string(), "boom");
	}

	#[test]
	fn invariant_error_as_report_preserves_context_chain() {
		let report = Report::new(
			Option::<()>::None
				.invariant("value should be present")
				.unwrap_err(),
		);
		let chain = report.chain().map(ToString::to_string).collect::<Vec<_>>();

		assert_eq!(chain[0], "invariant violated: value should be present");
		assert_eq!(chain[1], "expected a value to be present");
	}
}
