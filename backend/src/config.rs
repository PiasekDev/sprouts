use std::env;
use time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppEnvironment {
	Development,
	Production,
}

pub struct AppConfig {
	pub auth_cookie: AuthCookieConfig,
}

#[derive(Clone, Copy)]
pub struct AuthCookieConfig {
	pub secure: bool,
	pub max_age: Duration,
}

impl AppEnvironment {
	pub fn from_env() -> Self {
		match env::var("APP_ENV")
			.map(|value| value.to_ascii_lowercase())
			.as_deref()
		{
			Ok("production") => Self::Production,
			_ => Self::Development,
		}
	}
}

impl From<AppEnvironment> for AppConfig {
	fn from(app_environment: AppEnvironment) -> Self {
		Self {
			auth_cookie: AuthCookieConfig::from(app_environment),
		}
	}
}

impl From<AppEnvironment> for AuthCookieConfig {
	fn from(app_environment: AppEnvironment) -> Self {
		Self {
			secure: matches!(app_environment, AppEnvironment::Production),
			max_age: Duration::days(30),
		}
	}
}
