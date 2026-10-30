use crate::auth::{current, User};
use crate::*;
use axum::extract::Path;
use serde::Deserialize;

pub(crate) async fn group_allowed(app: &App, u: &User, id: i64) -> Result<()> {
    let n:i64=sqlx::query_scalar("SELECT COUNT(*) FROM study_groups WHERE academy_id=$1 AND id=$2 AND archived=false AND ($3='owner' OR teacher_id=$4)").bind(u.academy_id).bind(id).bind(&u.role).bind(u.id).fetch_one(&app.db).await?;
    if n == 0 {
        return Err(missing());
    }
    Ok(())
}
async fn rows(app: &App, u: &User, sql: &str) -> Result<Value> {
    Ok(sqlx::query_scalar(sql)
        .bind(u.academy_id)
        .bind(u.id)
        .bind(&u.role)
        .fetch_one(&app.db)
        .await?)
}
pub async fn bootstrap(State(app): State<App>, headers: HeaderMap) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    let academy: Value = sqlx::query_scalar("SELECT to_jsonb(a) FROM academies a WHERE id=$1")
        .bind(u.academy_id)
        .fetch_one(&app.db)
        .await?;
    let groups=rows(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.id DESC),'[]') FROM (SELECT g.*, t.name AS teacher_name,(SELECT count(*) FROM users s WHERE s.academy_id=g.academy_id AND s.group_id=g.id AND s.role='student') AS student_count FROM study_groups g JOIN users t ON t.id=g.teacher_id AND t.academy_id=g.academy_id WHERE g.academy_id=$1 AND ($3='owner' OR ($3='teacher' AND g.teacher_id=$2) OR g.id IN (SELECT group_id FROM users WHERE academy_id=$1 AND (id=$2 OR id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2))))) x").await?;
    let users=rows(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.id DESC),'[]') FROM (SELECT p.id,p.name,p.email,p.role,p.group_id,g.name AS group_name,p.created_at,p.archived,p.email_verified FROM users p LEFT JOIN study_groups g ON g.id=p.group_id AND g.academy_id=p.academy_id WHERE p.academy_id=$1 AND ($3='owner' OR p.id=$2 OR ($3='teacher' AND p.group_id IN(SELECT id FROM study_groups WHERE academy_id=$1 AND teacher_id=$2)) OR ($3='parent' AND p.id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2)))) x").await?;
    let assignments=rows(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.id DESC),'[]') FROM (SELECT a.*,g.name AS group_name FROM assignments a JOIN study_groups g ON g.id=a.group_id AND g.academy_id=a.academy_id WHERE a.academy_id=$1 AND ($3='owner' OR ($3='teacher' AND g.teacher_id=$2) OR a.group_id IN(SELECT group_id FROM users WHERE academy_id=$1 AND (id=$2 OR id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2))))) x").await?;
    let submissions=rows(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.id DESC),'[]') FROM (SELECT s.*,p.name AS student_name,a.title,a.skill,g.name AS group_name FROM submissions s JOIN users p ON p.id=s.student_id AND p.academy_id=s.academy_id JOIN assignments a ON a.id=s.assignment_id AND a.academy_id=s.academy_id JOIN study_groups g ON g.id=a.group_id AND g.academy_id=a.academy_id WHERE s.academy_id=$1 AND ($3='owner' OR ($3='teacher' AND g.teacher_id=$2) OR s.student_id=$2 OR ($3='parent' AND s.student_id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2)))) x").await?;
    let attendance=rows(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.lesson_date DESC),'[]') FROM (SELECT a.* FROM attendance a JOIN study_groups g ON g.id=a.group_id AND g.academy_id=a.academy_id WHERE a.academy_id=$1 AND ($3='owner' OR ($3='teacher' AND g.teacher_id=$2) OR a.student_id=$2 OR ($3='parent' AND a.student_id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2)))) x").await?;
    let leads = if u.staff().is_ok() {
        sqlx::query_scalar::<_,Value>("SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.id DESC),'[]') FROM (SELECT * FROM leads WHERE academy_id=$1 ORDER BY id DESC LIMIT 500) x").bind(u.academy_id).fetch_one(&app.db).await?
    } else {
        json!([])
    };
    let audit = if u.staff().is_ok() {
        rows(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.id DESC),'[]') FROM (SELECT a.id,a.action,a.detail,a.created_at,u.name AS actor_name FROM audit_log a LEFT JOIN users u ON u.id=a.actor_id WHERE a.academy_id=$1 AND ($3='owner' OR a.actor_id=$2) ORDER BY a.id DESC LIMIT 100) x").await?
    } else {
        json!([])
    };
    Ok(Json(
        json!({"user":u,"academy":academy,"groups":groups,"users":users,"assignments":assignments,"submissions":submissions,"attendance":attendance,"leads":leads,"audit":audit}),
    ))
}
#[derive(Deserialize)]
pub struct NewGroup {
    name: String,
    level: String,
    audience: String,
    teacher_id: i64,
    schedule: String,
}
pub async fn create_group(
    State(app): State<App>,
    headers: HeaderMap,
    Json(i): Json<NewGroup>,
) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    u.owner()?;
    if !valid_text(&i.name, 2, 80)
        || !valid_text(&i.level, 1, 30)
        || !valid_text(&i.schedule, 0, 200)
        || !matches!(i.audience.as_str(), "child" | "adult")
    {
        return Err(bad("Guruh maydonlarini tekshiring."));
    }
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM users WHERE academy_id=$1 AND id=$2 AND role IN ('owner','teacher') AND archived=false",
    )
    .bind(u.academy_id)
    .bind(i.teacher_id)
    .fetch_one(&app.db)
    .await?;
    if n == 0 {
        return Err(missing());
    }
    let mut tx = app.db.begin().await?;
    let id:i64=sqlx::query_scalar("INSERT INTO study_groups(academy_id,name,level,audience,teacher_id,schedule) VALUES($1,$2,$3,$4,$5,$6) RETURNING id").bind(u.academy_id).bind(i.name.trim()).bind(i.level.trim()).bind(&i.audience).bind(i.teacher_id).bind(i.schedule.trim()).fetch_one(&mut *tx).await?;
    audit(&mut tx, &u, "group.created", &i.name).await?;
    tx.commit().await?;
    Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)]
