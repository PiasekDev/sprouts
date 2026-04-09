use axum::{
	Json,
	body::Body,
	extract::State,
	http::{StatusCode, header},
	response::Response,
};
use axum_extra::extract::WithRejection;
use color_eyre::eyre::WrapErr;
use serde::Deserialize;
use serde_fields::SerdeField;
use sqlx::PgPool;
use thiserror::Error;
use time::Duration;
use uuid::Uuid;

use super::cookie::SessionCookie;
use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};
use crate::domain::password_hash::PasswordHash;
use crate::domain::plain_password::PlainPassword;
use crate::domain::session::{SessionToken, SessionTokenHash};
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
) -> Result<Response, AppError> {
	let LoginRequest { username, password } = LoginRequest::try_from(dto)?;
	let user_id = authenticate(&app_state.db_pool, &username, &password).await?;
	let session_token = SessionToken::generate();
	create_session(
		&app_state.db_pool,
		user_id,
		&session_token.hash(),
		app_state.config.auth_cookie.max_age,
	)
	.await
	.wrap_err("failed to create session during login")?;

	let session_cookie_header =
		SessionCookie::new(app_state.config.auth_cookie, &session_token).into_header_value();
	Ok(Response::builder()
		.status(StatusCode::NO_CONTENT)
		.header(header::SET_COOKIE, session_cookie_header)
		.body(Body::empty())
		.expect("login response should be a valid response"))
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
) -> Result<Uuid, AppError> {
	let user = sqlx::query!(
		r#"
		SELECT id, password_hash
		FROM users
		WHERE username = $1
		"#,
		username.as_ref(),
	)
	.fetch_optional(db_pool)
	.await
	.wrap_err("failed to fetch user credentials during login")?
	.ok_or(InvalidCredentials)?;

	let password_hash = PasswordHash::new(user.password_hash);
	let is_valid = password_hash
		.verify(password)
		.wrap_err("failed to verify password during login")?;

	if !is_valid {
		return Err(InvalidCredentials.into());
	}

	Ok(user.id)
}

async fn create_session(
	db_pool: &PgPool,
	user_id: Uuid,
	session_token_hash: &SessionTokenHash,
	max_age: Duration,
) -> color_eyre::Result<()> {
	sqlx::query!(
		r#"
		INSERT INTO sessions (user_id, token_hash, expires_at)
		VALUES ($1, $2, NOW() + ($3 * INTERVAL '1 second'))
		"#,
		user_id,
		session_token_hash.as_ref(),
		max_age.as_seconds_f64(),
	)
	.execute(db_pool)
	.await?;

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
