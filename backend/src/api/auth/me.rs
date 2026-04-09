use axum::Json;
use serde::Serialize;
use uuid::Uuid;

use super::session::CurrentUser;

#[derive(Serialize)]
pub struct MeResponse {
	pub id: Uuid,
	pub username: String,
}

pub async fn handler(user: CurrentUser) -> Json<MeResponse> {
	Json(MeResponse {
		id: user.id,
		username: user.username.as_ref().to_owned(),
	})
}
