use axum::{
	extract::FromRequestParts,
	http::{StatusCode, request::Parts},
};
use axum_extra::extract::{
	CookieJar,
	cookie::{Cookie, SameSite},
};
use color_eyre::eyre::WrapErr;
use thiserror::Error;
use uuid::Uuid;

use crate::AppState;
use crate::api::support::error::AppError;
use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};
use crate::config::AuthCookieConfig;
use crate::domain::session::{SessionToken, SessionTokenHash};
use crate::domain::username::Username;

pub const SESSION_COOKIE_NAME: &str = "sprouts_session";

pub struct SessionCookie {
	config: AuthCookieConfig,
	token: SessionToken,
}

pub struct CurrentUser {
	pub id: Uuid,
	pub username: Username,
}

#[derive(Debug, Error)]
#[error("authentication required")]
struct AuthenticationRequired;

impl SessionCookie {
	pub fn new(config: AuthCookieConfig, token: SessionToken) -> Self {
		Self { config, token }
	}

	pub fn from_jar(config: AuthCookieConfig, jar: &CookieJar) -> Option<Self> {
		jar.get(SESSION_COOKIE_NAME)
			.map(|cookie| Self::new(config, SessionToken::new(cookie.value().to_owned())))
	}

	pub fn token_hash(&self) -> SessionTokenHash {
		self.token.hash()
	}

	pub fn into_cookie(self) -> Cookie<'static> {
		let mut cookie = Cookie::build((SESSION_COOKIE_NAME, self.token.as_ref().to_owned()))
			.path("/")
			.http_only(true)
			.same_site(SameSite::Lax)
			.secure(self.config.secure)
			.build();
		cookie.set_max_age(self.config.max_age);
		cookie
	}

	pub fn removal() -> Cookie<'static> {
		Cookie::build(SESSION_COOKIE_NAME).path("/").build()
	}
}

impl FromRequestParts<AppState> for CurrentUser {
	type Rejection = AppError;

	async fn from_request_parts(
		parts: &mut Parts,
		state: &AppState,
	) -> Result<Self, Self::Rejection> {
		let jar = CookieJar::from_headers(&parts.headers);
		let session_cookie = SessionCookie::from_jar(state.config.auth_cookie, &jar)
			.ok_or(AuthenticationRequired)?;
		let session_token_hash = session_cookie.token_hash();
		let user = sqlx::query!(
			r#"
			SELECT users.id, users.username
			FROM sessions
			INNER JOIN users ON users.id = sessions.user_id
			WHERE sessions.token_hash = $1
				AND sessions.expires_at > NOW()
			"#,
			session_token_hash.as_ref(),
		)
		.fetch_optional(&state.db_pool)
		.await
		.wrap_err("failed to resolve authenticated user from session")?
		.ok_or(AuthenticationRequired)?;

		// SAFETY: The username in the database is guaranteed to be valid as it is validated during user registration.
		let username = unsafe { Username::new_unchecked(user.username) };

		Ok(Self {
			id: user.id,
			username,
		})
	}
}

impl From<AuthenticationRequired> for AppError {
	fn from(error: AuthenticationRequired) -> Self {
		ProblemDetails::from(error).into()
	}
}

impl From<AuthenticationRequired> for ProblemDetails {
	fn from(_: AuthenticationRequired) -> Self {
		ProblemDetails::new(ProblemType::Custom("authentication-required"))
			.with_status(StatusCode::UNAUTHORIZED)
			.with_title("Authentication required")
			.with_detail("A valid authenticated session is required.")
			.with_error(ProblemField::new(
				"#",
				"authentication_required",
				"A valid authenticated session is required.",
			))
	}
}
