use axum::{Router, routing::get};
use color_eyre::Result;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;
	dotenvy::dotenv().ok();
	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.with(EnvFilter::from_default_env())
		.init();

	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL should be set");
	let bind_address = env::var("BIND_ADDRESS").expect("BIND_ADDRESS should be set");

	let db_pool = PgPoolOptions::new()
		.max_connections(10)
		.connect(&database_url)
		.await?;

	sqlx::migrate!("../migrations").run(&db_pool).await?;

	let app = Router::new().route("/", get(|| async { "Hello, World!" }));

	let listener = tokio::net::TcpListener::bind(&bind_address).await?;

	tracing::info!("listening on {bind_address}");

	axum::serve(listener, app).await?;

	Ok(())
}
