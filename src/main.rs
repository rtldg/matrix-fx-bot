mod types;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use matrix_sdk::RoomState;
use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::api::client::filter::FilterDefinition;
use matrix_sdk::ruma::events::room::member::StrippedRoomMemberEvent;
use matrix_sdk::ruma::events::room::message::MessageType;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use rand::Rng;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;

const TARGETS: &[&str] = &[
	"cunnyx.com",
	"fixupx.com",
	"fixvx.com",
	"fxtwitter.com",
	"hitlerx.com",
	"twitter.com",
	"twittpr.com",
	"vxtwitter.com",
	"x.com",
	"xcancel.com",
	"xfixup.com",
];

#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None, flatten_help = true, disable_help_subcommand = true)]
struct Args {
	#[arg(long)]
	database_dir: PathBuf,
	#[command(subcommand)]
	command: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
	Login {
		#[arg(long)]
		homeserver: String,
		#[arg(long)]
		username: Option<String>,
		#[arg(long)]
		password: Option<String>,
		#[arg(long)]
		login_token: Option<String>,
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

		let conn = rusqlite::Connection::open(&ARGS.database_dir.join("fxsession.sqlite3"))?;
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
		let conn = rusqlite::Connection::open(&ARGS.database_dir.join("fxsession.sqlite3"))?;
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
		/*
		.user_agent(format!(
			"{}/{} ({})",
			env!("CARGO_PKG_NAME"),
			env!("CARGO_PKG_VERSION"),
			env!("CARGO_PKG_REPOSITORY")
		))
		*/
		.user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36")
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
			login_token,
		} => login(&homeserver, &username, &password, &login_token).await,
		Commands::Run => run().await,
	}
}

async fn login(
	homeserver: &str,
	username: &Option<String>,
	password: &Option<String>,
	login_token: &Option<String>,
) -> anyhow::Result<()> {
	let _ = tokio::fs::remove_dir_all(&ARGS.database_dir).await; // Die, fool.
	tokio::fs::create_dir_all(&ARGS.database_dir).await?; // Live, fool.

	println!("Connecting to {homeserver}");
	let matrix_client = matrix_sdk::Client::builder()
		.server_name_or_homeserver_url(&homeserver)
		.sqlite_store(&ARGS.database_dir, None)
		.build()
		.await?;
	let matrix_auth: matrix_sdk::authentication::matrix::MatrixAuth = matrix_client.matrix_auth();

	let login_types = matrix_auth.get_login_types().await?;

	if let Some(username) = username
		&& let Some(password) = password
	{
		println!("Attempting to login to @{username}:{homeserver}");
		let _response = matrix_auth
			.login_username(&username, &password)
			.initial_device_display_name(&format!("Element {}", rand::rng().next_u32()))
			.await?;
	} else if let Some(login_token) = login_token {
		println!("Attempting to login with token {login_token}");
		let _response = matrix_auth
			.login_token(&login_token)
			.initial_device_display_name(&format!("Element {}", rand::rng().next_u32()))
			.await?;
	} else {
		println!("{:?}", login_types);
		anyhow::bail!("missing username/password or login_token combo!");
	}

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
		.server_name_or_homeserver_url(&fx_session_data.homeserver)
		.sqlite_store(&ARGS.database_dir, None)
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
	matrix_client.add_event_handler(on_stripped_state_member);

	println!("max_upload_size = {:?}", matrix_client.load_or_fetch_max_upload_size().await?);

	matrix_client.sync(sync_settings).await?;

	// TODO: setup a nice way to exit so your sqlite dbs close cleanly

	Ok(())
}

async fn on_stripped_state_member(room_member: StrippedRoomMemberEvent, client: matrix_sdk::Client, room: matrix_sdk::Room) {
	if room_member.state_key != client.user_id().unwrap() {
		return;
	}

	if let Some(name) = &room.name() {
		if name != "fx test" {
			return;
		}
	}

	tokio::spawn(async move {
		println!("Autojoining room {}", room.room_id());
		let mut delay = 2;

		while let Err(err) = room.join().await {
			// retry autojoin due to synapse sending invites, before the
			// invited user can join for more information see
			// https://github.com/matrix-org/synapse/issues/4345
			eprintln!("Failed to join room {} ({err:?}), retrying in {delay}s", room.room_id());

			tokio::time::sleep(Duration::from_secs(delay)).await;
			delay *= 2;

			if delay > 3600 {
				eprintln!("Can't join room {} ({err:?})", room.room_id());
				break;
			}
		}
		println!("Successfully joined room {}", room.room_id());
	});
}

