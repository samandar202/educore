use crate::auth::{current, User};
use crate::*;
use axum::{body::Bytes, extract::Path};
const MAX: usize = 8 * 1024 * 1024;
pub fn routes() -> Router<App> {
    Router::new()
        .route("/api/media", post(upload).layer(DefaultBodyLimit::max(MAX)))
        .route("/api/media/{id}", get(download))
}
fn sniff(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        Some("audio/webm")
    } else if b.starts_with(b"OggS") {
        Some("audio/ogg")
    } else if b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WAVE") {
        Some("audio/wav")
    } else if b.starts_with(b"ID3") || (b.len() > 2 && b[0] == 255 && b[1] & 0xe0 == 0xe0) {
        Some("audio/mpeg")
    } else if b.get(4..8) == Some(b"ftyp") {
        Some("audio/mp4")
    } else {
        None
    }
}
async fn upload(State(app): State<App>, h: HeaderMap, b: Bytes) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    auth::limited(&app, format!("upload:{}", u.id), 20)?;
    let purpose = h
        .get("x-media-purpose")
        .and_then(|x| x.to_str().ok())
        .unwrap_or("");
    if !(purpose == "answer" && u.role == "student" || purpose == "feedback" && u.staff().is_ok()) {
        return Err(forbidden());
    }
    if b.len() < 12 || b.len() > MAX {
        return Err(bad("Audio 12 baytdan 8 MB gacha bo‘lsin."));
    }
    let mime = sniff(&b).ok_or_else(|| bad("WebM, Ogg, WAV, MP3 yoki M4A audio tanlang."))?;
    let declared = h
        .get("content-type")
        .and_then(|x| x.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("");
    if !(declared == mime
        || (mime == "audio/wav" && declared == "audio/x-wav")
        || (mime == "audio/mp4" && declared == "audio/x-m4a"))
    {
        return Err(bad("Fayl turi tarkibiga mos emas."));
    }
    let quota = std::env::var("MEDIA_QUOTA_MB")
        .ok()
        .and_then(|x| x.parse::<i64>().ok())
        .filter(|n| *n > 0 && *n <= 1048576)
        .unwrap_or(200)
        * 1024
        * 1024;
    let mut tx = app.db.begin().await?;
    crate::v4::lock(&mut tx, u.academy_id).await?;
    let used: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(size_bytes),0)::bigint FROM media_files WHERE academy_id=$1",
    )
    .bind(u.academy_id)
    .fetch_one(&mut *tx)
    .await?;
    if used + b.len() as i64 > quota {
        return Err(Error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Akademiya audio xotirasi to‘ldi. Administratorga murojaat qiling.".into(),
        ));
    }
    let id = uuid::Uuid::new_v4().simple().to_string();
    let backend = crate::storage::backend()?;
    crate::storage::put(&backend, u.academy_id, &id, mime, &b).await?;
    let saved=sqlx::query("INSERT INTO media_files(id,academy_id,owner_id,mime,size_bytes,purpose,storage_backend) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(&id).bind(u.academy_id).bind(u.id).bind(mime).bind(b.len() as i64).bind(purpose).bind(&backend).execute(&mut *tx).await;
    if let Err(e) = saved {
        let _ = crate::storage::remove(&backend, u.academy_id, &id).await;
        return Err(e.into());
    }
    tx.commit().await?;
    Ok(Json(json!({"id":id,"size":b.len(),"mime":mime})))
}
pub async fn owned(app: &App, u: &User, id: &str, purpose: &str) -> Result<()> {
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM media_files WHERE id=$1 AND academy_id=$2 AND owner_id=$3 AND purpose=$4").bind(id).bind(u.academy_id).bind(u.id).bind(purpose).fetch_one(&app.db).await?;
    if n == 0 {
        return Err(bad("Audio sizga tegishli emas yoki maqsadi noto‘g‘ri."));
    }
    Ok(())
}
async fn download(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::response::Response> {
    let u = current(&app, &h).await?;
    if id.len() != 32 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(missing());
    }
    let record:Option<(String,String,i64)>=sqlx::query_as("SELECT m.mime,m.storage_backend,m.size_bytes FROM media_files m WHERE m.id=$1 AND m.academy_id=$2 AND (m.owner_id=$3 OR EXISTS(SELECT 1 FROM (SELECT academy_id,assignment_id,student_id,audio_id,feedback_audio_id FROM submissions UNION ALL SELECT academy_id,assignment_id,student_id,audio_id,feedback_audio_id FROM submission_versions) s JOIN assignments a ON a.id=s.assignment_id AND a.academy_id=s.academy_id JOIN study_groups g ON g.id=a.group_id AND g.academy_id=a.academy_id WHERE s.academy_id=$2 AND (s.audio_id=m.id OR s.feedback_audio_id=m.id) AND ($4='owner' OR ($4='teacher' AND g.teacher_id=$3) OR s.student_id=$3 OR ($4='parent' AND s.student_id IN(SELECT student_id FROM parent_links WHERE academy_id=$2 AND parent_id=$3)))))").bind(&id).bind(u.academy_id).bind(u.id).bind(&u.role).fetch_optional(&app.db).await?;
    let (mime, backend, size) = record.ok_or_else(missing)?;
    let bytes = crate::storage::get(&backend, u.academy_id, &id).await?;
    if bytes.len() as i64 != size {
        return Err(bad("Audio hajmi bazadagi yozuvga mos emas."));
    }
    let len = bytes.len();
    if len == 0 {
        return Err(missing());
    }
    let (mut start, mut end, mut status) = (0, len - 1, StatusCode::OK);
    if let Some(range) = h.get("range") {
        if let Some((a, b)) = range.to_str().ok().and_then(|s| parse_range(s, len)) {
            start = a;
            end = b;
            status = StatusCode::PARTIAL_CONTENT
        } else {
            return Ok((
                StatusCode::RANGE_NOT_SATISFIABLE,
                [("content-range", format!("bytes */{len}"))],
            )
                .into_response());
        }
    }
    let mut r = (status, bytes[start..=end].to_vec()).into_response();
    let rh = r.headers_mut();
    rh.insert("content-type", mime.parse().unwrap());
    rh.insert("accept-ranges", "bytes".parse().unwrap());
    rh.insert(
        "content-length",
        (end - start + 1).to_string().parse().unwrap(),
    );
    if status == StatusCode::PARTIAL_CONTENT {
        rh.insert(
            "content-range",
            format!("bytes {start}-{end}/{len}").parse().unwrap(),
        );
    }
    Ok(r)
}
fn parse_range(s: &str, len: usize) -> Option<(usize, usize)> {
    if len == 0 {
        return None;
    }
    let (a, b) = s.strip_prefix("bytes=")?.split_once('-')?;
    if a.is_empty() {
        let n = b.parse::<usize>().ok()?;
        return if n == 0 {
            None
        } else {
            Some((len.saturating_sub(n), len - 1))
        };
    }
    let start = a.parse::<usize>().ok()?;
    let end = if b.is_empty() {
        len - 1
    } else {
        b.parse::<usize>().ok()?.min(len - 1)
    };
    if start >= len || start > end {
        None
    } else {
        Some((start, end))
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn ranges() {
        assert_eq!(super::parse_range("bytes=3-8", 10), Some((3, 8)));
        assert_eq!(super::parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(super::parse_range("bytes=10-", 10), None);
        assert_eq!(super::parse_range("bytes=0-1,4-5", 10), None);
    }
    #[test]
    fn rejects_html() {
        assert!(super::sniff(b"<html>hello</html>").is_none());
    }
}
