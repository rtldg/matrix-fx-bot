use anyhow::Context as _;
use itertools::Itertools;
use matrix_sdk::attachment::Thumbnail;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;

use crate::HTTP;
use crate::UploadInfo;

pub(super) const TARGETS: &[&str] = &[
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

#[derive(Serialize, Deserialize)]
pub(crate) struct Author {
	pub avatar_url: Url,
	pub id: String,
	pub name: String,
	pub screen_name: String,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct VideoFormats {
	bitrate: Option<u32>,
	pub codec: Option<String>,
	pub container: String,
	pub url: Url,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Videos {
	pub format: String,
	pub formats: Vec<VideoFormats>,
	pub duration: f64,
	pub id: String,
	pub thumbnail_url: Url,
	pub r#type: String,
	pub url: Url,
	pub width: u32,
	pub height: u32,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Photos {
	pub id: String,
	pub r#type: String,
	pub url: Url,
	pub width: u32,
	pub height: u32,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct MosaicFormats {
	pub jpeg: Url,
	pub webp: Url,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Mosaic {
	pub formats: MosaicFormats,
	pub r#type: String,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Media {
	pub mosaic: Option<Mosaic>,
	pub photos: Option<Vec<Photos>>,
	pub videos: Option<Vec<Videos>>,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Tweet {
	#[serde(flatten)]
	pub tweet: TweetInner,
	pub quote: Option<TweetInner>,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct TweetInner {
	pub author: Author,
	pub created_at: String,
	#[serde(with = "jiff::fmt::serde::timestamp::second::required")]
	pub created_timestamp: jiff::Timestamp,
	pub id: String,
	pub likes: i64,
	pub media: Option<Media>,
	pub replies: i64,
	pub retweets: i64,
	pub text: String,
	pub url: Url,
	pub views: Option<i64>,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct FxApiResponse {
	pub code: i64,
	pub message: String,
	pub tweet: Option<Tweet>,
}

pub(super) async fn post(
	_event: &OriginalSyncRoomMessageEvent,
	room: &matrix_sdk::Room,
	mut url: Url,
	_client: &matrix_sdk::Client,
) -> anyhow::Result<()> {
	url.set_host(Some("api.fxtwitter.com")).unwrap();
	url.set_path(&url.path().split('/').skip(1).take(3).join("/"));
	url.set_query(None);
	println!("{url}");
	let response = HTTP
		.get(url)
		.send()
		.await
		.context("Failed to fetch api.fxtwitter.com results")?;
	let response = response
		.json::<FxApiResponse>()
		.await
		.context("failed to parse as JSON into FxApiResponse")?;
	let Tweet { tweet, quote } = response.tweet.context("response.tweet was None")?;

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

async fn fetch_and_post_media(room: matrix_sdk::Room, tweet: TweetInner) -> anyhow::Result<()> {
	let Some(media) = tweet.media else {
		println!("  No media");
		return Ok(());
	};

	// TODO: make sure to post ALL videos
	let mut upload_info = if let Some(videos) = media.videos {
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
				let (w, h) = crate::get_image_dimensions(&thumbnail_data).unwrap_or((200, 200));
				let thumbnail = Thumbnail {
					data: thumbnail_data.to_vec(),
					content_type: mime::IMAGE_JPEG, // should always be truee
					height: h.into(),
					width: w.into(),
					size: (thumbnail_size as u32).into(),
				};
				Ok(Some(thumbnail))
			} else {
				Ok(None)
			}
		}
	});

	let data = task_data.await.unwrap()?;
	let mut attachment_config = upload_info.to_attachment_config(&data);

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
