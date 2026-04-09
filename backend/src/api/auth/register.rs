use axum::{Json, http::StatusCode};
use axum_extra::extract::WithRejection;
use serde::Deserialize;
use serde_fields::SerdeField;

use crate::api::support::error::{ApiError, RequestValidationError};
use crate::api::support::problem::ProblemField;
use crate::api::support::validation::{Validated, ValidationProblems};
use crate::domain::plain_password::PlainPassword;
use crate::domain::username::Username;

pub struct RegisterRequest {
	pub username: Username,
	pub password: PlainPassword,
}

pub async fn handler(
	WithRejection(Json(dto), _): WithRejection<Json<RegisterRequestDto>, ApiError>,
) -> Result<StatusCode, ApiError> {
	let request = RegisterRequest::try_from(dto)?;
	let _ = (
		request.username.as_ref().len(),
		request.password.as_ref().len(),
	);

	Ok(StatusCode::NOT_IMPLEMENTED)
}

#[derive(Deserialize, SerdeField)]
pub struct RegisterRequestDto {
	pub username: String,
	pub password: String,
}

impl TryFrom<RegisterRequestDto> for RegisterRequest {
	type Error = RequestValidationError;

	fn try_from(dto: RegisterRequestDto) -> Result<Self, Self::Error> {
		let RegisterRequestDto { username, password } = dto;
		let mut problems = ValidationProblems::new();

		let draft = RegisterRequestDraft {
			username: problems.field(
				RegisterRequestDtoSerdeField::Username,
				Username::try_new(username),
			),
			password: problems.field(
				RegisterRequestDtoSerdeField::Password,
				PlainPassword::try_new(password),
			),
		};

		draft.into_result(problems)
	}
}

struct RegisterRequestDraft {
	username: Validated<Username>,
	password: Validated<PlainPassword>,
}

impl RegisterRequestDraft {
	fn into_result(
		self,
		problems: ValidationProblems,
	) -> Result<RegisterRequest, RequestValidationError> {
		let Self { username, password } = self;
		let errors = problems.into_errors();

		match (username, password) {
			(Validated::Valid(username), Validated::Valid(password)) if errors.is_empty() => {
				Ok(RegisterRequest { username, password })
			}
			(_, _) if !errors.is_empty() => Err(RequestValidationError::from_errors(errors)),
			_ => Err(RequestValidationError::from_errors(vec![ProblemField::new(
				"#",
				"validation_incomplete",
				"request validation could not be completed successfully",
			)])),
		}
	}
}
