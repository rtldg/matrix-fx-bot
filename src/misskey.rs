use anyhow::Context;
use itertools::Itertools;
use reqwest::Url;
use scraper::Selector;
use serde::Deserialize;
use serde::Serialize;

use crate::HTTP;

pub(super) const TARGETS: &[&str] = &["misskey.io"];

#[derive(Serialize, Deserialize)]
struct Properties1 {
	height: i64,
	width: i64,
}
#[derive(Serialize, Deserialize)]
struct Files1 {
	blurhash: String,
	comment: Option<String>,
	createdAt: String,
	//folder: (),
	//folderId: (),
	id: String,
	isSensitive: bool,
	md5: String,
	name: String,
	properties: Properties1,
	size: i64,
	thumbnailUrl: Url,
	r#type: String,
	url: Url,
	//user: (),
	userId: String,
}
#[derive(Serialize, Deserialize)]
struct ReactionEmojis1 {}
#[derive(Serialize, Deserialize)]
struct Reactions1 {}
#[derive(Serialize, Deserialize)]
struct AvatarDecorations1 {
	id: String,
	url: String,
}
#[derive(Serialize, Deserialize)]
struct Emojis1 {}
#[derive(Serialize, Deserialize)]
struct User1 {
	avatarBlurhash: String,
	avatarDecorations: Vec<AvatarDecorations1>,
	avatarUrl: Url,
	//badgeRoles: Vec<()>,
	emojis: Emojis1,
	host: Option<String>,
	id: String,
	isBot: bool,
	isCat: bool,
	name: String,
	onlineStatus: String,
	username: String,
}
#[derive(Serialize, Deserialize)]
struct Note1 {
	clippedCount: i64,
	createdAt: jiff::Timestamp,
	//cw: (),
	fileIds: Vec<String>,
	files: Vec<Files1>,
	id: String,
	localOnly: bool,
	//reactionAcceptance: (),
	reactionCount: i64,
	reactionEmojis: ReactionEmojis1,
	reactions: Reactions1,
	renoteCount: i64,
	renoteId: Option<String>,
	renote: Option<Box<Note1>>,
	repliesCount: i64,
	replyId: Option<String>,
	text: Option<String>,
	user: User1,
	userId: String,
	visibility: String,
}
#[derive(Serialize, Deserialize)]
struct Root1 {
	note: Note1,
}

pub(super) async fn get_post(url: Url) -> anyhow::Result<crate::Post> {
	let mut post = crate::Post::default();

	let misskey = HTTP
		.get(url.clone())
		.send()
		.await
		.context("Failed to fetch misskey.io url")?
		.text()
		.await
		.context("Failed to fetch misskey.io html body")?;
	let misskey = scraper::Html::parse_document(&misskey);
	let selector = Selector::parse("#misskey_clientCtx").unwrap();
	let misskey = misskey
		.select(&selector)
		.next()
		.context("failed to find #misskey_clientCtx")?;
	let misskey = misskey.text().next().context("failed to extract #misskey_clientCtx json")?;
	let misskey = serde_json::from_str::<Root1>(misskey)
		.context("failed to parse #misskey_clientCtx json")?
		.note;

	let text = misskey.text.unwrap_or_default();

	post.body_plain = format!(
		"{} (@{})\n{}\n💬{} ♻️{} ❤️{}\n{}",
		misskey.user.name,
		misskey.user.username,
		text,
		misskey.repliesCount,
		misskey.renoteCount,
		misskey.reactionCount,
		misskey.createdAt.strftime("%F %T")
	);

	let safe_author_name = htmlize::escape_text(&misskey.user.name);
	let safe_body = htmlize::escape_text(&text).lines().join("<br>");
	// TODO: quotes -- with Note1.renote
	// TODO: alt text -- probably Files1.comment
	post.body_html = format!(
		r##"<blockquote class="fx-embed" background-color="#6364FF">
		<p class="fx-embed-author">
			<!-- <img data-mx-emoticon height="24" src="{{author_icon_url}}" title="Author icon" alt="">
			&nbsp; -->
			<span>
				<a href="{url}">{safe_author_name} (@{})</a>
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
				💬{} ♻️{} ❤️{}
			</span>
			<br>
			<span>
				{}
			</span>
		</p>
		</blockquote>"##,
		misskey.user.username,
		misskey.repliesCount,
		misskey.renoteCount,
		misskey.reactionCount,
		misskey.createdAt.strftime("%F %T")
	);

	for media in misskey.files {
		post.media.push(crate::Media {
			is_video: media.r#type.contains("video/"),
			url: media.url,
			thumbnail_url: Some(media.thumbnailUrl),
		});
	}

	Ok(post)
}
