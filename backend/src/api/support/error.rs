use super::problem::{ProblemDetails, ProblemField, ProblemType};
use axum::{
	Json,
	extract::rejection::JsonRejection,
	http::{HeaderValue, StatusCode, header},
	response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
	#[error("request body could not be parsed")]
	InvalidJson {
		status: StatusCode,
		title: String,
		detail: String,
	},
	#[error(transparent)]
	InvalidRequest(#[from] RequestValidationError),
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		let (status, problem) = match self {
			Self::InvalidJson {
				status,
				title,
				detail,
			} => (
				status,
				ProblemDetails {
					problem_type: ProblemType::InvalidJson,
					title,
					status,
					detail,
					errors: Vec::new(),
				},
			),
			Self::InvalidRequest(validation_error) => (
				StatusCode::UNPROCESSABLE_ENTITY,
				ProblemDetails {
					problem_type: ProblemType::ValidationError,
					title: "Request validation failed".to_owned(),
					status: StatusCode::UNPROCESSABLE_ENTITY,
					detail: "One or more fields are invalid.".to_owned(),
					errors: validation_error.errors,
				},
			),
		};

		let mut response = (status, Json(problem)).into_response();
		response.headers_mut().insert(
			header::CONTENT_TYPE,
			HeaderValue::from_static("application/problem+json"),
		);
		response
	}
}

impl From<JsonRejection> for ApiError {
	fn from(rejection: JsonRejection) -> Self {
		let status = rejection.status();
		let detail = rejection.body_text();
		let title = match rejection {
			JsonRejection::JsonDataError(_) => {
				"JSON body does not match the expected request shape"
			}
			JsonRejection::JsonSyntaxError(_) => "Invalid JSON syntax",
			JsonRejection::MissingJsonContentType(_) => "Unsupported media type",
			JsonRejection::BytesRejection(_) => "Request body could not be read",
			_ => "Request body could not be read",
		};

		Self::InvalidJson {
			status,
			title: title.to_owned(),
			detail,
		}
	}
}

#[derive(Debug, Error)]
#[error("request validation failed")]
pub struct RequestValidationError {
	pub errors: Vec<ProblemField>,
}

impl RequestValidationError {
	pub fn from_errors(errors: Vec<ProblemField>) -> Self {
		Self { errors }
	}
}
