pub mod ai;
pub mod auth;
pub mod import;
pub mod mail;
pub mod media;
pub mod platform;
pub mod storage;
pub mod v4;
pub mod v5;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, patch, post},
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct App {
    pub db: PgPool,
    pub secure_cookie: bool,
    pub auth_limits: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
}
impl App {
    pub fn new(db: PgPool, secure_cookie: bool) -> Self {
        Self {
            db,
            secure_cookie,
            auth_limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
#[derive(Debug)]
pub struct Error(pub StatusCode, pub String);
impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        eprintln!("database: {e}");
        if let sqlx::Error::Database(ref d) = e {
            if d.is_unique_violation() {
                return Self(
                    StatusCode::CONFLICT,
                    "Bunday ma’lumot avval kiritilgan.".into(),
                );
            }
        }
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Bazaga murojaatda xato. Qayta urinib ko‘ring.".into(),
        )
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn bad(s: &str) -> Error {
    Error(StatusCode::BAD_REQUEST, s.into())
}
pub fn forbidden() -> Error {
    Error(StatusCode::FORBIDDEN, "Bu amal uchun ruxsat yo‘q.".into())
}
pub fn missing() -> Error {
    Error(StatusCode::NOT_FOUND, "Ma’lumot topilmadi.".into())
}
pub fn valid_text(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.trim().chars().count())
        && !s
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}
pub async fn migrate(db: &PgPool) -> std::result::Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(db).await
}
pub fn router(app: App) -> Router {
    Router::new()
 .route("/",get(||async{Html(include_str!("../web/index.html"))}))
 .route("/app",get(||async{Html(include_str!("../web/app.html"))}))
 .route("/style.css",get(||async{([("content-type","text/css; charset=utf-8")],include_str!("../web/style.css"))}))
 .route("/app.js",get(||async{([("content-type","text/javascript; charset=utf-8")],include_str!("../web/app.js"))}))
 .route("/v4.js",get(||async{([("content-type","text/javascript; charset=utf-8")],include_str!("../web/v4.js"))}))
 .route("/v5.js",get(||async{([("content-type","text/javascript; charset=utf-8")],include_str!("../web/v5.js"))}))
 .route("/v5.css",get(||async{([("content-type","text/css; charset=utf-8")],include_str!("../web/v5.css"))}))
 .route("/api/config",get(||async{Json(json!({"registration":std::env::var("REGISTRATION_ENABLED").unwrap_or_else(|_|"true".into())=="true","recovery":mail::enabled()}))}))
 .route("/health",get(||async{Json(json!({"status":"ok","version":"0.5.1"}))}))
 .route("/api/register",post(auth::register)).route("/api/login",post(auth::login)).route("/api/logout",post(auth::logout))
 .route("/api/me",get(auth::me)).route("/api/password",post(auth::change_password))
 .route("/api/bootstrap",get(platform::bootstrap))
 .route("/api/groups",post(platform::create_group))
 .route("/api/users",post(platform::create_user))
 .route("/api/users/{id}/password",post(auth::reset_password))
 .route("/api/assignments",post(platform::create_assignment))
 .route("/api/assignments/{id}/answer",post(platform::submit))
 .route("/api/submissions/{id}/review",patch(platform::review))
 .route("/api/leads/{id}",patch(platform::update_lead))
 .route("/api/public/{slug}",get(platform::public_academy).post(platform::public_lead))
 .route("/api/attendance",post(platform::attendance))
 .merge(v4::routes()).merge(mail::routes()).merge(v5::routes())
 .layer(DefaultBodyLimit::max(32768))
 .merge(media::routes())
 .layer(axum::middleware::from_fn(|request:axum::extract::Request,next:axum::middleware::Next|async move{
   if !matches!(*request.method(),axum::http::Method::GET|axum::http::Method::HEAD|axum::http::Method::OPTIONS) && request.headers().get("x-educore").and_then(|h|h.to_str().ok())!=Some("1") {return forbidden().into_response()}
   let mut response=next.run(request).await;
   for (k,v) in [("cache-control","no-store"),("x-content-type-options","nosniff"),("x-frame-options","DENY"),("referrer-policy","same-origin"),("content-security-policy","default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; media-src 'self' blob:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'")] {response.headers_mut().insert(axum::http::HeaderName::from_static(k),v.parse().unwrap());}
   response
 }))
 .with_state(app)
}
pub async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user: &auth::User,
    action: &str,
    detail: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO audit_log(academy_id,actor_id,action,detail) VALUES($1,$2,$3,$4)")
        .bind(user.academy_id)
        .bind(user.id)
        .bind(action)
        .bind(detail)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
