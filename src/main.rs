// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 rtldg <rtldg@protonmail.com>
// Copyright ????-???? matrix-rust-sdk contributors

mod types;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use matrix_sdk::RoomState;
use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::attachment::BaseImageInfo;
use matrix_sdk::attachment::BaseVideoInfo;
use matrix_sdk::attachment::Thumbnail;
use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::OwnedUserId;
use matrix_sdk::ruma::api::client::filter::FilterDefinition;
use matrix_sdk::ruma::events::relation::RelationType;
use matrix_sdk::ruma::events::room::member::StrippedRoomMemberEvent;
use matrix_sdk::ruma::events::room::message::MessageType;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use rand::Rng;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use signals_but_a_little_nicer::SignalInfo;

const TARGETS: &[&str] = &[
	"cunnyx.com",
	"fixupx.com",
	"fixvx.com",
	"fxtwitter.com",
	"girlcockx.com",
	"hitlerx.com",
	"nitter.net",
	"nitter.poast.org",
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
	#[arg(long)]
	proxy: Option<Url>,
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
static MY_USER_ID: OnceLock<OwnedUserId> = OnceLock::new();
static SHOULD_DIE: OnceLock<()> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FxSessionData {
	homeserver: String,
	user_session: MatrixSession,
}

impl FxSessionData {
	// We don't have to persist() after login because sync_with_callback()/sync_once() will store tokens for us in matrix_sdk::ClientBuilder::sqlite_store() files
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
	let mut builder = reqwest::ClientBuilder::new()
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
		.user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36");

	if let Some(proxy) = &ARGS.proxy {
		builder = builder.proxy(reqwest::Proxy::all(proxy.clone()).unwrap());
	}

	builder.build().unwrap()
});

fn main() -> anyhow::Result<()> {
	unsafe {
		std::env::set_var("RUST_BACKTRACE", "full");
	}

	let signal_recv = signals_but_a_little_nicer::get_or_init_receiver().context("failed to setup signal handler")?;

	tokio::runtime::Runtime::new()?.block_on(async { tokio::spawn(async_main(signal_recv)).await? })
}

