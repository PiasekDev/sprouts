use axum::Json;
use shared::auth::UserResponse;

use super::session::CurrentUser;

pub async fn handler(user: CurrentUser) -> Json<UserResponse> {
	Json(UserResponse {
		id: user.id,
		username: user.username.as_ref().to_owned(),
	})
}
