use crate::auth::{current, User};
use crate::*;
use axum::extract::Path;
use chrono::DateTime;
use serde::Deserialize;
pub fn routes() -> Router<App> {
    Router::new()
        .route("/api/v4", get(state))
        .route("/api/courses", post(course_new))
        .route("/api/courses/{id}", patch(course_edit))
        .route("/api/lessons", post(lesson_new))
        .route("/api/lessons/{id}", patch(lesson_edit))
        .route("/api/groups/{id}", patch(group_edit))
        .route("/api/users/{id}", patch(user_edit))
        .route("/api/assignments/{id}", patch(assignment_edit))
        .route("/api/calendar", post(event_new))
        .route("/api/calendar/{id}", patch(event_edit))
        .route("/api/notifications/{id}/read", post(read_notice))
}
pub async fn lock(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, aid: i64) -> Result<()> {
    sqlx::query("SELECT id FROM academies WHERE id=$1 FOR UPDATE")
        .bind(aid)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub async fn notify_group(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    u: &User,
    gid: i64,
    title: &str,
    body: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO notifications(academy_id,user_id,title,body) SELECT $1,id,$3,$4 FROM users WHERE academy_id=$1 AND archived=false AND (group_id=$2 OR id IN(SELECT parent_id FROM parent_links WHERE academy_id=$1 AND student_id IN(SELECT id FROM users WHERE academy_id=$1 AND group_id=$2)))").bind(u.academy_id).bind(gid).bind(title).bind(body).execute(&mut **tx).await?;
    Ok(())
}
pub async fn notify_student(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    aid: i64,
    sid: i64,
    title: &str,
    body: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO notifications(academy_id,user_id,title,body) SELECT $1,id,$3,$4 FROM users WHERE academy_id=$1 AND archived=false AND (id=$2 OR id IN(SELECT parent_id FROM parent_links WHERE academy_id=$1 AND student_id=$2))").bind(aid).bind(sid).bind(title).bind(body).execute(&mut **tx).await?;
    Ok(())
}
async fn scoped(app: &App, u: &User, sql: &str) -> Result<Value> {
    Ok(sqlx::query_scalar(sql)
        .bind(u.academy_id)
        .bind(u.id)
        .bind(&u.role)
        .fetch_one(&app.db)
        .await?)
}
async fn state(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    let courses=scoped(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.id DESC),'[]') FROM (SELECT c.* FROM courses c WHERE c.academy_id=$1 AND ($3 IN ('owner','teacher') OR c.id IN(SELECT course_id FROM study_groups WHERE academy_id=$1 AND id IN(SELECT group_id FROM users WHERE academy_id=$1 AND (id=$2 OR id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2)))))) x").await?;
    let lessons=scoped(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.position,x.id),'[]') FROM (SELECT l.* FROM course_lessons l WHERE l.academy_id=$1 AND ($3 IN ('owner','teacher') OR l.course_id IN(SELECT course_id FROM study_groups WHERE academy_id=$1 AND id IN(SELECT group_id FROM users WHERE academy_id=$1 AND (id=$2 OR id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2)))))) x").await?;
    let events=scoped(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.starts_at),'[]') FROM (SELECT e.*,g.name AS group_name,u.name AS teacher_name FROM calendar_events e JOIN study_groups g ON g.id=e.group_id JOIN users u ON u.id=e.teacher_id WHERE e.academy_id=$1 AND ($3='owner' OR ($3='teacher' AND e.teacher_id=$2) OR e.group_id IN(SELECT group_id FROM users WHERE academy_id=$1 AND (id=$2 OR id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2))))) x").await?;
    let history=scoped(&app,&u,"SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.submitted_at),'[]') FROM (SELECT v.*,a.title,a.skill,u.name AS student_name FROM submission_versions v JOIN assignments a ON a.id=v.assignment_id JOIN study_groups g ON g.id=a.group_id JOIN users u ON u.id=v.student_id WHERE v.academy_id=$1 AND ($3='owner' OR ($3='teacher' AND g.teacher_id=$2) OR v.student_id=$2 OR ($3='parent' AND v.student_id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2)))) x").await?;
    let notices:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(to_jsonb(n) ORDER BY n.id DESC),'[]') FROM (SELECT * FROM notifications WHERE academy_id=$1 AND user_id=$2 ORDER BY id DESC LIMIT 100) n").bind(u.academy_id).bind(u.id).fetch_one(&app.db).await?;
    let verified: bool = sqlx::query_scalar("SELECT email_verified FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&app.db)
        .await?;
    Ok(Json(
        json!({"courses":courses,"lessons":lessons,"events":events,"history":history,"notifications":notices,"profile":{"verified":verified},"mail_enabled":crate::mail::enabled()}),
    ))
}
async fn course_allowed(app: &App, u: &User, id: i64) -> Result<()> {
    u.staff()?;
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM courses WHERE academy_id=$1 AND id=$2 AND ($3='owner' OR created_by=$4)").bind(u.academy_id).bind(id).bind(&u.role).bind(u.id).fetch_one(&app.db).await?;
    if n == 0 {
        return Err(missing());
    }
    Ok(())
}
#[derive(Deserialize)]
struct Course {
    title: String,
    description: String,
    level: String,
    #[serde(default)]
    archived: bool,
}
async fn save_course(app: App, h: HeaderMap, id: Option<i64>, i: Course) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.staff()?;
    if let Some(id) = id {
        course_allowed(&app, &u, id).await?
    }
    if !valid_text(&i.title, 2, 120)
        || !valid_text(&i.description, 0, 4000)
        || !valid_text(&i.level, 1, 30)
    {
        return Err(bad("Kurs maydonlarini tekshiring."));
    }
    let mut tx = app.db.begin().await?;
    let id = if let Some(id) = id {
        sqlx::query("UPDATE courses SET title=$1,description=$2,level=$3,archived=$4 WHERE id=$5 AND academy_id=$6").bind(&i.title).bind(&i.description).bind(&i.level).bind(i.archived).bind(id).bind(u.academy_id).execute(&mut *tx).await?;
        id
    } else {
        sqlx::query_scalar("INSERT INTO courses(academy_id,title,description,level,created_by) VALUES($1,$2,$3,$4,$5) RETURNING id").bind(u.academy_id).bind(&i.title).bind(&i.description).bind(&i.level).bind(u.id).fetch_one(&mut *tx).await?
    };
    audit(&mut tx, &u, "course.saved", &i.title).await?;
    tx.commit().await?;
    Ok(Json(json!({"id":id})))
}
async fn course_new(
    State(a): State<App>,
    h: HeaderMap,
    Json(i): Json<Course>,
) -> Result<Json<Value>> {
    save_course(a, h, None, i).await
}
async fn course_edit(
    State(a): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<Course>,
) -> Result<Json<Value>> {
    save_course(a, h, Some(id), i).await
}
#[derive(Deserialize)]
struct Lesson {
    course_id: i64,
    module_title: String,
    title: String,
    content: String,
    resource_url: String,
    position: i32,
}
async fn save_lesson(app: App, h: HeaderMap, id: Option<i64>, i: Lesson) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    course_allowed(&app, &u, i.course_id).await?;
    if !valid_text(&i.module_title, 1, 120)
        || !valid_text(&i.title, 2, 120)
        || !valid_text(&i.content, 1, 8000)
        || !(1..=1000).contains(&i.position)
        || i.resource_url.len() > 1000
        || (!i.resource_url.is_empty() && !i.resource_url.starts_with("https://"))
    {
        return Err(bad("Dars maydonlari yoki HTTPS havolani tekshiring."));
    }
    let mut tx = app.db.begin().await?;
    let saved = if let Some(id) = id {
        let r=sqlx::query("UPDATE course_lessons SET module_title=$1,title=$2,content=$3,resource_url=$4,position=$5 WHERE id=$6 AND academy_id=$7 AND course_id=$8").bind(&i.module_title).bind(&i.title).bind(&i.content).bind(&i.resource_url).bind(i.position).bind(id).bind(u.academy_id).bind(i.course_id).execute(&mut *tx).await?;
        if r.rows_affected() == 0 {
            return Err(missing());
        }
        id
    } else {
        sqlx::query_scalar("INSERT INTO course_lessons(academy_id,course_id,module_title,title,content,resource_url,position) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING id").bind(u.academy_id).bind(i.course_id).bind(&i.module_title).bind(&i.title).bind(&i.content).bind(&i.resource_url).bind(i.position).fetch_one(&mut *tx).await?
    };
    audit(&mut tx, &u, "lesson.saved", &i.title).await?;
    tx.commit().await?;
    Ok(Json(json!({"id":saved})))
}
async fn lesson_new(
    State(a): State<App>,
    h: HeaderMap,
    Json(i): Json<Lesson>,
) -> Result<Json<Value>> {
    save_lesson(a, h, None, i).await
}
async fn lesson_edit(
    State(a): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<Lesson>,
) -> Result<Json<Value>> {
    save_lesson(a, h, Some(id), i).await
}
#[derive(Deserialize)]
struct GroupEdit {
    name: String,
    level: String,
    audience: String,
    teacher_id: i64,
    schedule: String,
    course_id: Option<i64>,
    archived: bool,
}
async fn group_edit(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<GroupEdit>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    if !valid_text(&i.name, 2, 80)
        || !valid_text(&i.level, 1, 30)
        || !valid_text(&i.schedule, 0, 200)
        || !matches!(i.audience.as_str(), "child" | "adult")
    {
        return Err(bad("Guruh maydonlarini tekshiring."));
    }
    let mut tx = app.db.begin().await?;
    lock(&mut tx, u.academy_id).await?;
    let old: i64 =
        sqlx::query_scalar("SELECT teacher_id FROM study_groups WHERE academy_id=$1 AND id=$2")
            .bind(u.academy_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(missing)?;
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM users WHERE academy_id=$1 AND id=$2 AND role IN ('owner','teacher') AND archived=false").bind(u.academy_id).bind(i.teacher_id).fetch_one(&mut *tx).await?;
    if n == 0 {
        return Err(bad("Faol ustoz tanlang."));
    }
    if old != i.teacher_id {
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM calendar_events WHERE academy_id=$1 AND group_id=$2 AND ends_at>now() AND cancelled=false").bind(u.academy_id).bind(id).fetch_one(&mut *tx).await?;
        if n > 0 {
            return Err(bad(
                "Avval kelajak va davom etayotgan darslarni bekor qiling.",
            ));
        }
    }
    if let Some(cid) = i.course_id {
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM courses WHERE academy_id=$1 AND id=$2 AND (archived=false OR id=(SELECT course_id FROM study_groups WHERE id=$3 AND academy_id=$1))").bind(u.academy_id).bind(cid).bind(id).fetch_one(&mut *tx).await?;
        if n == 0 {
            return Err(missing());
        }
    }
    sqlx::query("UPDATE study_groups SET name=$1,level=$2,audience=$3,teacher_id=$4,schedule=$5,course_id=$6,archived=$7 WHERE academy_id=$8 AND id=$9").bind(&i.name).bind(i.level).bind(i.audience).bind(i.teacher_id).bind(i.schedule).bind(i.course_id).bind(i.archived).bind(u.academy_id).bind(id).execute(&mut *tx).await?;
    if i.archived {
        sqlx::query("UPDATE calendar_events SET cancelled=true WHERE academy_id=$1 AND group_id=$2 AND ends_at>now()").bind(u.academy_id).bind(id).execute(&mut *tx).await?;
        notify_group(&mut tx, &u, id, "Guruh arxivlandi", &i.name).await?;
    }
    audit(&mut tx, &u, "group.updated", &i.name).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
struct UserEdit {
    name: String,
    group_id: Option<i64>,
    archived: bool,
}
async fn user_edit(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<UserEdit>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    if !valid_text(&i.name, 2, 80) {
        return Err(bad("Ismni tekshiring."));
    }
    let mut tx = app.db.begin().await?;
    lock(&mut tx, u.academy_id).await?;
    let role: String = sqlx::query_scalar(
        "SELECT role FROM users WHERE academy_id=$1 AND id=$2 AND role<>'owner'",
    )
    .bind(u.academy_id)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(missing)?;
    if role == "teacher" && i.archived {
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM study_groups WHERE academy_id=$1 AND teacher_id=$2 AND archived=false").bind(u.academy_id).bind(id).fetch_one(&mut *tx).await?;
        if n > 0 {
            return Err(bad(
                "Avval ustozning faol guruhlarini boshqa ustozga bering yoki arxivlang.",
            ));
        }
    }
    let gid = if role == "student" {
        let gid = i.group_id.ok_or_else(|| bad("Guruh tanlang."))?;
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM study_groups WHERE academy_id=$1 AND id=$2 AND (archived=false OR $3)").bind(u.academy_id).bind(gid).bind(i.archived).fetch_one(&mut *tx).await?;
        if n == 0 {
            return Err(missing());
        }
        Some(gid)
    } else {
        None
    };
    sqlx::query("UPDATE users SET name=$1,group_id=$2,archived=$3 WHERE academy_id=$4 AND id=$5")
        .bind(&i.name)
        .bind(gid)
        .bind(i.archived)
        .bind(u.academy_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if i.archived {
        sqlx::query("DELETE FROM sessions WHERE user_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM account_tokens WHERE user_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    audit(&mut tx, &u, "user.updated", &i.name).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
struct AssignmentEdit {
    title: String,
    instructions: String,
    due_date: String,
    archived: bool,
    lesson_id: Option<i64>,
}
async fn assignment_edit(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<AssignmentEdit>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.staff()?;
    let gid: i64 =
        sqlx::query_scalar("SELECT group_id FROM assignments WHERE academy_id=$1 AND id=$2")
            .bind(u.academy_id)
            .bind(id)
            .fetch_optional(&app.db)
            .await?
            .ok_or_else(missing)?;
    crate::platform::group_allowed(&app, &u, gid).await?;
    if !valid_text(&i.title, 2, 120)
        || !valid_text(&i.instructions, 5, 4000)
        || (!i.due_date.is_empty() && !crate::platform::date_valid(&i.due_date))
    {
        return Err(bad("Topshiriq maydonlarini tekshiring."));
    }
    check_lesson(&app, u.academy_id, gid, i.lesson_id).await?;
    let mut tx = app.db.begin().await?;
    sqlx::query("UPDATE assignments SET title=$1,instructions=$2,due_date=$3,archived=$4,lesson_id=$5 WHERE academy_id=$6 AND id=$7").bind(&i.title).bind(i.instructions).bind(i.due_date).bind(i.archived).bind(i.lesson_id).bind(u.academy_id).bind(id).execute(&mut *tx).await?;
    audit(&mut tx, &u, "assignment.updated", &i.title).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
pub async fn check_lesson(app: &App, aid: i64, gid: i64, lid: Option<i64>) -> Result<()> {
    if let Some(lid) = lid {
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM course_lessons l JOIN study_groups g ON g.academy_id=l.academy_id AND g.course_id=l.course_id WHERE l.academy_id=$1 AND l.id=$2 AND g.id=$3").bind(aid).bind(lid).bind(gid).fetch_one(&app.db).await?;
        if n == 0 {
            return Err(bad("Dars guruhning kursidan bo‘lsin."));
        }
    }
    Ok(())
}
#[derive(Deserialize)]
struct Event {
    group_id: i64,
    title: String,
    starts_at: String,
    ends_at: String,
    #[serde(default)]
    cancelled: bool,
}
async fn save_event(app: App, h: HeaderMap, id: Option<i64>, i: Event) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.staff()?;
    if !valid_text(&i.title, 2, 120) {
        return Err(bad("Dars nomini tekshiring."));
    }
    let start = DateTime::parse_from_rfc3339(&i.starts_at)
        .map_err(|_| bad("Boshlanish vaqti noto‘g‘ri."))?;
    let end =
        DateTime::parse_from_rfc3339(&i.ends_at).map_err(|_| bad("Tugash vaqti noto‘g‘ri."))?;
    if end <= start || (end - start).num_seconds() > 28800 {
        return Err(bad("Dars davomiyligi 0–8 soat bo‘lsin."));
    }
    let mut tx = app.db.begin().await?;
    lock(&mut tx, u.academy_id).await?;
    let teacher:i64=sqlx::query_scalar("SELECT teacher_id FROM study_groups WHERE academy_id=$1 AND id=$2 AND archived=false AND ($3='owner' OR teacher_id=$4)").bind(u.academy_id).bind(i.group_id).bind(&u.role).bind(u.id).fetch_optional(&mut *tx).await?.ok_or_else(missing)?;
    if let Some(id) = id {
        let old: Option<i64> = sqlx::query_scalar(
            "SELECT teacher_id FROM calendar_events WHERE academy_id=$1 AND id=$2",
        )
        .bind(u.academy_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        if old.is_none() || (u.role != "owner" && old != Some(u.id)) {
            return Err(missing());
        }
    }
    if !i.cancelled {
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM calendar_events WHERE academy_id=$1 AND cancelled=false AND (teacher_id=$2 OR group_id=$3) AND starts_at<$4::timestamptz AND ends_at>$5::timestamptz AND id<>$6").bind(u.academy_id).bind(teacher).bind(i.group_id).bind(&i.ends_at).bind(&i.starts_at).bind(id.unwrap_or(0)).fetch_one(&mut *tx).await?;
        if n > 0 {
            return Err(Error(
                StatusCode::CONFLICT,
                "Bu vaqtda ustoz yoki guruhning boshqa darsi bor.".into(),
            ));
        }
    }
    let saved = if let Some(id) = id {
        sqlx::query("UPDATE calendar_events SET group_id=$1,teacher_id=$2,title=$3,starts_at=$4::timestamptz,ends_at=$5::timestamptz,cancelled=$6 WHERE academy_id=$7 AND id=$8").bind(i.group_id).bind(teacher).bind(&i.title).bind(&i.starts_at).bind(&i.ends_at).bind(i.cancelled).bind(u.academy_id).bind(id).execute(&mut *tx).await?;
        id
    } else {
        sqlx::query_scalar("INSERT INTO calendar_events(academy_id,group_id,teacher_id,title,starts_at,ends_at,cancelled) VALUES($1,$2,$3,$4,$5::timestamptz,$6::timestamptz,$7) RETURNING id").bind(u.academy_id).bind(i.group_id).bind(teacher).bind(&i.title).bind(&i.starts_at).bind(&i.ends_at).bind(i.cancelled).fetch_one(&mut *tx).await?
    };
    notify_group(&mut tx, &u, i.group_id, "Dars jadvali yangilandi", &i.title).await?;
    audit(&mut tx, &u, "calendar.saved", &i.title).await?;
    tx.commit().await?;
    Ok(Json(json!({"id":saved})))
}
async fn event_new(
    State(a): State<App>,
    h: HeaderMap,
    Json(i): Json<Event>,
) -> Result<Json<Value>> {
    save_event(a, h, None, i).await
}
async fn event_edit(
    State(a): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<Event>,
) -> Result<Json<Value>> {
    save_event(a, h, Some(id), i).await
}
async fn read_notice(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    let r = sqlx::query(
        "UPDATE notifications SET read_at=now() WHERE academy_id=$1 AND user_id=$2 AND id=$3",
    )
    .bind(u.academy_id)
    .bind(u.id)
    .bind(id)
    .execute(&app.db)
    .await?;
    if r.rows_affected() == 0 {
        return Err(missing());
    }
    Ok(Json(json!({"ok":true})))
}