pub struct NewUser {
    name: String,
    email: String,
    password: String,
    role: String,
    group_id: Option<i64>,
    student_id: Option<i64>,
}
pub async fn create_user(
    State(app): State<App>,
    headers: HeaderMap,
    Json(i): Json<NewUser>,
) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    u.owner()?;
    let email = i.email.trim().to_lowercase();
    if !valid_text(&i.name, 2, 80)
        || !auth::email_valid(&email)
        || !matches!(i.role.as_str(), "teacher" | "student" | "parent")
    {
        return Err(bad("Hisob maydonlarini tekshiring."));
    }
    let gid = if i.role == "student" {
        let g = i
            .group_id
            .ok_or_else(|| bad("O‘quvchi uchun guruh tanlang."))?;
        group_allowed(&app, &u, g).await?;
        Some(g)
    } else {
        None
    };
    if i.role == "parent" {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM users WHERE academy_id=$1 AND id=$2 AND role='student'",
        )
        .bind(u.academy_id)
        .bind(i.student_id.unwrap_or(0))
        .fetch_one(&app.db)
        .await?;
        if n == 0 {
            return Err(bad("Ota-ona uchun farzandini tanlang."));
        }
    }
    let hashed = auth::hash(i.password).await?;
    let mut tx = app.db.begin().await?;
    let id:i64=sqlx::query_scalar("INSERT INTO users(academy_id,name,email,password_hash,role,group_id) VALUES($1,$2,$3,$4,$5,$6) RETURNING id").bind(u.academy_id).bind(i.name.trim()).bind(email).bind(hashed).bind(&i.role).bind(gid).fetch_one(&mut *tx).await?;
    if i.role == "parent" {
        sqlx::query("INSERT INTO parent_links(academy_id,parent_id,student_id) VALUES($1,$2,$3)")
            .bind(u.academy_id)
            .bind(id)
            .bind(i.student_id)
            .execute(&mut *tx)
            .await?;
    }
    audit(
        &mut tx,
        &u,
        "user.created",
        &format!("{} · {}", i.name, i.role),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)]
