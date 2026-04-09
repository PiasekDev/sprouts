pub(crate) trait ProblemSpec: std::error::Error {
	fn code(&self) -> &str;

	fn detail(&self) -> String {
		self.to_string()
	}
}
