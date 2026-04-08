use axum::{
	Json,
	http::StatusCode,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
	pub username: String,
	pub password: String,
}

pub async fn handler(Json(request): Json<RegisterRequest>) -> StatusCode {
	let _ = (request.username.len(), request.password.len());

	StatusCode::NOT_IMPLEMENTED
}
