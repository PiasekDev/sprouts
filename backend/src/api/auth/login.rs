use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::WithRejection;
use color_eyre::eyre::WrapErr;
use serde::Deserialize;
use serde_fields::SerdeField;
use sqlx::PgPool;
use thiserror::Error;

use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};
use crate::domain::password_hash::PasswordHash;
use crate::domain::plain_password::PlainPassword;
use crate::domain::username::Username;
use crate::{
	AppState,
	api::support::error::{AppError, RequestValidationError},
};

pub struct LoginRequest {
	pub username: Username,
	pub password: PlainPassword,
}

pub async fn handler(
	State(app_state): State<AppState>,
	WithRejection(Json(dto), _): WithRejection<Json<LoginRequestDto>, ProblemDetails>,
) -> Result<StatusCode, AppError> {
	let LoginRequest { username, password } = LoginRequest::try_from(dto)?;
	authenticate(&app_state.db_pool, &username, &password).await?;

	Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Error)]
#[error("invalid credentials")]
struct InvalidCredentials;

#[derive(Debug, Deserialize, SerdeField)]
pub struct LoginRequestDto {
	pub username: String,
	pub password: String,
}

impl TryFrom<LoginRequestDto> for LoginRequest {
	type Error = RequestValidationError;

	fn try_from(dto: LoginRequestDto) -> Result<Self, Self::Error> {
		let LoginRequestDto { username, password } = dto;
		let username = Username::try_new(username).map_err(|error| {
			RequestValidationError::for_field(LoginRequestDtoSerdeField::Username, error)
		})?;
		let password = PlainPassword::try_new(password).map_err(|error| {
			RequestValidationError::for_field(LoginRequestDtoSerdeField::Password, error)
		})?;

		Ok(Self { username, password })
	}
}

async fn authenticate(
	db_pool: &PgPool,
	username: &Username,
	password: &PlainPassword,
) -> Result<(), AppError> {
	let password_hash = sqlx::query_scalar!(
		r#"
		SELECT password_hash
		FROM users
		WHERE username = $1
		"#,
		username.as_ref(),
	)
	.fetch_optional(db_pool)
	.await
	.wrap_err("failed to fetch user credentials during login")?
	.ok_or(InvalidCredentials)?;

	let password_hash = PasswordHash::new(password_hash);
	let is_valid = password_hash
		.verify(password)
		.wrap_err("failed to verify password during login")?;

	if !is_valid {
		return Err(InvalidCredentials.into());
	}

	Ok(())
}

impl From<InvalidCredentials> for AppError {
	fn from(error: InvalidCredentials) -> Self {
		ProblemDetails::from(error).into()
	}
}

impl From<InvalidCredentials> for ProblemDetails {
	fn from(_: InvalidCredentials) -> Self {
		ProblemDetails::new(ProblemType::Custom("invalid-credentials"))
			.with_status(StatusCode::UNAUTHORIZED)
			.with_title("Invalid credentials")
			.with_detail("The provided username or password is incorrect.")
			.with_error(ProblemField::new(
				"#",
				"invalid_credentials",
				"The provided username or password is incorrect.",
			))
	}
}