pub struct Assignment {
    group_id: i64,
    title: String,
    instructions: String,
    skill: String,
    #[serde(default)]
    due_date: String,
    #[serde(default = "text_mode")]
    answer_mode: String,
    lesson_id: Option<i64>,
}
fn text_mode() -> String {
    "text".into()
}
pub(crate) fn date_valid(s: &str) -> bool {
    let parts: Vec<_> = s.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        parts[0].parse::<u32>(),
        parts[1].parse::<u32>(),
        parts[2].parse::<u32>(),
    ) else {
        return false;
    };
    if !(2000..=2100).contains(&year) {
        return false;
    }
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    (1..=days).contains(&day)
}

pub async fn create_assignment(
    State(app): State<App>,
    headers: HeaderMap,
    Json(i): Json<Assignment>,
) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    u.staff()?;
    group_allowed(&app, &u, i.group_id).await?;
    if !valid_text(&i.title, 2, 120)
        || !valid_text(&i.instructions, 5, 4000)
        || !matches!(
            i.skill.as_str(),
            "grammar" | "vocabulary" | "reading" | "writing" | "speaking" | "listening"
        )
        || (!i.due_date.is_empty() && !date_valid(&i.due_date))
    {
        return Err(bad("Topshiriq ma’lumotlarini tekshiring."));
    }
    if !matches!(i.answer_mode.as_str(), "text" | "audio") {
        return Err(bad("Javob turi noto‘g‘ri."));
    }
    crate::v4::check_lesson(&app, u.academy_id, i.group_id, i.lesson_id).await?;
    let mut tx = app.db.begin().await?;
    let id:i64=sqlx::query_scalar("INSERT INTO assignments(academy_id,group_id,title,instructions,skill,due_date,created_by,answer_mode,lesson_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id").bind(u.academy_id).bind(i.group_id).bind(i.title.trim()).bind(i.instructions.trim()).bind(&i.skill).bind(&i.due_date).bind(u.id).bind(&i.answer_mode).bind(i.lesson_id).fetch_one(&mut *tx).await?;
    audit(&mut tx, &u, "assignment.created", &i.title).await?;
    crate::v4::notify_group(&mut tx, &u, i.group_id, "Yangi topshiriq", &i.title).await?;
    tx.commit().await?;
    Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)]
