use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::WithRejection;
use color_eyre::eyre::WrapErr;
use shared::auth::{RegisterRequest, RegisterRequestSerdeField};
use sqlx::PgPool;
use thiserror::Error;

use crate::api::support::error::{AppError, RequestValidationError};
use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};
use crate::domain::password_hash::PasswordHash;
use crate::domain::plain_password::PlainPassword;
use crate::domain::username::Username;

pub struct RegisterCredentials {
	pub username: Username,
	pub password: PlainPassword,
}

pub async fn handler(
	State(db_pool): State<PgPool>,
	WithRejection(Json(dto), _): WithRejection<Json<RegisterRequest>, ProblemDetails>,
) -> Result<StatusCode, AppError> {
	let RegisterCredentials { username, password } = RegisterCredentials::try_from(dto)?;
	let username = ensure_username_available(&db_pool, username).await?;
	let password_hash = PasswordHash::hash(&password)
		.wrap_err("failed to hash user password during registration")?;
	create_user(&db_pool, &username, &password_hash)
		.await
		.wrap_err("failed to create user during registration")?;

	Ok(StatusCode::CREATED)
}

#[derive(Debug, Error)]
enum RegisterError {
	#[error("username is already taken")]
	UsernameTaken,
}

async fn ensure_username_available(
	db_pool: &PgPool,
	username: Username,
) -> Result<Username, AppError> {
	let username_exists = sqlx::query_scalar!(
		r#"
		SELECT EXISTS (
			SELECT 1
			FROM users
			WHERE username = $1
		) AS "exists!"
		"#,
		username.as_ref(),
	)
	.fetch_one(db_pool)
	.await
	.wrap_err("failed to check username availability during registration")?;

	if username_exists {
		return Err(RegisterError::UsernameTaken.into());
	}

	Ok(username)
}

async fn create_user(
	db_pool: &PgPool,
	username: &Username,
	password_hash: &PasswordHash,
) -> color_eyre::Result<()> {
	sqlx::query!(
		r#"
		INSERT INTO users (username, password_hash)
		VALUES ($1, $2)
		"#,
		username.as_ref(),
		password_hash.as_ref(),
	)
	.execute(db_pool)
	.await?;

	Ok(())
}

impl From<RegisterError> for AppError {
	fn from(error: RegisterError) -> Self {
		ProblemDetails::from(error).into()
	}
}

impl From<RegisterError> for ProblemDetails {
	fn from(error: RegisterError) -> Self {
		match error {
			RegisterError::UsernameTaken => {
				ProblemDetails::new(ProblemType::Custom("username-taken"))
					.with_status(StatusCode::CONFLICT)
					.with_title("Username is already taken")
					.with_detail("A user with this username already exists.")
					.with_error(ProblemField::for_field(
						"username",
						"already_exists",
						"is already taken",
					))
			}
		}
	}
}

impl TryFrom<RegisterRequest> for RegisterCredentials {
	type Error = RequestValidationError;

	fn try_from(dto: RegisterRequest) -> Result<Self, Self::Error> {
		let RegisterRequest { username, password } = dto;
		let username = Username::try_new(username).map_err(|error| {
			RequestValidationError::for_field(RegisterRequestSerdeField::Username, error)
		})?;
		let password = PlainPassword::try_new(password).map_err(|error| {
			RequestValidationError::for_field(RegisterRequestSerdeField::Password, error)
		})?;

		Ok(Self { username, password })
	}
}
