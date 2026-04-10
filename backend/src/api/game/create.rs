use axum::{Json, extract::State, http::StatusCode};
use color_eyre::eyre::{OptionExt, WrapErr};
use uuid::Uuid;

use super::{GameResponse, fetch_game_for_user};
use crate::AppState;
use crate::api::auth::session::CurrentUser;
use crate::api::support::error::AppError;
use crate::domain::game::{BoardState, GameStatus};

pub async fn handler(
	State(app_state): State<AppState>,
	current_user: CurrentUser,
) -> Result<(StatusCode, Json<GameResponse>), AppError> {
	let board_state = BoardState::initial();
	let board_state_jsonb =
		serde_json::to_value(&board_state).wrap_err("failed to serialize initial board state")?;
	let join_code = generate_join_code();
	let game_id = sqlx::query!(
		r#"
		INSERT INTO games (status, player1_user_id, join_code, board_state_jsonb)
		VALUES ($1, $2, $3, $4)
		RETURNING id
		"#,
		GameStatus::Waiting as GameStatus,
		current_user.id,
		join_code,
		board_state_jsonb,
	)
	.fetch_one(&app_state.db_pool)
	.await
	.wrap_err("failed to create game")?
	.id;
	let game = fetch_game_for_user(&app_state.db_pool, game_id, &current_user)
		.await
		.wrap_err("failed to fetch created game")?
		.ok_or_eyre("created game could not be fetched for its creator")?;

	Ok((StatusCode::CREATED, Json(game)))
}

fn generate_join_code() -> String {
	let join_code = Uuid::now_v7().simple().to_string();
	join_code[..8].to_uppercase()
}
