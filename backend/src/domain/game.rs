use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "game_status", rename_all = "snake_case")]
pub enum GameStatus {
	Waiting,
	Active,
	Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardState {
	pub spots: Vec<Spot>,
	pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spot {
	pub id: i32,
	pub x: f64,
	pub y: f64,
	pub degree: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
	pub id: i32,
	pub start_spot_id: i32,
	pub end_spot_id: i32,
	pub points: Vec<[f64; 2]>,
	pub new_spot_id: i32,
}

impl BoardState {
	pub fn initial() -> Self {
		Self {
			spots: vec![
				Spot {
					id: 1,
					x: 100.0,
					y: 100.0,
					degree: 0,
				},
				Spot {
					id: 2,
					x: 300.0,
					y: 100.0,
					degree: 0,
				},
			],
			edges: Vec::new(),
		}
	}
}
