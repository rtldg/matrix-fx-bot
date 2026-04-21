pub(super) const TARGETS: &[&str] = &["bsky.app", "xbsky.app"];

use std::time::Duration;

use anyhow::Context;
use itertools::Itertools;
use matrix_sdk::attachment::AttachmentConfig;
use matrix_sdk::attachment::BaseImageInfo;
use matrix_sdk::attachment::BaseVideoInfo;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;

use crate::HTTP;
use crate::UploadInfo;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BskyRoot {
	//pub original_data: OriginalData,
	pub parsed_data: ParsedData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OriginalData {
	pub thread: Thread,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Thread {
	pub post: Post,
	pub parent: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Post {
	pub author: Author,
	pub record: Record,
	pub embed: Embed,
	pub reply_count: i64,
	pub repost_count: i64,
	pub like_count: i64,
	pub quote_count: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Author {
	pub did: String,
	pub handle: String,
	pub display_name: String,
	pub avatar: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
	pub text: String,
	pub created_at: jiff::Timestamp,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Embed {
	#[serde(rename = "$type")]
	pub type_field: String,
	pub media: Media,
	pub external: External2,
	pub record: Record2,
	pub images: Vec<Image>,
	pub cid: String,
	pub thumbnail: String,
	pub aspect_ratio: AspectRatio3,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Media {
	#[serde(rename = "$type")]
	pub type_field: String,
	pub images: Value,
	pub external: External,
	pub cid: String,
	pub thumbnail: String,
	pub aspect_ratio: AspectRatio,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct External {
	pub uri: String,
	pub title: String,
	pub description: String,
	pub thumb: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AspectRatio {
	pub width: i64,
	pub height: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct External2 {
	pub uri: String,
	pub title: String,
	pub description: String,
	pub thumb: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record2 {
	#[serde(rename = "$type")]
	pub type_field: String,
	pub uri: String,
	pub record: Record3,
	pub value: Value2,
	pub author: Author3,
	pub embeds: Value,
	pub display_name: String,
	pub purpose: String,
	pub name: String,
	pub avatar: String,
	pub description: String,
	pub creator: Creator,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record3 {
	pub value: Value,
	pub author: Author2,
	pub name: String,
	pub description: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Value {
	pub text: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Author2 {
	pub did: String,
	pub handle: String,
	pub display_name: String,
	pub avatar: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Value2 {
	pub text: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Author3 {
	pub did: String,
	pub handle: String,
	pub display_name: String,
	pub avatar: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Creator {
	pub did: String,
	pub handle: String,
	pub display_name: String,
	pub avatar: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Image {
	pub fullsize: String,
	pub alt: String,
	pub aspect_ratio: AspectRatio2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AspectRatio2 {
	pub width: i64,
	pub height: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AspectRatio3 {
	pub width: i64,
	pub height: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParsedData {
	#[serde(rename = "type")]
	pub type_field: String,
	pub author: Author4,
	pub record: Record4,
	pub images: Vec<Image2>,
	pub external: External3,
	pub pds: String,
	#[serde(rename = "videoCID")]
	pub video_cid: String,
	#[serde(rename = "videoDID")]
	pub video_did: String,
	#[serde(rename = "videoURI")]
	pub video_uri: String,
	pub description: String,
	#[serde(rename = "statsForTG")]
	pub stats_for_tg: String,
	pub thumbnail: String,
	pub aspect_ratio: AspectRatio5,
	pub reply_count: i64,
	pub repost_count: i64,
	pub like_count: i64,
	pub quote_count: i64,
	pub is_video: bool,
	pub is_gif: bool,
	pub common_embeds: CommonEmbeds,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Author4 {
	pub did: String,
	pub handle: String,
	pub display_name: String,
	pub avatar: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record4 {
	pub text: String,
	pub created_at: jiff::Timestamp,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Image2 {
	pub fullsize: String,
	pub alt: String,
	pub aspect_ratio: AspectRatio4,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AspectRatio4 {
	pub width: i64,
	pub height: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct External3 {
	pub uri: String,
	pub title: String,
	pub description: String,
	pub thumb: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AspectRatio5 {
	pub width: i64,
	pub height: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommonEmbeds {
	pub purpose: String,
	pub name: String,
	pub avatar: String,
	pub description: String,
	pub creator: Creator2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Creator2 {
	pub did: String,
	pub handle: String,
	pub display_name: String,
	pub avatar: String,
}

pub(super) async fn post(
	_event: &OriginalSyncRoomMessageEvent,
	room: &matrix_sdk::Room,
	original_url: Url,
	_client: &matrix_sdk::Client,
) -> anyhow::Result<()> {
	let mut url = original_url.clone();
	url.set_host(Some("api.xbsky.app")).unwrap();
	println!("{url}");
	let response = HTTP.get(url).send().await.context("Failed to fetch api.xbsky.app results")?;
	let post = response
		.json::<BskyRoot>()
		.await
		.context("failed to parse as JSON into BskyRoot")?;
	let post = post.parsed_data;

	let media_url = if !post.video_uri.is_empty() {
		Some(post.video_uri.clone().parse()?)
	} else if !post.images.is_empty() {
		let mut mosaic = original_url.clone();
		mosaic.set_host(Some("mosaic.xbsky.app")).unwrap();
		Some(mosaic)
	} else {
		None
	};

	let body_plain = format!(
		"{} (@{})\n{}\n💬{} ❤️{}\n{}",
		post.author.display_name,
		post.author.handle,
		post.record.text,
		post.reply_count + post.quote_count,
		post.like_count,
		post.record.created_at.strftime("%F %T")
	);

	let safe_author_name = htmlize::escape_text(&post.author.display_name);
	let safe_body = htmlize::escape_text(&post.record.text).lines().join("<br>");
	// TODO: quotes
	let body_html = format!(
		r##"<blockquote class="fx-embed" background-color="#6364FF">
		<p class="fx-embed-author">
			<!-- <img data-mx-emoticon height="24" src="{{author_icon_url}}" title="Author icon" alt="">
			&nbsp; -->
			<span>
				<a href="{original_url}">{safe_author_name} (@{})</a>
			</span>
		</p>
		<p class="fx-embed-text">
			<span>
				{safe_body}
			</span>
		</p>
		<!-- {{file_html}} -->
		<p class="fx-bottom">
			<span>
				💬{} ❤️{}
			</span>
			<br>
			<span>
				{}
			</span>
		</p>
		</blockquote>"##,
		post.author.handle,
		post.reply_count + post.quote_count,
		post.like_count,
		post.record.created_at.strftime("%F %T")
	);

	let task_post = tokio::spawn({
		let room = room.clone();
		async move { room.send(RoomMessageEventContent::text_html(body_plain, body_html)).await }
	});

	let task_media = tokio::spawn({
		let room = room.clone();
		async move { fetch_and_post_media(room, post, media_url).await }
	});

	let te = task_post.await.unwrap().context("Failed to send post");
	let tm = task_media.await.unwrap();
	te?; // might eat errors from tm if this failed...
	tm?;

	Ok(())
}

async fn fetch_and_post_media(room: matrix_sdk::Room, post: ParsedData, media_url: Option<Url>) -> anyhow::Result<()> {
	let Some(media_url) = media_url else {
		println!("  No media");
		return Ok(());
	};
	let filename = media_url.path_segments().unwrap().last().unwrap().to_owned();

	let upload_info = if post.is_video {
		UploadInfo {
			url: media_url.clone(),
			width: None,    // TODO:
			height: None,   // TODO:
			duration: None, // TODO:
			content_type: "video/mp4".parse().unwrap(),
			filename: filename + ".mp4", // TODO: could be webm?
			thumbnail_url: None,         // TODO:
		}
	} else {
		// TODO: can't resolve until we fetch the stupid media...
		let content_type = if filename.ends_with(".jpg") || true {
			mime::IMAGE_JPEG
		} else {
			format!("image/{}", filename.split('.').last().unwrap()).parse().unwrap()
		};
		UploadInfo {
			url: media_url.clone(),
			width: None,  // TODO:
			height: None, // TODO:
			duration: None,
			content_type: content_type,
			filename: filename + ".jpg", // TODO: could be png, gif, etc...
			thumbnail_url: None,
		}
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

	let data = task_data.await.unwrap()?;
	/*
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
	*/

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
