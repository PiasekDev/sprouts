use axum::{
	extract::FromRequestParts,
	http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
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

pub struct SessionCookie(SessionToken);

pub struct CurrentUser {
	pub id: Uuid,
	pub username: Username,
}

#[derive(Debug, Error)]
#[error("authentication required")]
struct AuthenticationRequired;

impl SessionCookie {
	pub fn new(session_token: SessionToken) -> Self {
		Self(session_token)
	}

	pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
		headers
			.get_all(header::COOKIE)
			.iter()
			.filter_map(|header_value| header_value.to_str().ok())
			.find_map(Self::from_cookie_header)
	}

	fn from_cookie_header(cookie_header: &str) -> Option<Self> {
		cookie_header
			.split(';')
			.filter_map(|cookie| cookie.trim().split_once('='))
			.find_map(|(name, value)| {
				(name == SESSION_COOKIE_NAME)
					.then(|| Self::new(SessionToken::new(value.to_owned())))
			})
	}

	pub fn token_hash(&self) -> SessionTokenHash {
		self.0.hash()
	}

	pub fn into_header_value(self, config: AuthCookieConfig) -> HeaderValue {
		let secure = if config.secure { "; Secure" } else { "" };
		let max_age_seconds = config.max_age.whole_seconds();
		let value = format!(
			"{SESSION_COOKIE_NAME}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_seconds}{secure}",
			self.0.as_ref(),
		);

		HeaderValue::from_str(&value).expect("session cookie should be a valid header value")
	}
}

impl FromRequestParts<AppState> for CurrentUser {
	type Rejection = AppError;

	async fn from_request_parts(
		parts: &mut Parts,
		state: &AppState,
	) -> Result<Self, Self::Rejection> {
		let session_cookie =
			SessionCookie::from_headers(&parts.headers).ok_or(AuthenticationRequired)?;
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
		let username = Username::try_new(user.username)
			.wrap_err("invariant violated: resolved db session contained an invalid username")?;

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
