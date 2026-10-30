use crate::*;
use axum::{extract::Path, routing::delete};
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};
use serde::Deserialize;
pub fn enabled() -> bool {
    matches!(
        std::env::var("MAIL_MODE").unwrap_or_default().as_str(),
        "local" | "smtp"
    )
}
pub fn routes() -> Router<App> {
    Router::new()
        .route("/api/password/request", post(request_reset))
        .route("/api/password/confirm", post(confirm_reset))
        .route("/api/email/request", post(request_verify))
        .route("/api/email/confirm", post(confirm_verify))
        .route("/api/sessions", get(sessions))
        .route("/api/sessions/{id}", delete(revoke))
}
#[derive(Deserialize)]
struct RequestReset {
    slug: String,
    email: String,
}
async fn request_reset(State(app): State<App>, Json(i): Json<RequestReset>) -> Result<Json<Value>> {
    if !enabled() {
        return Err(Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Email xizmati sozlanmagan. Administrator parolni yangilashi mumkin.".into(),
        ));
    }
    auth::limited(&app, "mail-global".into(), 30)?;
    auth::limited(
        &app,
        format!(
            "reset:{}:{}",
            i.slug.trim().to_lowercase(),
            i.email.trim().to_lowercase()
        ),
        3,
    )?;
    let uid:Option<i64>=sqlx::query_scalar("SELECT u.id FROM users u JOIN academies a ON a.id=u.academy_id WHERE a.slug=$1 AND u.email=$2 AND u.archived=false").bind(i.slug.trim().to_lowercase()).bind(i.email.trim().to_lowercase()).fetch_optional(&app.db).await?;
    if let Some(id) = uid {
        queue_token(&app, id, "reset").await?;
    }
    Ok(Json(
        json!({"ok":true,"message":"Mos hisob mavjud bo‘lsa, tiklash havolasi yuboriladi."}),
    ))
}
async fn queue_token(app: &App, id: i64, purpose: &str) -> Result<()> {
    let base = std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".into());
    if !base.starts_with("https://")
        && !base.starts_with("http://127.0.0.1:")
        && !base.starts_with("http://localhost:")
    {
        return Err(bad("APP_BASE_URL HTTPS yoki localhost bo‘lsin."));
    }
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let mut tx = app.db.begin().await?;
    let email: String =
        sqlx::query_scalar("SELECT email FROM users WHERE id=$1 AND archived=false FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query("DELETE FROM account_tokens WHERE user_id=$1 AND purpose=$2")
        .bind(id)
        .bind(purpose)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO account_tokens(token_hash,user_id,purpose,expires_at) VALUES($1,$2,$3,now()+interval '30 minutes')").bind(auth::digest(&token)).bind(id).bind(purpose).execute(&mut *tx).await?;
    let subject = if purpose == "reset" {
        "EduCore: parolni tiklash"
    } else {
        "EduCore: emailni tasdiqlash"
    };
    let body=format!("{subject}\n\n{}/app#{purpose}={token}\n\nHavola 30 daqiqa va bir marta amal qiladi. Siz so‘ramagan bo‘lsangiz, e’tiborsiz qoldiring.",base.trim_end_matches('/'));
    sqlx::query("INSERT INTO mail_jobs(recipient,subject,body) VALUES($1,$2,$3)")
        .bind(email)
        .bind(subject)
        .bind(body)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
#[derive(Deserialize)]
struct Confirm {
    token: String,
    #[serde(default)]
    password: String,
}
async fn consume(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    token: &str,
    purpose: &str,
) -> Result<i64> {
    if token.len() != 64 {
        return Err(bad("Havola yaroqsiz yoki muddati tugagan."));
    }
    sqlx::query_scalar("DELETE FROM account_tokens t USING users u WHERE t.token_hash=$1 AND t.purpose=$2 AND t.expires_at>now() AND u.id=t.user_id AND u.archived=false RETURNING t.user_id").bind(auth::digest(token)).bind(purpose).fetch_optional(&mut **tx).await?.ok_or_else(||bad("Havola yaroqsiz yoki muddati tugagan."))
}
async fn confirm_reset(State(app): State<App>, Json(i): Json<Confirm>) -> Result<Json<Value>> {
    auth::limited(&app, "reset-confirm-global".into(), 20)?;
    let hashed = auth::hash(i.password).await?;
    let mut tx = app.db.begin().await?;
    let id = consume(&mut tx, &i.token, "reset").await?;
    sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
        .bind(hashed)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
async fn request_verify(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let u = auth::current(&app, &h).await?;
    if !enabled() {
        return Err(bad("Email xizmati sozlanmagan."));
    }
    auth::limited(&app, format!("verify:{}", u.id), 2)?;
    queue_token(&app, u.id, "verify").await?;
    Ok(Json(json!({"ok":true})))
}
async fn confirm_verify(State(app): State<App>, Json(i): Json<Confirm>) -> Result<Json<Value>> {
    auth::limited(&app, "verify-confirm-global".into(), 30)?;
    let mut tx = app.db.begin().await?;
    let id = consume(&mut tx, &i.token, "verify").await?;
    sqlx::query("UPDATE users SET email_verified=true WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
async fn sessions(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let u = auth::current(&app, &h).await?;
    let token = auth::digest(auth::token(&h).unwrap_or(""));
    let v:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(jsonb_build_object('id',token_hash,'expires_at',expires_at,'current',token_hash=$2) ORDER BY expires_at DESC),'[]') FROM sessions WHERE user_id=$1 AND expires_at>now()").bind(u.id).bind(token).fetch_one(&app.db).await?;
    Ok(Json(v))
}
async fn revoke(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let u = auth::current(&app, &h).await?;
    sqlx::query("DELETE FROM sessions WHERE user_id=$1 AND token_hash=$2")
        .bind(u.id)
        .bind(id)
        .execute(&app.db)
        .await?;
    Ok(Json(json!({"ok":true})))
}
pub async fn process_one(app: &App) -> Result<bool> {
    if !enabled() {
        return Ok(false);
    }
    let row:Option<(i64,String,String,String)>=sqlx::query_as("UPDATE mail_jobs SET status='processing',attempts=attempts+1,available_at=now()+interval '5 minutes' WHERE id=(SELECT id FROM mail_jobs WHERE status IN ('pending','processing') AND attempts<5 AND available_at<=now() ORDER BY id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING id,recipient,subject,body").fetch_optional(&app.db).await?;
    let Some((id, to, subject, body)) = row else {
        return Ok(false);
    };
    match deliver(id, &to, &subject, &body).await {
        Ok(()) => {
            sqlx::query("UPDATE mail_jobs SET status='sent',body='',last_error=NULL WHERE id=$1")
                .bind(id)
                .execute(&app.db)
                .await?;
        }
        Err(e) => {
            eprintln!("mail job #{id}: {e}");
            sqlx::query("UPDATE mail_jobs SET status=CASE WHEN attempts>=5 THEN 'failed' ELSE 'pending' END,available_at=now()+interval '1 minute',last_error='Yetkazilmadi; server jurnalini tekshiring' WHERE id=$1").bind(id).execute(&app.db).await?;
        }
    }
    Ok(true)
}
async fn deliver(
    id: i64,
    to: &str,
    subject: &str,
    body: &str,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if std::env::var("MAIL_MODE").unwrap_or_default() == "local" {
        let dir = std::env::var("MAIL_DIR").unwrap_or_else(|_| "data/mail".into());
        tokio::fs::create_dir_all(&dir).await?;
        tokio::fs::write(
            std::path::Path::new(&dir).join(format!("{id}.txt")),
            format!("To: {to}\nSubject: {subject}\n\n{body}"),
        )
        .await?;
        return Ok(());
    }
    let host = std::env::var("SMTP_HOST")?;
    let port = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".into())
        .parse::<u16>()?;
    let mail = Message::builder()
        .from(std::env::var("SMTP_FROM")?.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .body(body.to_owned())?;
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?
        .port(port)
        .timeout(Some(std::time::Duration::from_secs(30)));
    if let Ok(user) = std::env::var("SMTP_USER") {
        builder = builder.credentials(Credentials::new(user, std::env::var("SMTP_PASSWORD")?));
    }
    builder.build().send(mail).await?;
    Ok(())
}
pub async fn reminders(app: &App) -> Result<()> {
    sqlx::query("INSERT INTO notifications(academy_id,user_id,title,body,dedupe) SELECT a.academy_id,u.id,'Topshiriq muddati yaqin',a.title,'due:'||a.id||':'||a.due_date FROM assignments a JOIN users u ON u.academy_id=a.academy_id AND u.group_id=a.group_id JOIN study_groups g ON g.id=a.group_id WHERE a.archived=false AND g.archived=false AND u.archived=false AND u.role='student' AND a.due_date=to_char((now() AT TIME ZONE 'Asia/Tashkent')::date+1,'YYYY-MM-DD') AND NOT EXISTS(SELECT 1 FROM submissions s WHERE s.assignment_id=a.id AND s.student_id=u.id) ON CONFLICT(user_id,dedupe) DO NOTHING").execute(&app.db).await?;
    sqlx::query("DELETE FROM account_tokens WHERE expires_at<now()")
        .execute(&app.db)
        .await?;
    sqlx::query("DELETE FROM mail_jobs WHERE created_at<now()-interval '7 days'")
        .execute(&app.db)
        .await?;
    Ok(())
}
pub async fn worker(app: App) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut count = 0u64;
    loop {
        tick.tick().await;
        if let Err(e) = process_one(&app).await {
            eprintln!("worker: {}", e.1)
        }
        if count % 12 == 0 {
            if let Err(e) = reminders(&app).await {
                eprintln!("reminders: {}", e.1)
            }
        }
        count = count.wrapping_add(1);
    }
}
