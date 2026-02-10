use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

use clap::Parser;
use matrix_sdk::authentication::matrix::MatrixSession;
use serde::Deserialize;
use serde::Serialize;

#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

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

/// We read/write this sucker as JSON to a table in the sqlite database.
#[derive(Debug, Serialize, Deserialize)]
struct FxSessionData {
	homeserver: String,
	user_session: MatrixSession,
	sync_token: Option<String>,
}

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

fn main() -> anyhow::Result<()> {
	unsafe {
		std::env::set_var("RUST_BACKTRACE", "full");
	}

	let args = Args::parse();

	tokio::runtime::Runtime::new()?.block_on(async { tokio::spawn(async_main(args)).await? })
}

async fn async_main(args: Args) -> anyhow::Result<()> {
	match args.command {
		Commands::Login {
			homeserver,
			username,
			password,
		} => login(args.database_path, homeserver, username, password).await,
		Commands::Run => run(args.database_path).await,
	}
}

async fn login(database_path: PathBuf, homeserver: String, username: String, password: String) -> anyhow::Result<()> {
	tokio::fs::remove_file(database_path).await?; // Die, fool.
	Ok(())
}

async fn run(database_path: PathBuf) -> anyhow::Result<()> {
	Ok(())
}
