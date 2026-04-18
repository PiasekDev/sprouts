use backend::config::{AppConfig, AppEnvironment};
use backend::{AppState, app};
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;
	dotenvy::dotenv().ok();
	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.with(EnvFilter::from_default_env())
		.init();

	let database_url = env::var("DATABASE_URL").wrap_err("DATABASE_URL should be set")?;
	let bind_address = env::var("BIND_ADDRESS").wrap_err("BIND_ADDRESS should be set")?;
	let app_environment = AppEnvironment::from_env();
	let config = Arc::new(AppConfig::from(app_environment));

	let db_pool = PgPoolOptions::new()
		.max_connections(10)
		.connect(&database_url)
		.await?;

	sqlx::migrate!("../migrations").run(&db_pool).await?;

	let state = AppState { db_pool, config };
	let app = app(state);

	let listener = tokio::net::TcpListener::bind(&bind_address).await?;

	tracing::info!("listening on {bind_address}");

	axum::serve(listener, app).await?;

	Ok(())
}
