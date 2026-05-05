pub(super) const TARGETS: &[&str] = &["bsky.app", "xbsky.app"];

use anyhow::Context;
use itertools::Itertools;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;

use crate::HTTP;

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

pub(super) async fn get_post(original_url: Url) -> anyhow::Result<crate::Post> {
	let mut post = crate::Post::default();

	let mut url = original_url.clone();
	url.set_host(Some("api.xbsky.app")).unwrap();
	println!("{url}");
	let response = HTTP.get(url).send().await.context("Failed to fetch api.xbsky.app results")?;
	let bsky = response
		.json::<BskyRoot>()
		.await
		.context("failed to parse as JSON into BskyRoot")?;
	let bsky = bsky.parsed_data;

	post.body_plain = format!(
		"{} (@{})\n{}\n💬{} ❤️{}\n{}",
		bsky.author.display_name,
		bsky.author.handle,
		bsky.record.text,
		bsky.reply_count + bsky.quote_count,
		bsky.like_count,
		bsky.record.created_at.strftime("%F %T")
	);

	let safe_author_name = htmlize::escape_text(&bsky.author.display_name);
	let safe_body = htmlize::escape_text(&bsky.record.text).lines().join("<br>");
	// TODO: quotes
	// TODO: alt text
	post.body_html = format!(
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
		bsky.author.handle,
		bsky.reply_count + bsky.quote_count,
		bsky.like_count,
		bsky.record.created_at.strftime("%F %T")
	);

	if !bsky.video_uri.is_empty() {
		post.media.push(crate::Media {
			is_video: true,
			url: bsky.video_uri.clone().parse()?,
			thumbnail_url: None,
		});
	} else if !bsky.images.is_empty() {
		let mut mosaic = original_url.clone();
		mosaic.set_host(Some("mosaic.xbsky.app")).unwrap();
		post.media.push(crate::Media {
			is_video: true,
			url: mosaic,
			thumbnail_url: None,
		});
	}

	Ok(post)
}
