use axum::http::StatusCode;
use serde::{Serialize, Serializer};

#[derive(Debug, Serialize)]
pub struct ProblemDetails {
	#[serde(rename = "type")]
	pub problem_type: ProblemType,
	pub title: String,
	#[serde(serialize_with = "serialize_status_code")]
	pub status: StatusCode,
	pub detail: String,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub errors: Vec<ProblemField>,
}

#[derive(Debug, Clone, Copy)]
pub enum ProblemType {
	InvalidJson,
	ValidationError,
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
			Self::InvalidJson => "urn:sprouts:problem:invalid-json",
			Self::ValidationError => "urn:sprouts:problem:validation-error",
		}
	}
}
