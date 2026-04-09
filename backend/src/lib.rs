pub mod api {
	use axum::Router;

	pub mod auth;

	pub mod support {
		pub mod error;
		pub mod problem;
		pub mod validation;
	}

	pub fn router() -> Router {
		Router::new().nest("/auth", auth::router())
	}
}

pub mod domain {
	pub mod plain_password;
	pub mod username;
}
