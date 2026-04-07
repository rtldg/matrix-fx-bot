// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 rtldg <rtldg@protonmail.com>
// Copyright ????-???? matrix-rust-sdk contributors

mod types;
mod verification;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use itertools::Itertools;
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
	#[arg(long, short)]
	invite_pattern_to_accept: Option<String>,
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
			.initial_device_display_name(&format!("Element {}", rand::rng().next_u32() & 255))
			.await?;
	} else if let Some(login_token) = login_token {
		println!("Attempting to login with token {login_token}");
		let _response = matrix_auth
			.login_token(&login_token)
			.initial_device_display_name(&format!("Element {}", rand::rng().next_u32() & 255))
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

	verification::register_handlers(&matrix_client);

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

	let Some(invite_pattern_to_join) = &ARGS.invite_pattern_to_accept else {
		return;
	};

	if !name.contains(invite_pattern_to_join) {
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

async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: matrix_sdk::Room, client: matrix_sdk::Client) {
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

	match text.body.trim() {
		"!status" => {
			println!("IKIRU");
			let content = RoomMessageEventContent::text_plain("IKIRU");
			let _ = room.send(content).await;
			return;
		},
		"!die" => {
			if let Ok(Some(sender)) = room.get_member(&event.sender).await
				&& sender.can_kick()
			{
				let _ = SHOULD_DIE.set(());
				println!("!die");
			}
			return;
		},
		_ => (),
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
		if let Err(e) = post_tweet(&event, &room, link, &client).await {
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

async fn post_tweet(
	_event: &OriginalSyncRoomMessageEvent,
	room: &matrix_sdk::Room,
	mut link: Url,
	_client: &matrix_sdk::Client,
) -> anyhow::Result<()> {
	link.set_host(Some("api.fxtwitter.com")).unwrap();
	link.set_path(&link.path().split('/').skip(1).take(3).join("/"));
	link.set_query(None);
	println!("{link}");
	let response = HTTP
		.get(link)
		.send()
		.await
		.context("Failed to fetch api.fxtwitter.com results")?;
	let response = response
		.json::<types::FxApiResponse>()
		.await
		.context("failed to parse as JSON into FxApiResponse")?;
	let types::Tweet { tweet, quote } = response.tweet.context("response.tweet was None")?;

	let quote_plain = if let Some(quote) = &quote {
		let t = quote.text.lines().join("\n> ");
		format!("\n> {} (@{})\n{}", quote.author.name, quote.author.screen_name, t)
	} else {
		"".into()
	};

	let body_plain = format!(
		"{} (@{})\n{}{}\n💬{} ♻️{} ❤️{} 👁️{}\n{}",
		tweet.author.name,
		tweet.author.screen_name,
		tweet.text,
		quote_plain,
		tweet.replies,
		tweet.retweets,
		tweet.likes,
		tweet.views.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string()),
		tweet.created_timestamp.strftime("%F %T")
	);

	let quote_html = if let Some(quote) = &quote {
		let mut tweet_url = quote.url.clone();
		tweet_url.set_host(Some("x.com")).unwrap();
		let safe_author_name = htmlize::escape_text(&quote.author.name);
		let safe_author_handle = quote.author.screen_name.as_str();
		let safe_tweet_body = htmlize::escape_text(&quote.text).lines().join("<br>");
		format!(
			r##"<blockquote class="fx-embed-quote" background-color="#6364FF">
			<p class="fx-embed-quote-author">
				<!-- <img data-mx-emoticon height="24" src="{{author_icon_url}}" title="Author icon" alt="">
				&nbsp; -->
				<span>
					Quoting <a href="{tweet_url}">{safe_author_name} (@{safe_author_handle})</a>
				</span>
			</p>
			<p class="fx-embed-quote-text">
				<span>
					{safe_tweet_body}
				</span>
			</p>
			</blockquote>"##
		)
	} else {
		"".into()
	};

	let mut tweet_url = tweet.url.clone();
	tweet_url.set_host(Some("x.com")).unwrap();
	let safe_author_name = htmlize::escape_text(&tweet.author.name);
	let safe_author_handle = tweet.author.screen_name.as_str();
	let safe_tweet_body = htmlize::escape_text(&tweet.text).lines().join("<br>");
	let body_html = format!(
		r##"<blockquote class="fx-embed" background-color="#6364FF">
		<p class="fx-embed-author">
			<!-- <img data-mx-emoticon height="24" src="{{author_icon_url}}" title="Author icon" alt="">
			&nbsp; -->
			<span>
				<a href="{tweet_url}">{safe_author_name} (@{safe_author_handle})</a>
			</span>
		</p>
		<p class="fx-embed-text">
			<span>
				{safe_tweet_body}
			</span>
		</p>
		<!-- {{file_html}} -->
		{quote_html}
		<p class="fx-bottom">
			<span>
				💬{} ♻️{} ❤️{} 👁️{}
			</span>
			<br>
			<span>
				{}
			</span>
		</p>
		</blockquote>"##,
		tweet.replies,
		tweet.retweets,
		tweet.likes,
		tweet.views.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string()),
		tweet.created_timestamp.strftime("%F %T")
	);

	let task_tweet = tokio::spawn({
		let room = room.clone();
		async move { room.send(RoomMessageEventContent::text_html(body_plain, body_html)).await }
	});

	let task_media = tokio::spawn({
		let room = room.clone();
		async move { fetch_and_post_media(room, tweet).await }
	});

	let te = task_tweet.await.unwrap().context("Failed to send tweet");
	let tm = task_media.await.unwrap();
	te?; // might eat errors from tm if this failed...
	tm?;

	Ok(())
}

async fn fetch_and_post_media(room: matrix_sdk::Room, tweet: types::TweetInner) -> anyhow::Result<()> {
	let Some(media) = tweet.media else {
		println!("  No media");
		return Ok(());
	};

	let upload_info = if let Some(videos) = media.videos {
		let video = &videos[0];
		let mut url = video.url.clone();
		if video.r#type == "gif" {
			url.set_path(&url.path().replace(".mp4", ".gif"));
		}
		let filename = url.path_segments().unwrap().last().unwrap().to_string();
		if video.r#type == "gif" {
			url.set_host(Some("gif.fxtwitter.com")).unwrap();
			UploadInfo {
				url: url,
				width: None,
				height: None,
				duration: None,
				content_type: mime::IMAGE_GIF,
				filename: filename,
				thumbnail_url: None,
			}
		} else {
			UploadInfo {
				url: url,
				width: Some(video.width),
				height: Some(video.height),
				duration: Some(video.duration),
				content_type: video.format.parse().unwrap(),
				filename: filename,
				thumbnail_url: Some(video.thumbnail_url.clone()),
			}
		}
	} else if let Some(mosaic) = media.mosaic {
		UploadInfo {
			url: mosaic.formats.webp.clone(),
			width: None,
			height: None,
			duration: None,
			content_type: "image/webp".parse().unwrap(),
			filename: format!("{}_mosaic.webp", tweet.id),
			thumbnail_url: None,
		}
	} else if let Some(photos) = media.photos {
		let photo = &photos[0];
		let filename = photo.url.path_segments().unwrap().last().unwrap();
		let content_type = if filename.ends_with(".jpg") {
			mime::IMAGE_JPEG
		} else {
			format!("image/{}", filename.split('.').last().unwrap()).parse().unwrap()
		};
		UploadInfo {
			url: photo.url.clone(),
			width: Some(photo.width),
			height: Some(photo.height),
			duration: None,
			content_type: content_type,
			filename: filename.to_string(),
			thumbnail_url: None,
		}
	} else {
		return Ok(());
	};

	let task_data = tokio::spawn({
		let media_url = upload_info.url.clone();
		async move {
			println!("  fetching & uploading {}", media_url);
			HTTP.get(media_url.clone())
				.send()
				.await
				.context("Failed to GET main file")?
				.error_for_status()
				.context("Bad status")?
				.bytes()
				.await
				.context("Failed to read entire body of main file")
		}
	});

	/*
	let encrypted_file = client
		.upload_encrypted_file(&mut std::io::Cursor::new(&data))
		.with_request_config(RequestConfig::short_retry())
		.await
		.context("Failed to upload media")?;
	println!("  uploaded {}", upload_info.url);

	let encrypted_file_url = encrypted_file.url.as_str();
	let file_html = if upload_info.filename.ends_with(".mp4") {
		format!(r##"<video controls><source src="{encrypted_file_url}" /></video>"##)
	} else {
		format!(r##"<img src="{encrypted_file_url}">"##)
	};
	*/

	let mut attachment_config = AttachmentConfig::new();

	let task_thumbnail: tokio::task::JoinHandle<anyhow::Result<Option<Thumbnail>>> = tokio::spawn({
		let thumbnail_url = upload_info.thumbnail_url.clone();
		async move {
			if let Some(thumbnail_url) = thumbnail_url {
				println!("  fetching thumbnail {thumbnail_url}");
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
				Ok(Some(thumbnail))
			} else {
				Ok(None)
			}
		}
	});

	let data = task_data.await.unwrap()?;
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

	match task_thumbnail.await.unwrap() {
		Ok(Some(thumbnail)) => {
			attachment_config = attachment_config.thumbnail(Some(thumbnail));
		},
		Ok(None) => (),
		Err(e) => {
			println!("  failed to fetch thumbnail {}: {e:?}", upload_info.thumbnail_url.unwrap());
		},
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

	Ok(())
}
