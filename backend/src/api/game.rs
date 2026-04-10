use axum::{
	Router,
	routing::{get, post},
};
use color_eyre::eyre::WrapErr;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppState;
use crate::api::auth::session::CurrentUser;
use crate::domain::game::{BoardState, GameStatus};

pub mod create;
pub mod get;
pub mod join;

pub fn router() -> Router<AppState> {
	Router::new()
		.route("/", post(create::handler))
		.route("/{id}", get(get::handler))
		.route("/{id}/join", post(join::handler))
}

#[derive(Debug, Serialize)]
pub struct GamePlayerResponse {
	pub id: Uuid,
	pub username: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GameResponse {
	Waiting {
		id: Uuid,
		join_code: String,
		player1: GamePlayerResponse,
		board_state: BoardState,
	},
	Active {
		id: Uuid,
		join_code: String,
		player1: GamePlayerResponse,
		player2: GamePlayerResponse,
		current_turn_user_id: Uuid,
		board_state: BoardState,
	},
	Finished {
		id: Uuid,
		join_code: String,
		player1: GamePlayerResponse,
		player2: GamePlayerResponse,
		winner_user_id: Uuid,
		board_state: BoardState,
	},
}

struct GameRow {
	id: Uuid,
	status: GameStatus,
	join_code: String,
	player1_user_id: Uuid,
	player1_username: String,
	player2_user_id: Option<Uuid>,
	player2_username: Option<String>,
	current_turn_user_id: Option<Uuid>,
	winner_user_id: Option<Uuid>,
	board_state_jsonb: sqlx::types::Json<BoardState>,
}

async fn fetch_game_for_user(
	db_pool: &PgPool,
	game_id: Uuid,
	current_user: &CurrentUser,
) -> color_eyre::Result<Option<GameResponse>> {
	let game = sqlx::query_as!(
		GameRow,
		r#"
		SELECT
			games.id,
			games.status AS "status!: GameStatus",
			games.join_code,
			games.player1_user_id,
			player1.username AS player1_username,
			games.player2_user_id,
			player2.username AS "player2_username?",
			games.current_turn_user_id,
			games.winner_user_id,
			games.board_state_jsonb AS "board_state_jsonb!: sqlx::types::Json<BoardState>"
		FROM games
		INNER JOIN users AS player1 ON player1.id = games.player1_user_id
		LEFT JOIN users AS player2 ON player2.id = games.player2_user_id
		WHERE games.id = $1
			AND ($2 = games.player1_user_id OR $2 = games.player2_user_id)
		"#,
		game_id,
		current_user.id,
	)
	.fetch_optional(db_pool)
	.await
	.wrap_err("failed to fetch game for authenticated user")?;

	Ok(game.map(GameResponse::from))
}

impl From<GameRow> for GameResponse {
	fn from(game: GameRow) -> Self {
		let GameRow {
			id,
			status,
			join_code,
			player1_user_id,
			player1_username,
			player2_user_id,
			player2_username,
			current_turn_user_id,
			winner_user_id,
			board_state_jsonb,
		} = game;
		let player1 = GamePlayerResponse {
			id: player1_user_id,
			username: player1_username,
		};
		let board_state = board_state_jsonb.0;

		match status {
			GameStatus::Waiting => Self::Waiting {
				id,
				join_code,
				player1,
				board_state,
			},
			GameStatus::Active => Self::Active {
				id,
				join_code,
				player1,
				player2: GamePlayerResponse {
					id: player2_user_id.expect(
						"invariant violated: active game did not contain a second player id",
					),
					username: player2_username.expect(
						"invariant violated: active game did not contain a second player username",
					),
				},
				current_turn_user_id: current_turn_user_id
					.expect("invariant violated: active game did not contain current turn user id"),
				board_state,
			},
			GameStatus::Finished => Self::Finished {
				id,
				join_code,
				player1,
				player2: GamePlayerResponse {
					id: player2_user_id.expect(
						"invariant violated: finished game did not contain a second player id",
					),
					username: player2_username.expect(
						"invariant violated: finished game did not contain a second player username",
					),
				},
				winner_user_id: winner_user_id
					.expect("invariant violated: finished game did not contain winner user id"),
				board_state,
			},
		}
	}
}
