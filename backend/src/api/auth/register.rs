use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::WithRejection;
use color_eyre::eyre::WrapErr;
use serde::Deserialize;
use serde_fields::SerdeField;
use sqlx::PgPool;
use thiserror::Error;

use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};
use crate::api::support::validation::{Validated, ValidationProblems};
use crate::domain::password_hash::PasswordHash;
use crate::domain::plain_password::PlainPassword;
use crate::domain::username::Username;
use crate::{
	AppState,
	api::support::error::{AppError, RequestValidationError},
};

pub struct RegisterRequest {
	pub username: Username,
	pub password: PlainPassword,
}

pub async fn handler(
	State(app_state): State<AppState>,
	WithRejection(Json(dto), _): WithRejection<Json<RegisterRequestDto>, ProblemDetails>,
) -> Result<StatusCode, AppError> {
	let RegisterRequest { username, password } = RegisterRequest::try_from(dto)?;
	let username = ensure_username_available(&app_state.db_pool, username).await?;
	let password_hash = PasswordHash::hash(&password)
		.wrap_err("failed to hash user password during registration")?;
	create_user(&app_state.db_pool, &username, &password_hash)
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
			RegisterError::UsernameTaken => ProblemDetails::new(
				ProblemType::Custom("urn:sprouts:problem:username-taken"),
				StatusCode::CONFLICT,
			)
			.with_title("Username is already taken")
			.with_detail("A user with this username already exists.")
			.with_error(ProblemField::for_field(
				"username",
				"already_exists",
				"is already taken",
			)),
		}
	}
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
			_ => Err(RequestValidationError::from_errors(vec![
				ProblemField::new(
					"#",
					"validation_incomplete",
					"request validation could not be completed successfully",
				),
			])),
		}
	}
}
