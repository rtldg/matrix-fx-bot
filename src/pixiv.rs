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

pub(super) const TARGETS: &[&str] = &[
	"pixiv.net",
	"www.pixiv.net",
	"phixiv.net",
	"www.phixiv.net",
	"ppxiv.net",
	"www.ppxiv.net",
];

#[derive(Serialize, Deserialize)]
struct PhixivResponse {
	ai_generated: bool,
	author_id: String,
	author_name: String,
	bookmark_count: i64,
	comment_count: i64,
	create_date: jiff::Timestamp,
	description: String,
	illust_id: String,
	image_proxy_urls: Vec<Url>,
	is_ugoira: bool,
	language: String,
	like_count: i64,
	profile_image_url: Url,
	tags: Vec<String>,
	title: String,
	url: String,
	view_count: i64,
	x_restrict: i64,
}

pub(super) async fn post(
	_event: &OriginalSyncRoomMessageEvent,
	room: &matrix_sdk::Room,
	mut url: Url,
	_client: &matrix_sdk::Client,
) -> anyhow::Result<()> {
	url.set_host(Some("www.phixiv.net")).unwrap();
	let id = url
		.path()
		.split('/')
		.rev()
		.next()
		.context("Failed to grab the artwork ID")?
		.parse::<i64>()?;
	url.set_path("/api/info");
	url.set_query(Some(&format!("id={id}")));
	println!("{url}");
	let response = HTTP.get(url).send().await.context("Failed to fetch www.phixiv.net results")?;
	let post = response
		.json::<PhixivResponse>()
		.await
		.context("failed to parse as JSON into PhixivResponse")?;

	let unsafe_tags = format!("#{}", post.tags.iter().map(|s| s.trim_start_matches('#')).join(","));

	let media_count = if post.image_proxy_urls.len() > 1 {
		if post.image_proxy_urls[0].path().ends_with(".mp4") {
			post.image_proxy_urls.len() - 1
		} else {
			post.image_proxy_urls.len()
		}
	} else {
		1
	};
	let media_count = if media_count > 1 {
		format!(" ({media_count} total images)")
	} else {
		"".to_owned()
	};

	let body_plain = format!(
		"{} (by {})\n{}\n{unsafe_tags}\n💬{} 🙂{} ❤️{} 👁️{}{media_count}\n{}",
		post.title,
		post.author_name,
		post.description,
		post.comment_count,
		post.like_count,
		post.bookmark_count,
		post.view_count,
		post.create_date.strftime("%F %T")
	);

	let post_url = post.url.clone();
	let safe_author_name = htmlize::escape_text(&post.author_name);
	let safe_post_title = htmlize::escape_text(&post.title);
	//let safe_post_body = htmlize::escape_text(&post.description).lines().join("<br>");
	let yolo_body = &post.description;
	let safe_tags = htmlize::escape_text(&unsafe_tags);
	let maybe_br = if yolo_body.len() > 0 && safe_tags.len() > 0 {
		"<br>"
	} else {
		""
	};
	let body_html = format!(
		r##"<blockquote class="fx-embed" background-color="#6364FF">
		<p class="fx-embed-author">
			<!-- <img data-mx-emoticon height="24" src="{{author_icon_url}}" title="Author icon" alt="">
			&nbsp; -->
			<span>
				<a href="{post_url}">{safe_post_title} (by {safe_author_name})</a>
			</span>
		</p>
		<p class="fx-embed-text">
			<span>
				{yolo_body}
				{maybe_br}
				{safe_tags}
			</span>
		</p>
		<!-- {{file_html}} -->
		<p class="fx-bottom">
			<span>
				💬{} 🙂{} ❤️{} 👁️{}{media_count}
			</span>
			<br>
			<span>
				{}
			</span>
		</p>
		</blockquote>"##,
		post.comment_count,
		post.like_count,
		post.bookmark_count,
		post.view_count,
		post.create_date.strftime("%F %T")
	);

	let task_post = tokio::spawn({
		let room = room.clone();
		async move { room.send(RoomMessageEventContent::text_html(body_plain, body_html)).await }
	});

	let task_media = tokio::spawn({
		let room = room.clone();
		async move { fetch_and_post_media(room, post).await }
	});

	let te = task_post.await.unwrap().context("Failed to send post");
	let tm = task_media.await.unwrap();
	te?; // might eat errors from tm if this failed...
	tm?;

	Ok(())
}

async fn fetch_and_post_media(room: matrix_sdk::Room, post: PhixivResponse) -> anyhow::Result<()> {
	let media_count = if post.image_proxy_urls[0].path().ends_with(".mp4") {
		1
	} else {
		post.image_proxy_urls.len()
	};

	for media in post.image_proxy_urls.iter().take(media_count) {
		let filename = media.path_segments().unwrap().last().unwrap().to_owned();

		let upload_info = if media.path().ends_with(".mp4") {
			UploadInfo {
				url: media.clone(),
				width: None,    // TODO:
				height: None,   // TODO:
				duration: None, // TODO:
				content_type: "video/mp4".parse().unwrap(),
				filename: filename,
				thumbnail_url: None, // TODO:
			}
		} else {
			let content_type = if filename.ends_with(".jpg") {
				mime::IMAGE_JPEG
			} else {
				format!("image/{}", filename.split('.').last().unwrap()).parse().unwrap()
			};
			UploadInfo {
				url: media.clone(),
				width: None,  // TODO:
				height: None, // TODO:
				duration: None,
				content_type: content_type,
				filename: filename,
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

		let _ = room
			.send_attachment(
				upload_info.filename,
				&upload_info.content_type,
				task_data.await.unwrap()?.into(),
				AttachmentConfig::new(),
			)
			.await
			.context("Failed to send attachment")?;
		println!("  uploaded {}", upload_info.url);
	}

	Ok(())
}
