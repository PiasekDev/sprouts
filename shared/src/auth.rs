use serde::{Deserialize, Serialize};
use serde_fields::SerdeField;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
	pub id: Uuid,
	pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, SerdeField)]
pub struct LoginRequest {
	pub username: String,
	pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, SerdeField)]
pub struct RegisterRequest {
	pub username: String,
	pub password: String,
}
