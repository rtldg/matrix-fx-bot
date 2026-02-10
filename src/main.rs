use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use matrix_sdk::RoomState;
use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::api::client::filter::FilterDefinition;
use matrix_sdk::ruma::events::room::message::MessageType;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;

#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None, flatten_help = true, disable_help_subcommand = true)]
struct Args {
	database_path: PathBuf,
	#[command(subcommand)]
	command: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
	Login {
		homeserver: String,
		username: String,
		password: String,
	},
	Run,
}

static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

/// We read/write this sucker as JSON to a table in the sqlite database.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct FxSessionData {
	homeserver: String,
	user_session: MatrixSession,
}

impl FxSessionData {
	fn persist(&self) -> anyhow::Result<()> {
		let fx_session_data = serde_json::to_string(self)?;

		let conn = rusqlite::Connection::open(&ARGS.database_path)?;
		conn.execute(
			"CREATE TABLE IF NOT EXISTS FxSessionData (id INTEGER PRIMARY KEY, settings TEXT NOT NULL);",
			(),
		)?;

		conn.execute(
			"
			INSERT INTO FxSessionData (id, settings)
			VALUES (1, ?1)
			ON CONFLICT (id)
			DO UPDATE SET settings = ?1;
			",
			(&fx_session_data,),
		)?;

		conn.close().unwrap();
		Ok(())
	}

	fn load() -> anyhow::Result<FxSessionData> {
		let conn = rusqlite::Connection::open(&ARGS.database_path)?;
		let settings = conn.query_one("SELECT settings FROM FxSessionData;", (), |r| {
			Ok(r.get_ref(0)?.as_str()?.to_owned())
		})?;
		let settings: FxSessionData = serde_json::from_str(&settings)?;
		Ok(settings)
	}
}

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
	reqwest::ClientBuilder::new()
		.connect_timeout(Duration::from_secs(10))
		.read_timeout(Duration::from_secs(120))
		.timeout(Duration::from_secs(140))
		.user_agent(format!(
			"{}/{} ({})",
			env!("CARGO_PKG_NAME"),
			env!("CARGO_PKG_VERSION"),
			env!("CARGO_PKG_REPOSITORY")
		))
		.build()
		.unwrap()
});

fn main() -> anyhow::Result<()> {
	unsafe {
		std::env::set_var("RUST_BACKTRACE", "full");
	}

	tokio::runtime::Runtime::new()?.block_on(async { tokio::spawn(async_main()).await? })
}

async fn async_main() -> anyhow::Result<()> {
	match &ARGS.command {
		Commands::Login {
			homeserver,
			username,
			password,
		} => login(&homeserver, &username, &password).await,
		Commands::Run => run().await,
	}
}

async fn login(homeserver: &str, username: &str, password: &str) -> anyhow::Result<()> {
	tokio::fs::remove_file(&ARGS.database_path).await?; // Die, fool.

	println!("Connecting to {homeserver}");
	let matrix_client = matrix_sdk::Client::builder()
		.homeserver_url(&homeserver)
		.sqlite_store(&ARGS.database_path, None)
		.build()
		.await?;
	let matrix_auth: matrix_sdk::authentication::matrix::MatrixAuth = matrix_client.matrix_auth();

	println!("Attempting to login to @{username}:{homeserver}");
	let _response = matrix_auth
		.login_username(&username, &password)
		.initial_device_display_name(&format!("bot {}", rand::rng().next_u32()))
		.await?;

	let matrix_session = matrix_auth.session().context("matrix_auth.session()")?;
	FxSessionData {
		homeserver: homeserver.to_owned(),
		user_session: matrix_session,
	}
	.persist()?;

	Ok(())
}

async fn run() -> anyhow::Result<()> {
	let fx_session_data = FxSessionData::load()?;
	let matrix_client = matrix_sdk::Client::builder()
		.homeserver_url(&fx_session_data.homeserver)
		.sqlite_store(&ARGS.database_path, None)
		.build()
		.await?;

	matrix_client.restore_session(fx_session_data.user_session.clone()).await?;

	println!("Syncing...");

	let filter = FilterDefinition::with_lazy_loading();
	let mut sync_settings = SyncSettings::default().filter(filter.into());

	{
		let response = matrix_client.sync_once(sync_settings.clone()).await?;
		sync_settings = sync_settings.token(response.next_batch.clone());
	}

	matrix_client.add_event_handler(on_room_message);

	matrix_client.sync(sync_settings).await?;

	Ok(())
}

async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: matrix_sdk::Room) {
	if room.state() != RoomState::Joined {
		return;
	}

	let MessageType::Text(text) = &event.content.msgtype else {
		return;
	};

	/*
	- find twitter/x/fxtwitter/etc urls in body
	- fetch fxtwitter shit
	- upload images & embed body
	- post message in reply
	*/
}
