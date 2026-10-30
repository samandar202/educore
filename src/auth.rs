use crate::*;
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub academy_id: i64,
    pub name: String,
    pub email: String,
    pub role: String,
    pub group_id: Option<i64>,
}
impl User {
    pub fn owner(&self) -> Result<()> {
        if self.role == "owner" {
            Ok(())
        } else {
            Err(forbidden())
        }
    }
    pub fn staff(&self) -> Result<()> {
        if self.role == "owner" || self.role == "teacher" {
            Ok(())
        } else {
            Err(forbidden())
        }
    }
}
pub fn digest(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
pub fn password_valid(s: &str) -> bool {
    (10..=128).contains(&s.chars().count())
}
pub fn email_valid(s: &str) -> bool {
    s.len() <= 160 && s.contains('@') && !s.chars().any(char::is_whitespace)
}
pub async fn hash(password: String) -> Result<String> {
    if !password_valid(&password) {
        return Err(bad("Parol 10–128 belgidan iborat bo‘lsin."));
    }
    tokio::task::spawn_blocking(move || {
        let salt =
            SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(|_| bad("Kalit xatosi."))?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|s| s.to_string())
            .map_err(|_| bad("Parol yaratishda xato."))
    })
    .await
    .map_err(|_| bad("Parol yaratishda xato."))?
}
async fn verify(password: String, hashed: String) -> bool {
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hashed)
            .map(|h| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &h)
                    .is_ok()
            })
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}
pub(crate) fn token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("cookie")
        .and_then(|h| h.to_str().ok())?
        .split(';')
        .find_map(|s| s.trim().strip_prefix("educore="))
}
pub async fn current(app: &App, headers: &HeaderMap) -> Result<User> {
    let t = token(headers)
        .filter(|s| s.len() == 64)
        .ok_or_else(|| Error(StatusCode::UNAUTHORIZED, "Hisobingizga kiring.".into()))?;
    sqlx::query_as("SELECT u.id,u.academy_id,u.name,u.email,u.role,u.group_id FROM users u JOIN sessions s ON s.user_id=u.id WHERE s.token_hash=$1 AND s.expires_at>now() AND u.archived=false")
 .bind(digest(t)).fetch_optional(&app.db).await?.ok_or_else(||Error(StatusCode::UNAUTHORIZED,"Sessiya tugadi. Qayta kiring.".into()))
}
fn cookie(token: &str, secure: bool, age: i64) -> String {
    format!(
        "educore={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={age}{}",
        if secure { "; Secure" } else { "" }
    )
}
async fn issue(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, id: i64) -> Result<String> {
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query("DELETE FROM sessions WHERE expires_at<now()")
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO sessions(token_hash,user_id,expires_at) VALUES($1,$2,now()+interval '12 hours')").bind(digest(&token)).bind(id).execute(&mut **tx).await?;
    Ok(token)
}
pub(crate) fn limited(app: &App, key: String, max: u32) -> Result<()> {
    let mut map = app
        .auth_limits
        .lock()
        .map_err(|_| bad("Qayta urinib ko‘ring."))?;
    map.retain(|_, (when, _)| when.elapsed().as_secs() < 60);
    if map.len() > 10000 {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "Bir daqiqadan keyin urinib ko‘ring.".into(),
        ));
    }
    let entry = map.entry(key).or_insert((Instant::now(), 0));
    entry.1 += 1;
    if entry.1 > max {
        Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "Juda ko‘p urinish. Bir daqiqadan keyin urinib ko‘ring.".into(),
        ))
    } else {
        Ok(())
    }
}
#[derive(Deserialize)]
pub struct Register {
    academy_name: String,
    slug: String,
    name: String,
    email: String,
    password: String,
}
pub async fn register(
    State(app): State<App>,
    Json(input): Json<Register>,
) -> Result<impl IntoResponse> {
    if std::env::var("REGISTRATION_ENABLED").unwrap_or_else(|_| "true".into()) != "true" {
        return Err(bad(
            "Yangi akademiya ochish yopilgan. Mavjud hisobingiz bilan kiring.",
        ));
    }
    limited(&app, "register-global".into(), 20)?;
    let slug = input.slug.trim().to_lowercase();
    let email = input.email.trim().to_lowercase();
    if !(3..=40).contains(&slug.len())
        || !slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || slug.starts_with('-')
        || !valid_text(&input.name, 2, 80)
        || !valid_text(&input.academy_name, 2, 80)
        || !email_valid(&email)
    {
        return Err(bad("Akademiya nomi, manzili, ism yoki emailni tekshiring."));
    }
    let hashed = hash(input.password).await?;
    let mut tx = app.db.begin().await?;
    let aid: i64 =
        sqlx::query_scalar("INSERT INTO academies(slug,name) VALUES($1,$2) RETURNING id")
            .bind(&slug)
            .bind(input.academy_name.trim())
            .fetch_one(&mut *tx)
            .await?;
    let id:i64=sqlx::query_scalar("INSERT INTO users(academy_id,name,email,password_hash,role) VALUES($1,$2,$3,$4,'owner') RETURNING id").bind(aid).bind(input.name.trim()).bind(&email).bind(hashed).fetch_one(&mut *tx).await?;
    let u = User {
        id,
        academy_id: aid,
        name: input.name.trim().into(),
        email,
        role: "owner".into(),
        group_id: None,
    };
    audit(&mut tx, &u, "academy.created", "Akademiya ochildi").await?;
    let t = issue(&mut tx, id).await?;
    tx.commit().await?;
    Ok((
        [("set-cookie", cookie(&t, app.secure_cookie, 43200))],
        Json(json!({"user":u,"slug":slug})),
    ))
}
#[derive(Deserialize)]
pub struct Login {
    slug: String,
    email: String,
    password: String,
}
pub async fn login(State(app): State<App>, Json(input): Json<Login>) -> Result<impl IntoResponse> {
    let slug = input.slug.trim().to_lowercase();
    let email = input.email.trim().to_lowercase();
    limited(&app, format!("login:{slug}:{email}"), 10)?;
    if input.password.len() > 512 {
        return Err(bad("Parol juda uzun."));
    }
    let row:Option<(i64,String)>=sqlx::query_as("SELECT u.id,u.password_hash FROM users u JOIN academies a ON a.id=u.academy_id WHERE a.slug=$1 AND u.email=$2 AND u.archived=false").bind(slug).bind(email).fetch_optional(&app.db).await?;
    let (id, hashed) = row.unwrap_or((0, String::new()));
    // Always perform Argon2 for unknown accounts too, avoiding a cheap enumeration path.
    let enabled = !hashed.is_empty();
    let checked = if hashed.is_empty() {
        hash("dummy-password-value".into()).await?
    } else {
        hashed
    };
    if !verify(input.password, checked).await || id == 0 || !enabled {
        return Err(Error(
            StatusCode::UNAUTHORIZED,
            "Akademiya, email yoki parol noto‘g‘ri.".into(),
        ));
    }
    let mut tx = app.db.begin().await?;
    let t = issue(&mut tx, id).await?;
    tx.commit().await?;
    Ok((
        [("set-cookie", cookie(&t, app.secure_cookie, 43200))],
        Json(json!({"ok":true})),
    ))
}
pub async fn logout(State(app): State<App>, headers: HeaderMap) -> Result<impl IntoResponse> {
    if let Some(t) = token(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token_hash=$1")
            .bind(digest(t))
            .execute(&app.db)
            .await?;
    }
    Ok((
        [("set-cookie", cookie("", app.secure_cookie, 0))],
        Json(json!({"ok":true})),
    ))
}
pub async fn me(State(app): State<App>, headers: HeaderMap) -> Result<Json<User>> {
    Ok(Json(current(&app, &headers).await?))
}
#[derive(Deserialize)]
pub struct ChangePassword {
    old_password: String,
    new_password: String,
}
pub async fn change_password(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<ChangePassword>,
) -> Result<impl IntoResponse> {
    let u = current(&app, &headers).await?;
    limited(&app, format!("password:{}", u.id), 5)?;
    let old: String =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE id=$1 AND academy_id=$2")
            .bind(u.id)
            .bind(u.academy_id)
            .fetch_one(&app.db)
            .await?;
    if !verify(input.old_password, old).await {
        return Err(bad("Joriy parol noto‘g‘ri."));
    }
    let hash = hash(input.new_password).await?;
    let mut tx = app.db.begin().await?;
    sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2 AND academy_id=$3")
        .bind(hash)
        .bind(u.id)
        .bind(u.academy_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM account_tokens WHERE user_id=$1 AND purpose='reset'")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    audit(&mut tx, &u, "password.changed", "Shaxsiy parol yangilandi").await?;
    tx.commit().await?;
    Ok((
        [("set-cookie", cookie("", app.secure_cookie, 0))],
        Json(json!({"ok":true})),
    ))
}
#[derive(Deserialize)]
pub struct Reset {
    password: String,
}
pub async fn reset_password(
    State(app): State<App>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(input): Json<Reset>,
) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    u.owner()?;
    let hash = hash(input.password).await?;
    let mut tx = app.db.begin().await?;
    let r = sqlx::query(
        "UPDATE users SET password_hash=$1 WHERE id=$2 AND academy_id=$3 AND role<>'owner'",
    )
    .bind(hash)
    .bind(id)
    .bind(u.academy_id)
    .execute(&mut *tx)
    .await?;
    if r.rows_affected() == 0 {
        return Err(missing());
    }
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM account_tokens WHERE user_id=$1 AND purpose='reset'")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit(&mut tx, &u, "user.password_reset", &format!("Hisob #{id}")).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
