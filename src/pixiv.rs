use anyhow::Context;
use itertools::Itertools;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;

use crate::HTTP;

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

pub(super) async fn get_post(mut url: Url) -> anyhow::Result<crate::Post> {
	let mut post = crate::Post::default();

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
	let phixiv = response
		.json::<PhixivResponse>()
		.await
		.context("failed to parse as JSON into PhixivResponse")?;

	let unsafe_tags = format!("#{}", phixiv.tags.iter().map(|s| s.trim_start_matches('#')).join(","));

	let media_count = if phixiv.image_proxy_urls.len() > 1 {
		if phixiv.image_proxy_urls[0].path().ends_with(".mp4") {
			phixiv.image_proxy_urls.len() - 1
		} else {
			phixiv.image_proxy_urls.len()
		}
	} else {
		1
	};
	let media_count = if media_count > 1 {
		format!(" ({media_count} total images)")
	} else {
		"".to_owned()
	};

	post.body_plain = format!(
		"{} (by {})\n{}\n{unsafe_tags}\n💬{} 🙂{} ❤️{} 👁️{}{media_count}\n{}",
		phixiv.title,
		phixiv.author_name,
		phixiv.description,
		phixiv.comment_count,
		phixiv.like_count,
		phixiv.bookmark_count,
		phixiv.view_count,
		phixiv.create_date.strftime("%F %T")
	);

	let post_url = phixiv.url.clone();
	let safe_author_name = htmlize::escape_text(&phixiv.author_name);
	let safe_post_title = htmlize::escape_text(&phixiv.title);
	//let safe_post_body = htmlize::escape_text(&post.description).lines().join("<br>");
	let yolo_body = &phixiv.description;
	let safe_tags = htmlize::escape_text(&unsafe_tags);
	let maybe_br = if yolo_body.len() > 0 && safe_tags.len() > 0 {
		"<br>"
	} else {
		""
	};
	post.body_html = format!(
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
		phixiv.comment_count,
		phixiv.like_count,
		phixiv.bookmark_count,
		phixiv.view_count,
		phixiv.create_date.strftime("%F %T")
	);

	if phixiv.image_proxy_urls[0].path().ends_with(".mp4") {
		post.media.push(crate::Media {
			is_video: true,
			url: phixiv.image_proxy_urls[0].clone(),
			thumbnail_url: None,
		});
	} else {
		for url in phixiv.image_proxy_urls {
			post.media.push(crate::Media {
				is_video: false,
				url: url,
				thumbnail_url: None,
			});
		}
	}

	Ok(post)
}