pub struct Answer {
    #[serde(default)]
    answer: String,
    audio_id: Option<String>,
}
pub async fn submit(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<Answer>,
) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    if u.role != "student" {
        return Err(forbidden());
    }
    let mode:Option<String>=sqlx::query_scalar("SELECT a.answer_mode FROM assignments a JOIN study_groups g ON g.id=a.group_id WHERE a.academy_id=$1 AND a.id=$2 AND a.group_id=$3 AND a.archived=false AND g.archived=false").bind(u.academy_id).bind(id).bind(u.group_id).fetch_optional(&app.db).await?;
    let mode = mode.ok_or_else(missing)?;
    if !valid_text(&i.answer, if mode == "audio" { 0 } else { 1 }, 4000) {
        return Err(bad("Javob matnini tekshiring."));
    }
    if mode == "audio" && i.audio_id.is_none() {
        return Err(bad("Audio javob kerak."));
    }
    if mode == "text" && i.audio_id.is_some() {
        return Err(bad("Bu yozma topshiriq."));
    }
    if let Some(ref audio) = i.audio_id {
        crate::media::owned(&app, &u, audio, "answer").await?;
    }
    let mut tx = app.db.begin().await?;
    let r=sqlx::query("INSERT INTO submissions(academy_id,assignment_id,student_id,answer,audio_id) VALUES($1,$2,$3,$4,$5) ON CONFLICT(academy_id,assignment_id,student_id) DO UPDATE SET answer=excluded.answer,audio_id=excluded.audio_id,revision=submissions.revision+1,submitted_at=now(),score=NULL,feedback=NULL,feedback_audio_id=NULL,reviewed_at=NULL,retry_allowed=false WHERE submissions.score IS NULL OR submissions.retry_allowed=true").bind(u.academy_id).bind(id).bind(u.id).bind(i.answer.trim()).bind(i.audio_id).execute(&mut *tx).await?;
    if r.rows_affected() == 0 {
        return Err(Error(
            StatusCode::CONFLICT,
            "Qayta topshirish uchun ustoz ruxsati kerak.".into(),
        ));
    }
    audit(&mut tx, &u, "answer.submitted", &format!("Topshiriq #{id}")).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
pub struct Review {
    score: i32,
    #[serde(default)]
    feedback: String,
    revision: i32,
    feedback_audio_id: Option<String>,
    #[serde(default)]
    retry_allowed: bool,
}
pub async fn review(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<Review>,
) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    u.staff()?;
    let row:Option<(i64,i64)>=sqlx::query_as("SELECT a.group_id,s.student_id FROM submissions s JOIN assignments a ON a.id=s.assignment_id AND a.academy_id=s.academy_id WHERE s.id=$1 AND s.academy_id=$2").bind(id).bind(u.academy_id).fetch_optional(&app.db).await?;
    let (gid, sid) = row.ok_or_else(missing)?;
    group_allowed(&app, &u, gid).await?;
    if !(0..=100).contains(&i.score)
        || !valid_text(
            &i.feedback,
            if i.feedback_audio_id.is_some() { 0 } else { 1 },
            2000,
        )
    {
        return Err(bad("Baho 0–100 va yozma yoki audio izoh kerak."));
    }
    if let Some(ref audio) = i.feedback_audio_id {
        crate::media::owned(&app, &u, audio, "feedback").await?;
    }
    let mut tx = app.db.begin().await?;
    let r=sqlx::query("UPDATE submissions SET score=$1,feedback=$2,reviewed_at=now(),feedback_audio_id=$6,retry_allowed=$7 WHERE id=$3 AND academy_id=$4 AND revision=$5").bind(i.score).bind(i.feedback.trim()).bind(id).bind(u.academy_id).bind(i.revision).bind(i.feedback_audio_id).bind(i.retry_allowed).execute(&mut *tx).await?;
    if r.rows_affected() == 0 {
        return Err(Error(
            StatusCode::CONFLICT,
            "Javob yangilangan. Ro‘yxatni yangilang.".into(),
        ));
    }
    audit(
        &mut tx,
        &u,
        "answer.reviewed",
        &format!("Javob #{id} · {} ball", i.score),
    )
    .await?;
    crate::v4::notify_student(
        &mut tx,
        u.academy_id,
        sid,
        "Javob baholandi",
        &format!("{} ball. Ustoz izohini ko‘ring.", i.score),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
pub struct LeadStatus {
    status: String,
}
pub async fn update_lead(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<LeadStatus>,
) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    u.staff()?;
    if !matches!(
        i.status.as_str(),
        "new" | "contacted" | "trial" | "enrolled"
    ) {
        return Err(bad("Noto‘g‘ri holat."));
    }
    let mut tx = app.db.begin().await?;
    let r = sqlx::query("UPDATE leads SET status=$1 WHERE id=$2 AND academy_id=$3")
        .bind(&i.status)
        .bind(id)
        .bind(u.academy_id)
        .execute(&mut *tx)
        .await?;
    if r.rows_affected() == 0 {
        return Err(missing());
    }
    audit(
        &mut tx,
        &u,
        "lead.updated",
        &format!("Ariza #{id}: {}", i.status),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
pub async fn public_academy(
    State(app): State<App>,
    Path(slug): Path<String>,
) -> Result<Json<Value>> {
    let a: Option<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('name',name,'slug',slug,'description',description,'contact',contact) FROM academies WHERE slug=$1",
    )
    .bind(slug)
    .fetch_optional(&app.db)
    .await?;
    Ok(Json(a.ok_or_else(missing)?))
}
#[derive(Deserialize)]
pub struct PublicLead {
    name: String,
    phone: String,
    audience: String,
    goal: String,
    consent: bool,
}
pub async fn public_lead(
    State(app): State<App>,
    Path(slug): Path<String>,
    Json(i): Json<PublicLead>,
) -> Result<Json<Value>> {
    let phone: String = i
        .phone
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '(' | ')'))
        .collect();
    if !valid_text(&i.name, 2, 80)
        || !valid_text(&i.goal, 0, 500)
        || !matches!(i.audience.as_str(), "child" | "adult")
        || !i.consent
        || !phone.starts_with("+998")
        || phone.len() != 13
        || !phone[1..].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(bad("Ism, +998 telefon, yo‘nalish va rozilikni tekshiring."));
    }
    let aid: Option<i64> = sqlx::query_scalar("SELECT id FROM academies WHERE slug=$1")
        .bind(slug)
        .fetch_optional(&app.db)
        .await?;
    sqlx::query("INSERT INTO leads(academy_id,name,phone,audience,goal) VALUES($1,$2,$3,$4,$5)")
        .bind(aid.ok_or_else(missing)?)
        .bind(i.name.trim())
        .bind(phone)
        .bind(i.audience)
        .bind(i.goal.trim())
        .execute(&app.db)
        .await?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
