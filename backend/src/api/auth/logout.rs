use axum::{extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use color_eyre::eyre::WrapErr;

use super::session::SessionCookie;
use crate::domain::session::SessionTokenHash;
use crate::{AppState, api::support::error::AppError};

pub async fn handler(
	State(app_state): State<AppState>,
	jar: CookieJar,
) -> Result<(StatusCode, CookieJar), AppError> {
	if let Some(session_cookie) = SessionCookie::from_jar(app_state.config.auth_cookie, &jar) {
		revoke_session(&app_state.db_pool, &session_cookie.token_hash())
			.await
			.wrap_err("failed to revoke session during logout")?;
	}

	let jar = jar.remove(SessionCookie::removal());

	Ok((StatusCode::NO_CONTENT, jar))
}

async fn revoke_session(
	db_pool: &sqlx::PgPool,
	session_token_hash: &SessionTokenHash,
) -> color_eyre::Result<()> {
	sqlx::query!(
		r#"
		DELETE FROM sessions
		WHERE token_hash = $1
		"#,
		session_token_hash.as_ref(),
	)
	.execute(db_pool)
	.await?;

	Ok(())
}
