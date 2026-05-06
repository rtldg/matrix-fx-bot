use std::str::FromStr;

use anyhow::Context;
use itertools::Itertools;
use reqwest::Url;
use scraper::Selector;

use crate::HTTP;

pub(super) async fn get_post(url: Url) -> anyhow::Result<crate::Post> {
	let mut post = crate::Post::default();

	let page = HTTP
		.get(url.clone())
		.send()
		.await
		.context("Failed to fetch opengraph url")?
		.text()
		.await
		.context("Failed to fetch opengraph html body")?;
	let page = scraper::Html::parse_document(&page);

	// TODO: page.select(&og_image).zip_longest(page.select(&og_image_alt))

	let get_og = |prop| {
		anyhow::Ok(
			page.select(&Selector::parse(&format!("meta[property=\"{prop}\"]")).unwrap())
				.next()
				.context(format!("missing og:title"))?
				.attr("content")
				.context(format!("missing content on property {prop}"))?,
		)
	};

	let title = get_og("og:title").map(|s| s.to_owned()).unwrap_or_else(|_| {
		if let Some(title) = page.select(&Selector::parse("title").unwrap()).next() {
			title.text().next().unwrap_or_default().chars().take(40).collect::<String>()
		} else {
			"".into()
		}
	});
	//let ogtype = get_og("og:type")?;
	let published_time = jiff::Timestamp::from_str(get_og("og:published_time").unwrap_or("2000-01-01T12:34:56Z"))?;
	let description = get_og("og:description").unwrap_or("");

	post.body_plain = format!("{title}\n{description}\n{}", published_time.strftime("%F %T"));

	let safe_title = htmlize::escape_text(title);
	let safe_body = htmlize::escape_text(description).lines().join("<br>");
	post.body_html = format!(
		r##"<blockquote class="fx-embed" background-color="#6364FF">
		<p class="fx-embed-author">
			<!-- <img data-mx-emoticon height="24" src="{{author_icon_url}}" title="Author icon" alt="">
			&nbsp; -->
			<span>
				<a href="{url}">{safe_title}</a>
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
				{}
			</span>
		</p>
		</blockquote>"##,
		published_time.strftime("%F %T")
	);

	// TODO: support multiple videos...
	if let Ok(video) = get_og("og:video") {
		post.media.push(crate::Media {
			is_video: true,
			url: video.parse()?,
			thumbnail_url: Some(get_og("og:image")?.parse()?),
		});
	} else {
		for image in page.select(&Selector::parse(&format!("meta[property=\"og:image\"]")).unwrap()) {
			let Some(url) = image.attr("content") else {
				continue;
			};
			post.media.push(crate::Media {
				is_video: false,
				url: url.parse()?,
				thumbnail_url: None,
			});
		}
	}

	Ok(post)
}
