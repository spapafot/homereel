use tauri::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use tauri::http::{Response, StatusCode};

use crate::db::{Database, people};

/// Serves person photos as `personphoto://localhost/<photoId>` (http://personphoto.localhost on
/// Windows). A photo never changes once stored; edits add new IDs, so it can be cached for good.
pub fn respond(db: &Database, photo_id: &str) -> Response<Vec<u8>> {
    let photo = match photo_id.parse::<i64>() {
        Ok(id) => db.with(|connection| people::photo(connection, id)),
        Err(_) => Ok(None),
    };
    let builder = Response::builder();
    let response = match photo {
        Ok(Some(bytes)) => builder
            .header(CONTENT_TYPE, "image/jpeg")
            .header(CACHE_CONTROL, "max-age=31536000, immutable")
            .body(bytes),
        Ok(None) => builder.status(StatusCode::NOT_FOUND).body(Vec::new()),
        Err(error) => {
            log::warn!("Person photo {photo_id} failed: {error}");
            builder
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Vec::new())
        }
    };
    response.unwrap_or_else(|_| Response::new(Vec::new()))
}
