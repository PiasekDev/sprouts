use super::problem::{ProblemDetails, ProblemField, ProblemType};
use axum::{
	extract::rejection::JsonRejection,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
	#[error("api problem")]
	Api(ProblemDetails),

	#[error(transparent)]
	Unexpected(#[from] color_eyre::Report),
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		match self {
			Self::Api(problem) => problem.into_response(),
			Self::Unexpected(error) => {
				tracing::error!(?error, "request failed unexpectedly");

				ProblemDetails::new(
					ProblemType::InternalServerError,
					StatusCode::INTERNAL_SERVER_ERROR,
				)
				.with_title("Internal server error")
				.with_detail("The server could not complete the request.")
				.into_response()
			}
		}
	}
}

impl From<JsonRejection> for ProblemDetails {
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

		ProblemDetails::new(ProblemType::InvalidJson, status)
			.with_title(title)
			.with_detail(detail)
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

impl From<RequestValidationError> for AppError {
	fn from(error: RequestValidationError) -> Self {
		ProblemDetails::from(error).into()
	}
}

impl From<ProblemDetails> for AppError {
	fn from(problem: ProblemDetails) -> Self {
		Self::Api(problem)
	}
}

impl From<RequestValidationError> for ProblemDetails {
	fn from(error: RequestValidationError) -> Self {
		ProblemDetails::new(
			ProblemType::ValidationError,
			StatusCode::UNPROCESSABLE_ENTITY,
		)
		.with_title("Request validation failed")
		.with_detail("One or more fields are invalid.")
		.with_errors(error.errors)
	}
}
