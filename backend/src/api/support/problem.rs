use axum::{
	Json,
	http::{HeaderValue, StatusCode, header},
	response::{IntoResponse, Response},
};
use serde::{Serialize, Serializer};

#[derive(Debug, Serialize)]
pub struct ProblemDetails {
	#[serde(rename = "type")]
	pub problem_type: ProblemType,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub title: Option<String>,
	#[serde(serialize_with = "serialize_status_code")]
	pub status: StatusCode,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub detail: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub errors: Vec<ProblemField>,
}

#[derive(Debug, Clone, Copy)]
pub enum ProblemType {
	Custom(&'static str),
	InvalidJson,
	ValidationError,
	InternalServerError,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProblemField {
	pub pointer: String,
	pub code: String,
	pub detail: String,
}

impl ProblemField {
	pub fn new(
		pointer: impl Into<String>,
		code: impl Into<String>,
		detail: impl Into<String>,
	) -> Self {
		Self {
			pointer: pointer.into(),
			code: code.into(),
			detail: detail.into(),
		}
	}

	pub fn for_field(
		field_name: impl AsRef<str>,
		code: impl Into<String>,
		detail: impl Into<String>,
	) -> Self {
		let pointer = format!("#/{}", field_name.as_ref());
		Self::new(pointer, code, detail)
	}
}

impl ProblemDetails {
	pub fn new(problem_type: ProblemType, status: StatusCode) -> Self {
		Self {
			problem_type,
			title: None,
			status,
			detail: None,
			errors: Vec::new(),
		}
	}

	pub fn with_title(mut self, title: impl Into<String>) -> Self {
		self.title = Some(title.into());
		self
	}

	pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
		self.detail = Some(detail.into());
		self
	}

	pub fn with_error(mut self, error: ProblemField) -> Self {
		self.errors.push(error);
		self
	}

	pub fn with_errors(mut self, errors: Vec<ProblemField>) -> Self {
		self.errors = errors;
		self
	}
}

impl IntoResponse for ProblemDetails {
	fn into_response(self) -> Response {
		let status = self.status;

		let mut response = (status, Json(self)).into_response();
		response.headers_mut().insert(
			header::CONTENT_TYPE,
			HeaderValue::from_static("application/problem+json"),
		);
		response
	}
}

fn serialize_status_code<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	serializer.serialize_u16(status.as_u16())
}

impl Serialize for ProblemType {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(self.as_str())
	}
}

impl ProblemType {
	fn as_str(self) -> &'static str {
		match self {
			Self::Custom(problem_type) => problem_type,
			Self::InvalidJson => "urn:sprouts:problem:invalid-json",
			Self::ValidationError => "urn:sprouts:problem:validation-error",
			Self::InternalServerError => "urn:sprouts:problem:internal-server-error",
		}
	}
}
