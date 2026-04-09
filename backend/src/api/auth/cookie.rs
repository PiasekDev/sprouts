use axum::http::HeaderValue;

use crate::config::AuthCookieConfig;
use crate::domain::session::SessionToken;

pub const SESSION_COOKIE_NAME: &str = "sprouts_session";

pub struct SessionCookie(HeaderValue);

impl SessionCookie {
	pub fn new(config: AuthCookieConfig, session_token: &SessionToken) -> Self {
		let secure = if config.secure { "; Secure" } else { "" };
		let max_age_seconds = config.max_age.whole_seconds();
		let value = format!(
			"{SESSION_COOKIE_NAME}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_seconds}{secure}",
			session_token.as_ref(),
		);

		Self(HeaderValue::from_str(&value).expect("session cookie should be a valid header value"))
	}

	pub fn into_header_value(self) -> HeaderValue {
		self.0
	}
}
