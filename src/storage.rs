//! Private audio storage. The media handler authorizes access before calling this module.
use crate::{Error, Result};
use axum::http::StatusCode;
use reqwest::{Client, Method};
use std::{path::PathBuf, time::Duration};
use tokio::io::AsyncWriteExt;
const MAX: usize = 8 * 1024 * 1024;
fn failure(s: &str) -> Error {
    Error(StatusCode::BAD_GATEWAY, s.into())
}
fn io_error(_: std::io::Error) -> Error {
    failure("Audio faylni o‘qish/saqlashda xato.")
}
fn dir() -> PathBuf {
    std::env::var("MEDIA_DIR")
        .unwrap_or_else(|_| "data/media".into())
        .into()
}
pub fn backend() -> Result<String> {
    let b = std::env::var("MEDIA_BACKEND").unwrap_or_else(|_| "local".into());
    if b != "local" && b != "supabase" {
        return Err(failure("MEDIA_BACKEND local yoki supabase bo‘lsin."));
    }
    Ok(b)
}
struct Cloud {
    url: String,
    bucket: String,
    key: String,
    client: Client,
}
impl Cloud {
    fn from_env() -> Result<Self> {
        let url = std::env::var("SUPABASE_URL").map_err(|_| failure("SUPABASE_URL kerak."))?;
        let parsed = reqwest::Url::parse(&url).map_err(|_| failure("SUPABASE_URL noto‘g‘ri."))?;
        if parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.path() != "/"
        {
            return Err(failure("SUPABASE_URL faqat HTTPS loyiha manzili bo‘lsin."));
        }
        let bucket =
            std::env::var("SUPABASE_STORAGE_BUCKET").unwrap_or_else(|_| "educore-audio".into());
        if bucket.is_empty()
            || bucket.len() > 63
            || !bucket
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        {
            return Err(failure("Bucket nomi noto‘g‘ri."));
        }
        let key = std::env::var("SUPABASE_SECRET_KEY")
            .map_err(|_| failure("SUPABASE_SECRET_KEY kerak."))?;
        if !key.starts_with("sb_secret_") {
            return Err(failure(
                "Supabase server uchun sb_secret_ kalitini kiriting.",
            ));
        }
        Self::new(url.trim_end_matches('/').into(), bucket, key)
    }
    fn new(url: String, bucket: String, key: String) -> Result<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|_| failure("Storage HTTP sozlanmadi."))?;
        Ok(Self {
            url,
            bucket,
            key,
            client,
        })
    }
    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}/storage/v1/{path}", self.url))
            .header("apikey", &self.key)
    }
    async fn send(&self, r: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let response = r
            .send()
            .await
            .map_err(|_| failure("Supabase Storage bilan aloqa o‘rnatilmadi."))?;
        if !response.status().is_success() {
            return Err(failure(&format!(
                "Supabase Storage HTTP {}. Bucket va server kalitini tekshiring.",
                response.status().as_u16()
            )));
        }
        Ok(response)
    }
    async fn private(&self) -> Result<()> {
        let r = self
            .send(self.request(Method::GET, &format!("bucket/{}", self.bucket)))
            .await?;
        let b = bounded(r, 64 * 1024).await?;
        let v: serde_json::Value =
            serde_json::from_slice(&b).map_err(|_| failure("Bucket javobi noto‘g‘ri."))?;
        if v.get("public").and_then(|v| v.as_bool()) != Some(false) {
            return Err(failure("Audio bucket PRIVATE bo‘lishi shart."));
        }
        Ok(())
    }
    async fn put(&self, path: &str, mime: &str, bytes: &[u8]) -> Result<()> {
        self.private().await?;
        self.send(
            self.request(Method::POST, &format!("object/{}/{path}", self.bucket))
                .header("content-type", mime)
                .header("x-upsert", "false")
                .body(bytes.to_vec()),
        )
        .await?;
        Ok(())
    }
    async fn get(&self, path: &str) -> Result<Vec<u8>> {
        let r = self
            .send(self.request(
                Method::GET,
                &format!("object/authenticated/{}/{path}", self.bucket),
            ))
            .await?;
        bounded(r, MAX).await
    }
    async fn remove(&self, path: &str) -> Result<()> {
        self.send(
            self.request(Method::DELETE, &format!("object/{}", self.bucket))
                .json(&serde_json::json!({"prefixes":[path]})),
        )
        .await?;
        Ok(())
    }
}
async fn bounded(mut r: reqwest::Response, max: usize) -> Result<Vec<u8>> {
    if r.content_length().is_some_and(|n| n > max as u64) {
        return Err(failure("Storage javobi juda katta."));
    }
    let mut b = Vec::new();
    while let Some(chunk) = r.chunk().await.map_err(|_| failure("Audio yuklanmadi."))? {
        if b.len() + chunk.len() > max {
            return Err(failure("Storage javobi juda katta."));
        }
        b.extend_from_slice(&chunk);
    }
    Ok(b)
}
fn path(academy: i64, id: &str) -> Result<String> {
    if academy <= 0 || id.len() != 32 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(failure("Audio identifikatori noto‘g‘ri."));
    }
    Ok(format!("{academy}/{id}"))
}
pub async fn validate_startup() -> Result<()> {
    let b = backend()?;
    if std::env::var("RENDER").unwrap_or_default() == "true"
        && (b != "supabase" || std::env::var("COOKIE_SECURE").unwrap_or_default() != "true")
    {
        return Err(failure(
            "Render: MEDIA_BACKEND=supabase va COOKIE_SECURE=true kerak.",
        ));
    }
    if b == "supabase" {
        Cloud::from_env()?.private().await?;
    }
    Ok(())
}
pub async fn put(backend: &str, academy: i64, id: &str, mime: &str, bytes: &[u8]) -> Result<()> {
    let key = path(academy, id)?;
    if bytes.len() > MAX {
        return Err(failure("Audio 8 MB dan oshmasin."));
    }
    match backend {
        "supabase" => Cloud::from_env()?.put(&key, mime, bytes).await,
        "local" => {
            tokio::fs::create_dir_all(dir()).await.map_err(io_error)?;
            let p = dir().join(id);
            let mut f = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&p)
                .await
                .map_err(io_error)?;
            let r = async {
                f.write_all(bytes).await?;
                f.sync_all().await
            }
            .await;
            drop(f);
            if r.is_err() {
                let _ = tokio::fs::remove_file(&p).await;
            }
            r.map_err(io_error)
        }
        _ => Err(failure("Noma’lum storage backend.")),
    }
}
pub async fn get(backend: &str, academy: i64, id: &str) -> Result<Vec<u8>> {
    let key = path(academy, id)?;
    match backend {
        "supabase" => Cloud::from_env()?.get(&key).await,
        "local" => tokio::fs::read(dir().join(id)).await.map_err(io_error),
        _ => Err(failure("Noma’lum storage backend.")),
    }
}
pub async fn remove(backend: &str, academy: i64, id: &str) -> Result<()> {
    let key = path(academy, id)?;
    match backend {
        "supabase" => Cloud::from_env()?.remove(&key).await,
        "local" => tokio::fs::remove_file(dir().join(id))
            .await
            .map_err(io_error),
        _ => Err(failure("Noma’lum storage backend.")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Bytes,
        extract::State,
        http::{HeaderMap, Uri},
        response::IntoResponse,
        routing::any,
        Json, Router,
    };
    use std::sync::{Arc, Mutex};
    #[derive(Default)]
    struct Mock {
        public: bool,
        fail: bool,
        bytes: Vec<u8>,
        deleted: bool,
    }
    async fn handler(
        State(state): State<Arc<Mutex<Mock>>>,
        method: axum::http::Method,
        uri: Uri,
        h: HeaderMap,
        b: Bytes,
    ) -> axum::response::Response {
        assert_eq!(h.get("apikey").unwrap(), "sb_secret_test");
        assert!(h.get("authorization").is_none());
        let mut s = state.lock().unwrap();
        if s.fail {
            return (StatusCode::UNAUTHORIZED, "SECRET_BODY").into_response();
        }
        if uri.path() == "/storage/v1/bucket/educore-audio" {
            return Json(serde_json::json!({"public":s.public})).into_response();
        }
        match method.as_str() {
            "POST" => {
                assert_eq!(h.get("x-upsert").unwrap(), "false");
                s.bytes = b.to_vec();
                StatusCode::OK.into_response()
            }
            "GET" => {
                assert!(uri
                    .path()
                    .starts_with("/storage/v1/object/authenticated/educore-audio/"));
                s.bytes.clone().into_response()
            }
            "DELETE" => {
                let v: serde_json::Value = serde_json::from_slice(&b).unwrap();
                assert_eq!(v["prefixes"][0], "7/0123456789abcdef0123456789abcdef");
                s.deleted = true;
                StatusCode::OK.into_response()
            }
            _ => StatusCode::NOT_FOUND.into_response(),
        }
    }
    #[tokio::test]
    async fn private_storage_roundtrip_and_failures() {
        let state = Arc::new(Mutex::new(Mock::default()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new()
            .fallback(any(handler))
            .with_state(state.clone());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let c = Cloud::new(url, "educore-audio".into(), "sb_secret_test".into()).unwrap();
        let key = path(7, "0123456789abcdef0123456789abcdef").unwrap();
        c.put(&key, "audio/webm", b"audio example").await.unwrap();
        assert_eq!(c.get(&key).await.unwrap(), b"audio example");
        c.remove(&key).await.unwrap();
        assert!(state.lock().unwrap().deleted);
        state.lock().unwrap().public = true;
        assert!(c
            .put(&key, "audio/webm", b"no")
            .await
            .unwrap_err()
            .1
            .contains("PRIVATE"));
        state.lock().unwrap().fail = true;
        let e = c.get(&key).await.unwrap_err();
        assert!(!e.1.contains("SECRET_BODY"));
        assert!(e.1.contains("401"));
        {
            let mut s = state.lock().unwrap();
            s.fail = false;
            s.bytes = vec![0; MAX + 1];
        }
        assert!(c.get(&key).await.is_err());
        task.abort();
    }
    #[test]
    fn rejects_path_traversal() {
        assert!(path(1, "../../etc/passwd").is_err());
        assert!(path(-1, "0123456789abcdef0123456789abcdef").is_err());
    }
}
