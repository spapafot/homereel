use serde::{Deserialize, Deserializer, Serialize};

/// Drive `files` resource fields DriveTube requests.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub modified_time: Option<String>,
    #[serde(default)]
    pub created_time: Option<String>,
    #[serde(default, deserialize_with = "int64_string")]
    pub size: Option<u64>,
    #[serde(default)]
    pub md5_checksum: Option<String>,
    #[serde(default)]
    pub video_media_metadata: Option<VideoMediaMetadata>,
    #[serde(default)]
    pub has_thumbnail: bool,
    #[serde(default)]
    pub thumbnail_link: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMediaMetadata {
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default, deserialize_with = "int64_string")]
    pub duration_millis: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileList {
    #[serde(default)]
    pub files: Vec<DriveFile>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DriveFolder {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentList {
    #[serde(default)]
    pub files: Vec<FileParents>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileParents {
    #[serde(default)]
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveUser {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub email_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct About {
    pub user: DriveUser,
}

/// Drive encodes int64 fields such as `size` and `durationMillis` as JSON strings.
fn int64_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(u64),
    }

    match Option::<StringOrNumber>::deserialize(deserializer)? {
        None => Ok(None),
        Some(StringOrNumber::Number(value)) => Ok(Some(value)),
        Some(StringOrNumber::String(text)) => {
            text.parse().map(Some).map_err(serde::de::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_int64_strings_and_optional_metadata() {
        let json = serde_json::json!({
            "id": "f1",
            "name": "VID_20240612_181455.mp4",
            "mimeType": "video/mp4",
            "size": "104857600",
            "videoMediaMetadata": { "width": 1920, "height": 1080, "durationMillis": "754000" },
            "hasThumbnail": true
        });

        let file: DriveFile = serde_json::from_value(json).unwrap();

        assert_eq!(file.size, Some(104_857_600));
        let metadata = file.video_media_metadata.unwrap();
        assert_eq!(metadata.duration_millis, Some(754_000));
        assert_eq!(metadata.width, Some(1920));
        assert!(file.modified_time.is_none());
        assert!(file.parents.is_empty());
    }
}
