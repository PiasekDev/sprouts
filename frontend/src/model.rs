pub use shared::auth::{LoginRequest, RegisterRequest, UserResponse as User};
pub use shared::game::{GameResponse, MoveRequest, NewSpot, Spot};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct ProblemDetails {
	#[serde(rename = "type")]
	pub problem_type: String,
	pub status: u16,
	pub title: Option<String>,
	pub detail: Option<String>,
	pub errors: Vec<ProblemField>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProblemField {
	pub pointer: String,
	pub code: String,
	pub detail: String,
}

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct ApiProblem {
	pub message: String,
	pub details: Option<ProblemDetails>,
}

#[derive(Debug, Error)]
#[error("unexpected response body for status {status}")]
pub struct UnexpectedResponse {
	pub status: u16,
	pub body: String,
	pub source: serde_json::Error,
}

#[derive(Debug, Clone)]
pub struct DraftMove {
	pub start_spot: Option<Spot>,
	pub points: Vec<[f64; 2]>,
	pub end_spot: Option<Spot>,
	pub new_spot: Option<[f64; 2]>,
}

impl DraftMove {
	pub fn empty() -> Self {
		Self {
			start_spot: None,
			points: Vec::new(),
			end_spot: None,
			new_spot: None,
		}
	}

	pub fn can_submit(&self) -> bool {
		self.start_spot.is_some() && self.end_spot.is_some() && self.new_spot.is_some()
	}

	pub fn to_request(&self) -> Option<MoveRequest> {
		let new_spot = self.new_spot?;

		Some(MoveRequest {
			start_spot_id: self.start_spot.as_ref()?.id,
			end_spot_id: self.end_spot.as_ref()?.id,
			points: self.points.clone(),
			new_spot: NewSpot {
				x: new_spot[0],
				y: new_spot[1],
			},
		})
	}
}