pub struct Attendance {
    group_id: i64,
    student_id: i64,
    lesson_date: String,
    status: String,
}
pub async fn attendance(
    State(app): State<App>,
    headers: HeaderMap,
    Json(i): Json<Attendance>,
) -> Result<Json<Value>> {
    let u = current(&app, &headers).await?;
    u.staff()?;
    group_allowed(&app, &u, i.group_id).await?;
    if !date_valid(&i.lesson_date) || !matches!(i.status.as_str(), "present" | "absent" | "late") {
        return Err(bad("Sana yoki holat noto‘g‘ri."));
    }
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM users WHERE academy_id=$1 AND id=$2 AND group_id=$3 AND role='student'").bind(u.academy_id).bind(i.student_id).bind(i.group_id).fetch_one(&app.db).await?;
    if n == 0 {
        return Err(missing());
    }
    let mut tx = app.db.begin().await?;
    sqlx::query("INSERT INTO attendance(academy_id,group_id,student_id,lesson_date,status) VALUES($1,$2,$3,$4,$5) ON CONFLICT(academy_id,student_id,lesson_date) DO UPDATE SET status=excluded.status").bind(u.academy_id).bind(i.group_id).bind(i.student_id).bind(&i.lesson_date).bind(&i.status).execute(&mut *tx).await?;
    audit(
        &mut tx,
        &u,
        "attendance.saved",
        &format!(
            "O‘quvchi #{} · {} · {}",
            i.student_id, i.lesson_date, i.status
        ),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}

#[cfg(test)]
mod date_tests {
    #[test]
    fn validates_calendar_dates() {
        assert!(super::date_valid("2028-02-29"));
        assert!(!super::date_valid("2026-02-29"));
        assert!(!super::date_valid("2026-04-31"));
        assert!(!super::date_valid("2100-02-29"));
        assert!(!super::date_valid("2026-13-01"));
    }
}