async fn async_main(signal_recv: signals_but_a_little_nicer::SignalReceiver) -> anyhow::Result<()> {
	match &ARGS.command {
		Commands::Login {
			homeserver,
			username,
			password,
			login_token,
		} => login(&homeserver, &username, &password, &login_token).await,
		Commands::Run => run(signal_recv).await,
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

async fn run(mut signal_recv: signals_but_a_little_nicer::SignalReceiver) -> anyhow::Result<()> {
	tokio::spawn(async move {
		while let Ok(signal) = signal_recv.recv().await {
			match signal {
				SignalInfo::Int | SignalInfo::Quit | SignalInfo::Term => {
					println!("\nReceived {signal:?}.  Exiting (slowly)");
					break;
				},
				_ => continue,
			}
		}
		let _ = SHOULD_DIE.set(());
	});

	while let Err(e) = run_session_once().await {
		println!("{e:?}");
		println!("Restarting in 10s");
		tokio::time::sleep(Duration::from_secs(10)).await;
	}
	Ok(())
}

async fn run_session_once() -> anyhow::Result<()> {
	let fx_session_data = FxSessionData::load()?;
	let mut matrix_client_builder = matrix_sdk::Client::builder()
		.server_name_or_homeserver_url(&fx_session_data.homeserver)
		.sqlite_store(&ARGS.database_dir, None);

	if let Some(proxy) = &ARGS.proxy {
		matrix_client_builder = matrix_client_builder.proxy(proxy);
	}

	let matrix_client = matrix_client_builder.build().await?;

	matrix_client.restore_session(fx_session_data.user_session.clone()).await?;

	println!("Syncing...");

	let filter = FilterDefinition::with_lazy_loading();
	let mut sync_settings = SyncSettings::default().filter(filter.into());

	{
		let response = matrix_client.sync_once(sync_settings.clone()).await?;
		sync_settings = sync_settings.token(response.next_batch.clone());
	}

	/*
	// TODO: doesn't quite work...
	let device = matrix_client.encryption().get_own_device().await?.unwrap();
	if !device.is_verified() {
		device.verify().await?;
	}
	*/

	MY_USER_ID.get_or_init(|| matrix_client.user_id().unwrap().to_owned());

	matrix_client.add_event_handler(on_room_message);
	matrix_client.add_event_handler(on_stripped_state_member);

	println!("max_upload_size = {:?}", matrix_client.load_or_fetch_max_upload_size().await?);

	matrix_client
		.sync_with_callback(sync_settings, |_| async {
			if SHOULD_DIE.get().is_some() {
				matrix_sdk::LoopCtrl::Break
			} else {
				matrix_sdk::LoopCtrl::Continue
			}
		})
		.await?;

	Ok(())
}

// copied from https://github.com/matrix-org/matrix-rust-sdk/blob/4257649933dfe61f44f35efd2de5726c2f24aac7/examples/autojoin/src/main.rs#L8
async fn on_stripped_state_member(room_member: StrippedRoomMemberEvent, client: matrix_sdk::Client, room: matrix_sdk::Room) {
	if room_member.state_key != client.user_id().unwrap() {
		return;
	}

	let Some(name) = &room.name() else {
		return;
	};

	if name != "fx test" || true {
		return;
	}

	tokio::spawn(async move {
		println!("Autojoining room {} (invite from {})", room.room_id(), room_member.sender);
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

async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: matrix_sdk::Room, _client: matrix_sdk::Client) {
	if room.state() != RoomState::Joined {
		return;
	}

	if event.sender.eq(MY_USER_ID.wait()) {
		return;
	}

	if !room.encryption_state().is_encrypted() {
		// [fx]twitter embeds mostly work in unencrypted rooms so this isn't necessary.
		return;
	}

	if let Some(relates_to) = &event.content.relates_to
		&& let Some(rel_type) = relates_to.rel_type()
		&& rel_type == RelationType::Replacement
	{
		// skip edited messages
		return;
	}

	let MessageType::Text(text) = &event.content.msgtype else {
		return;
	};

	if text.body.trim() == "!status" {
		println!("IKIRU");
		let content = RoomMessageEventContent::text_plain("IKIRU");
		let _ = room.send(content).await;
		return;
	}

	if text.body == "!die" {
		let _ = SHOULD_DIE.set(());
		println!("!die");
		return;
	}

	// TODO: pixiv/phixiv

	let mut links: Vec<_> = linkify::LinkFinder::new()
		.links(&text.body)
		.filter_map(|l| Url::from_str(l.as_str()).ok())
		.filter(|l| l.scheme() == "https" && l.has_host())
		.filter(|l| TARGETS.contains(&l.host_str().unwrap().to_ascii_lowercase().as_str()))
		.filter(|l| l.path().contains("/status/"))
		.collect();

	if links.is_empty() {
		return;
	}

	let typer = tokio::spawn({
		let room = room.clone();
		async move {
			loop {
				let _ = room.typing_notice(true).await;
				tokio::time::sleep(Duration::from_secs_f32(1.0)).await;
			}
		}
	});

	links.dedup();

	for link in links {
		println!("found {link}");
		if let Err(e) = post_tweet(&event, &room, link).await {
			println!("  error: {e:?}");
		}
	}

	// keep typing for a tad longer...
	tokio::spawn(async move {
		tokio::time::sleep(Duration::from_secs(1)).await;
		typer.abort();
		let _ = typer.await;
	});
}

#[derive(Debug)]
struct UploadInfo {
	url: Url,
	width: Option<u32>,
	height: Option<u32>,
	duration: Option<f64>,
	content_type: mime::Mime,
	filename: String,
	thumbnail_url: Option<Url>,
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

	// TODO: Nice HTML embeds for tweet.
	let textmsg = RoomMessageEventContent::text_plain(format!(
		"{} (@{})\n{}\n💬{} ♻️{} ❤️{} 👁️{}\n{}",
		tweet.author.name,
		tweet.author.screen_name,
		tweet.text,
		tweet.replies,
		tweet.retweets,
		tweet.likes,
		tweet.views,
		tweet.created_timestamp.strftime("%F %T")
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
		if video.r#type == "gif" {
			url.set_path(&url.path().replace(".mp4", ".gif"));
		}
		let filename = url.path_segments().unwrap().last().unwrap().to_string();
		if video.r#type == "gif" {
			url.set_host(Some("gif.fxtwitter.com")).unwrap();
			to_upload.push(UploadInfo {
				url: url,
				width: None,
				height: None,
				duration: None,
				content_type: mime::IMAGE_GIF,
				filename: filename,
				thumbnail_url: None,
			});
		} else {
			to_upload.push(UploadInfo {
				url: url,
				width: Some(video.width),
				height: Some(video.height),
				duration: Some(video.duration),
				content_type: video.format.parse().unwrap(),
				filename: filename,
				thumbnail_url: Some(video.thumbnail_url.clone()),
			});
		}
	} else if let Some(mosaic) = media.mosaic {
		to_upload.push(UploadInfo {
			url: mosaic.formats.webp.clone(),
			width: None,
			height: None,
			duration: None,
			content_type: "image/webp".parse().unwrap(),
			filename: format!("{}_mosaic.webp", tweet.id),
			thumbnail_url: None,
		});
	} else if let Some(photos) = media.photos {
		let photo = &photos[0];
		let filename = photo.url.path_segments().unwrap().last().unwrap();
		let content_type = if filename.ends_with(".jpg") {
			mime::IMAGE_JPEG
		} else {
			format!("image/{}", filename.split('.').last().unwrap()).parse().unwrap()
		};
		to_upload.push(UploadInfo {
			url: photo.url.clone(),
			width: Some(photo.width),
			height: Some(photo.height),
			duration: None,
			content_type: content_type,
			filename: filename.to_string(),
			thumbnail_url: None,
		});
	}

	for upload_info in to_upload {
		println!("  Trying to fetch and upload {}", upload_info.url);
		let data = HTTP
			.get(upload_info.url.clone())
			.send()
			.await
			.context("Failed to GET main file")?
			.error_for_status()
			.context("Bad status")?
			.bytes()
			.await
			.context("Failed to read entire body of main file")?;

		let mut attachment_config = AttachmentConfig::new();
		if let Some(thumbnail_url) = upload_info.thumbnail_url {
			println!("  Fetching thumbnail {thumbnail_url}");
			let thumbnail_data = HTTP
				.get(thumbnail_url)
				.send()
				.await
				.context("Failed to GET thumbnail")?
				.error_for_status()
				.context("Bad status")?
				.bytes()
				.await
				.context("Failed to read entire body of thumbnail")?;
			let thumbnail_size = thumbnail_data.len();
			let thumbnail = Thumbnail {
				data: thumbnail_data.to_vec(),
				content_type: mime::IMAGE_JPEG, // should always be truee
				height: 200u32.into(),          // just fucking lie TODO
				width: 200u32.into(),           // just fucking lie TODO
				size: (thumbnail_size as u32).into(),
			};
			attachment_config = attachment_config.thumbnail(Some(thumbnail));
		}
		if upload_info.duration.is_some() || upload_info.width.is_some() || upload_info.height.is_some() {
			if upload_info.filename.ends_with(".mp4") {
				attachment_config.info = Some(matrix_sdk::attachment::AttachmentInfo::Video(BaseVideoInfo {
					duration: upload_info.duration.map(Duration::from_secs_f64),
					height: upload_info.height.map(|a| a.into()),
					width: upload_info.width.map(|a| a.into()),
					size: Some((data.len() as u32).into()),
					blurhash: None,
				}))
			} else {
				attachment_config.info = Some(matrix_sdk::attachment::AttachmentInfo::Image(BaseImageInfo {
					height: upload_info.height.map(|a| a.into()),
					width: upload_info.width.map(|a| a.into()),
					size: Some((data.len() as u32).into()),
					is_animated: None, // idc
					blurhash: None,
				}))
			}
		}

		let _ = room
			.send_attachment(
				upload_info.filename,
				&upload_info.content_type,
				data.into(),
				attachment_config,
			)
			.await
			.context("Failed to send attachment")?;
		println!("  uploaded {}", upload_info.url);
	}

	Ok(())
}
