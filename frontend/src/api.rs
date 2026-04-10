use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use thiserror::Error;
use web_sys::RequestCredentials;

use crate::model::{
	ApiProblem, GameResponse, LoginRequest, MoveRequest, ProblemDetails, RegisterRequest,
	UnexpectedResponse, User,
};

#[derive(Debug, Error)]
pub enum ApiError {
	#[error(transparent)]
	Problem(#[from] ApiProblem),

	#[error(transparent)]
	Network(#[from] gloo_net::Error),

	#[error(transparent)]
	Unexpected(#[from] UnexpectedResponse),
}

pub async fn me() -> Result<Option<User>, ApiError> {
	let response = Request::get("/api/auth/me")
		.credentials(RequestCredentials::Include)
		.send()
		.await?;

	match response.status() {
		200 => Ok(Some(response.into_payload().await?)),
		401 => Ok(None),
		_ => Err(ApiProblem::from_response(response).await?.into()),
	}
}

pub async fn register(payload: &RegisterRequest) -> Result<(), ApiError> {
	Request::post("/api/auth/register")
		.credentials(RequestCredentials::Include)
		.json(payload)?
		.send()
		.await?
		.into_empty()
		.await
}

pub async fn login(payload: &LoginRequest) -> Result<(), ApiError> {
	Request::post("/api/auth/login")
		.credentials(RequestCredentials::Include)
		.json(payload)?
		.send()
		.await?
		.into_empty()
		.await
}

pub async fn logout() -> Result<(), ApiError> {
	Request::post("/api/auth/logout")
		.credentials(RequestCredentials::Include)
		.send()
		.await?
		.into_empty()
		.await
}

pub async fn create_game() -> Result<GameResponse, ApiError> {
	Request::post("/api/game")
		.credentials(RequestCredentials::Include)
		.send()
		.await?
		.into_payload()
		.await
}

pub async fn join_game(game_id: &str) -> Result<GameResponse, ApiError> {
	Request::post("/api/game/join")
		.credentials(RequestCredentials::Include)
		.json(&shared::game::JoinGameRequest {
			join_code: game_id.to_string(),
		})?
		.send()
		.await?
		.into_payload()
		.await
}

pub async fn get_game(game_id: &str) -> Result<GameResponse, ApiError> {
	Request::get(&format!("/api/game/{game_id}"))
		.credentials(RequestCredentials::Include)
		.send()
		.await?
		.into_payload()
		.await
}

pub async fn submit_move(game_id: &str, payload: &MoveRequest) -> Result<GameResponse, ApiError> {
	Request::post(&format!("/api/game/{game_id}/move"))
		.credentials(RequestCredentials::Include)
		.json(payload)?
		.send()
		.await?
		.into_payload()
		.await
}

impl ApiProblem {
	async fn from_response(response: gloo_net::http::Response) -> Result<Self, UnexpectedResponse> {
		let status = response.status();
		let text = response.text().await.unwrap_or_default();
		let details =
			serde_json::from_str::<ProblemDetails>(&text).map_err(|source| UnexpectedResponse {
				status,
				body: text.clone(),
				source,
			})?;
		let message = details
			.detail
			.as_ref()
			.or_else(|| details.errors.first().map(|field| &field.detail))
			.or(details.title.as_ref())
			.cloned()
			.unwrap_or_else(|| {
				let code = details
					.errors
					.first()
					.map(|field| format!("{} at {}", field.code, field.pointer))
					.unwrap_or_else(|| details.problem_type.clone());
				format!("{code} ({})", details.status)
			});

		Ok(Self {
			message,
			details: Some(details),
		})
	}
}

trait ResponseExt {
	async fn into_payload<T: DeserializeOwned>(self) -> Result<T, ApiError>;
	async fn into_empty(self) -> Result<(), ApiError>;
}

impl ResponseExt for gloo_net::http::Response {
	async fn into_payload<T: DeserializeOwned>(self) -> Result<T, ApiError> {
		if self.ok() {
			Ok(self.json().await?)
		} else {
			Err(ApiProblem::from_response(self).await?.into())
		}
	}

	async fn into_empty(self) -> Result<(), ApiError> {
		if self.ok() {
			Ok(())
		} else {
			Err(ApiProblem::from_response(self).await?.into())
		}
	}
}
