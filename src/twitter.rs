use anyhow::Context as _;
use itertools::Itertools;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;

use crate::HTTP;

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

pub(super) async fn get_post(mut url: Url) -> anyhow::Result<crate::Post> {
	let mut post = crate::Post::default();

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

	post.body_plain = format!(
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
	// TODO: alt text
	post.body_html = format!(
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

	if let Some(media) = tweet.media {
		// TODO: post ALL images and ALL videos...
		if let Some(videos) = media.videos {
			let video = &videos[0];
			let mut url = videos[0].url.clone();
			if video.r#type == "gif" {
				url.set_path(&url.path().replace(".mp4", ".gif"));
				url.set_host(Some("gif.fxtwitter.com")).unwrap();
			}
			post.media.push(crate::Media {
				is_video: false,
				url: url,
				thumbnail_url: Some(video.thumbnail_url.clone()),
			});
		} else if let Some(mosaic) = media.mosaic {
			post.media.push(crate::Media {
				is_video: false,
				url: mosaic.formats.webp.clone(),
				thumbnail_url: None,
			});
		} else if let Some(photos) = media.photos {
			let photo = &photos[0];
			post.media.push(crate::Media {
				is_video: false,
				url: photo.url.clone(),
				thumbnail_url: None,
			})
		}
	}

	Ok(post)
}
