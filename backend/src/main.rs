use backend::config::{AppConfig, AppEnvironment};
use backend::{AppState, app};
use color_eyre::Result;
use color_eyre::eyre::{WrapErr, eyre};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection};
use std::env;
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

enum BackendCommand {
	Serve,
	Migrate,
	PrintHelp,
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;
	dotenvy::dotenv().ok();
	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.with(EnvFilter::from_default_env())
		.init();

	match BackendCommand::from_args()? {
		BackendCommand::Serve => serve().await,
		BackendCommand::Migrate => migrate().await,
		BackendCommand::PrintHelp => {
			println!(
				r#"Usage: backend [serve|migrate]

Commands:
  serve    Start the HTTP API server (default)
  migrate  Run database migrations and exit"#
			);
			Ok(())
		}
	}
}

impl BackendCommand {
	fn from_args() -> Result<Self> {
		match env::args().nth(1).as_deref() {
			None | Some("serve") => Ok(Self::Serve),
			Some("migrate") => Ok(Self::Migrate),
			Some("-h" | "--help" | "help") => Ok(Self::PrintHelp),
			Some(command) => Err(eyre!(
				"unknown backend command `{command}`; expected `serve` or `migrate`"
			)),
		}
	}
}

async fn serve() -> Result<()> {
	let database_url = env::var("DATABASE_URL").wrap_err("DATABASE_URL should be set")?;
	let bind_address = env::var("BIND_ADDRESS").wrap_err("BIND_ADDRESS should be set")?;
	let database_max_connections = database_max_connections_from_env()?;
	let app_environment = AppEnvironment::from_env();
	let config = Arc::new(AppConfig::from(app_environment));
	let db_pool = PgPoolOptions::new()
		.max_connections(database_max_connections.get())
		.connect(&database_url)
		.await
		.wrap_err("failed to connect to database")?;

	let state = AppState { db_pool, config };
	let app = app(state);

	let listener = tokio::net::TcpListener::bind(&bind_address).await?;

	tracing::info!("listening on {bind_address}");

	axum::serve(listener, app).await?;

	Ok(())
}

fn database_max_connections_from_env() -> Result<NonZeroU32> {
	const DEFAULT_DATABASE_MAX_CONNECTIONS: NonZeroU32 =
		NonZeroU32::new(10).expect("default database connection limit should be non-zero");

	match env::var("DATABASE_MAX_CONNECTIONS") {
		Ok(value) => value
			.parse()
			.wrap_err("DATABASE_MAX_CONNECTIONS should be a positive integer"),
		Err(env::VarError::NotPresent) => Ok(DEFAULT_DATABASE_MAX_CONNECTIONS),
		Err(error) => Err(error).wrap_err("failed to read DATABASE_MAX_CONNECTIONS"),
	}
}

async fn migrate() -> Result<()> {
	let database_url = env::var("DATABASE_URL").wrap_err("DATABASE_URL should be set")?;
	let mut db_connection = PgConnection::connect(&database_url)
		.await
		.wrap_err("failed to connect to database for migrations")?;

	sqlx::migrate!("../migrations")
		.run_direct(&mut db_connection)
		.await
		.wrap_err("failed to run database migrations")?;

	tracing::info!("database migrations completed");

	Ok(())
}
