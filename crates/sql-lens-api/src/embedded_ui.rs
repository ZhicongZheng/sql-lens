//! Compile-time embedded SPA served when no on-disk `static_dir` is configured.

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../sql-lens-app/web/dist"]
struct EmbeddedAssets;

/// Serve an embedded SPA asset, falling back to `index.html` for client routes.
pub async fn embedded_spa_fallback(req: Request) -> Response {
    // Only GET/HEAD make sense for static assets.
    if req.method() != axum::http::Method::GET && req.method() != axum::http::Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    let path = req.uri().path().trim_start_matches('/');
    let candidates = if path.is_empty() {
        vec!["index.html".to_owned()]
    } else {
        vec![path.to_owned(), "index.html".to_owned()]
    };

    for candidate in candidates {
        if let Some(file) = EmbeddedAssets::get(&candidate) {
            let mime = mime_guess::from_path(&candidate)
                .first_or_octet_stream()
                .essence_str()
                .to_owned();
            let mut response = Response::new(Body::from(file.data.into_owned()));
            if let Ok(value) = HeaderValue::from_str(&mime) {
                response.headers_mut().insert(header::CONTENT_TYPE, value);
            }
            return response;
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_assets_include_index_html() {
        assert!(
            EmbeddedAssets::get("index.html").is_some(),
            "index.html must be embedded when feature embedded-ui is enabled"
        );
    }
}
