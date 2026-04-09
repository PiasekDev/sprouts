use super::problem::ProblemField;

pub(crate) trait ProblemSpec: std::error::Error {
	fn code(&self) -> &str;

	fn detail(&self) -> String {
		self.to_string()
	}
}

pub(crate) struct ValidationProblems {
	errors: Vec<ProblemField>,
}

impl ValidationProblems {
	pub(crate) fn new() -> Self {
		Self { errors: Vec::new() }
	}

	pub(crate) fn field<T, E>(
		&mut self,
		field_name: impl AsRef<str>,
		result: Result<T, E>,
	) -> Validated<T>
	where
		E: ProblemSpec,
	{
		match result {
			Ok(value) => Validated::Valid(value),
			Err(error) => {
				self.errors.push(ProblemField::for_field(
					field_name,
					error.code(),
					error.detail(),
				));
				Validated::Invalid
			}
		}
	}

	pub(crate) fn into_errors(self) -> Vec<ProblemField> {
		self.errors
	}
}

pub(crate) enum Validated<T> {
	Valid(T),
	Invalid,
}
