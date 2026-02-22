use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub(crate) struct Author {
	pub avatar_url: Url,
	pub id: String,
	pub name: String,
	pub screen_name: String,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct VideoFormats {
	bitrate: Option<i64>,
	pub codec: Option<String>,
	pub container: String,
	pub url: Url,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Videos {
	pub format: String,
	pub formats: Vec<VideoFormats>,
	pub height: i64,
	pub id: String,
	pub thumbnail_url: Url,
	pub r#type: String,
	pub url: Url,
	pub width: i64,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Photos {
	pub id: String,
	pub r#type: String,
	pub url: Url,
	pub width: i64,
	pub height: i64,
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
	pub author: Author,
	pub created_at: String,
	pub created_timestamp: i64,
	pub id: String,
	pub likes: i64,
	pub media: Option<Media>,
	pub replies: i64,
	pub retweets: i64,
	pub text: String,
	pub views: i64,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct FxApiResponse {
	pub code: i64,
	pub message: String,
	pub tweet: Option<Tweet>,
}