async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: matrix_sdk::Room) {
	if room.state() != RoomState::Joined {
		return;
	}

	let MessageType::Text(text) = &event.content.msgtype else {
		return;
	};

	println!("{:?}", event);

	let links: Vec<_> = linkify::LinkFinder::new()
		.links(&text.body)
		.filter_map(|l| Url::from_str(l.as_str()).ok())
		.filter(|l| l.scheme() == "https" && l.has_host())
		.filter(|l| TARGETS.contains(&l.host_str().unwrap().to_ascii_lowercase().as_str()))
		.filter(|l| l.path().contains("/status/"))
		.collect();

	for link in links {
		println!("found {link}");
		if let Err(e) = post_tweet(&event, &room, link).await {
			println!("  error: {e:?}");
		}
	}
}

async fn post_tweet(_event: &OriginalSyncRoomMessageEvent, room: &matrix_sdk::Room, mut link: Url) -> anyhow::Result<()> {
	link.set_host(Some("api.fxtwitter.com")).unwrap();
	let response = HTTP
		.get(link)
		.send()
		.await
		.context("Failed to fetch api.fxtwitter.com results")?;
	let response = response
		.json::<types::FxApiResponse>()
		.await
		.context("failed to parse as JSON into FxApiResponse")?;
	let tweet = response.tweet.context("response.tweet was None")?;

	let textmsg = RoomMessageEventContent::text_plain(format!(
		"{} (@{})\n{}\n💬{} ♻️{} ❤️{} 👁️{}\n{}",
		tweet.author.name,
		tweet.author.screen_name,
		tweet.text,
		tweet.replies,
		tweet.retweets,
		tweet.likes,
		tweet.views,
		tweet.created_at
	));
	let _ = room.send(textmsg).await.context("Failed to send tweet info")?;
	println!("  Sent textmsg");

	let Some(media) = tweet.media else {
		println!("  No media");
		return Ok(());
	};

	// keeping this as a vec in case I ever want to upload all images instead of the mosaic
	let mut to_upload = vec![];

	if let Some(videos) = media.videos {
		let video = &videos[0];
		let mut url = video.url.clone();
		url.set_path(&url.path().replace(".mp4", ".gif"));
		let filename = url.path_segments().unwrap().last().unwrap().to_string();
		if video.r#type == "gif" {
			url.set_host(Some("gif.fxtwitter.com")).unwrap();
			to_upload.push((mime::IMAGE_GIF, filename, url));
		} else {
			to_upload.push((video.format.parse().unwrap(), filename, url));
		}
	} else if let Some(mosaic) = media.mosaic {
		to_upload.push((
			"image/webp".parse().unwrap(),
			format!("{}_mosaic.webp", tweet.id),
			mosaic.formats.webp.clone(),
		));
	} else if let Some(photos) = media.photos {
		let filename = photos[0].url.path_segments().unwrap().last().unwrap();
		let content_type = if filename.ends_with(".jpg") {
			mime::IMAGE_JPEG
		} else {
			format!("image/{}", filename.split('.').last().unwrap()).parse().unwrap()
		};
		to_upload.push((content_type, filename.to_string(), photos[0].url.clone()));
	}

	for (content_type, filename, url) in to_upload {
		println!("  Trying to fetch {url}");
		let response = HTTP.get(url.clone()).send().await.context("Failed to GET")?;
		let response = response.error_for_status().context("Bad status or something")?;
		let data = response.bytes().await.context("Failed to read entire body")?;

		let _ = room
			.send_attachment(filename, &content_type, data.into(), AttachmentConfig::new())
			.await
			.context("Failed to send attachment")?;
	}

	Ok(())
}
