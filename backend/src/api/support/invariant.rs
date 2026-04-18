use std::borrow::Cow;

use color_eyre::eyre::{self, WrapErr};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("invariant violated: {message}")]
pub struct InvariantViolation {
	message: Cow<'static, str>,
}

impl InvariantViolation {
	pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
		Self {
			message: message.into(),
		}
	}
}

#[derive(Debug, Error)]
#[error("expected a value to be present")]
pub struct MissingInvariantValue;

pub trait InvariantExt<T> {
	fn invariant<M>(self, message: M) -> eyre::Result<T>
	where
		M: Into<Cow<'static, str>>;
}

impl<T> InvariantExt<T> for Option<T> {
	fn invariant<M>(self, message: M) -> eyre::Result<T>
	where
		M: Into<Cow<'static, str>>,
	{
		self.ok_or_else(|| eyre::Report::new(MissingInvariantValue))
			.wrap_err(InvariantViolation::new(message))
	}
}

impl<T, E> InvariantExt<T> for Result<T, E>
where
	E: Into<eyre::Report>,
{
	fn invariant<M>(self, message: M) -> eyre::Result<T>
	where
		M: Into<Cow<'static, str>>,
	{
		self.map_err(Into::into)
			.wrap_err(InvariantViolation::new(message))
	}
}

#[cfg(test)]
mod tests {
	use std::io;

	use super::InvariantExt;

	#[test]
	fn option_invariant_wraps_missing_value() {
		let error = Option::<()>::None
			.invariant("value should be present")
			.unwrap_err();
		let chain = error.chain().map(ToString::to_string).collect::<Vec<_>>();

		assert_eq!(chain[0], "invariant violated: value should be present");
		assert_eq!(chain[1], "expected a value to be present");
	}

	#[test]
	fn result_invariant_wraps_original_error() {
		let error = Err::<(), _>(io::Error::other("boom"))
			.invariant("operation should have succeeded")
			.unwrap_err();
		let chain = error.chain().map(ToString::to_string).collect::<Vec<_>>();

		assert_eq!(
			chain[0],
			"invariant violated: operation should have succeeded"
		);
		assert_eq!(chain[1], "boom");
	}
}
