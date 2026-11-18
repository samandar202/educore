//! V6 hamjamiyat: yagona akademiya, erkin ro'yxat, o'qituvchi tasdig'i,
//! farzand kodi va o'yin (XP, daraja, nishonlar, reyting, so'z o'yini).
use crate::auth::{current, User};
use crate::*;
use axum::extract::Path;
use serde::Deserialize;
use uuid::Uuid;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/api/community", get(community))
        .route("/api/join", post(join))
        .route("/api/pending", get(pending))
        .route("/api/users/{id}/approve", post(approve))
        .route("/api/users/{id}/reject", post(reject))
        .route("/api/link-code", get(link_code))
        .route("/api/link-child", post(link_child))
        .route("/api/game", get(game))
        .route("/api/game/score", post(score))
}

pub fn signup_open() -> bool {
    std::env::var("REGISTRATION_ENABLED").unwrap_or_else(|_| "true".into()) != "false"
}

/// Yagona akademiya (birinchi ochilgani).
pub async fn the_academy(db: &sqlx::PgPool) -> Result<Option<(i64, String, String)>> {
    Ok(
        // Faol egasi bor akademiya ustun; bo'lmasa birinchisi.
        sqlx::query_as("SELECT a.id,a.slug,a.name FROM academies a ORDER BY EXISTS(SELECT 1 FROM users u WHERE u.academy_id=a.id AND u.role='owner' AND u.archived=false) DESC, a.id LIMIT 1")
            .fetch_optional(db)
            .await?,
    )
}

async fn community(State(app): State<App>) -> Result<Json<Value>> {
    let a = the_academy(&app.db).await?;
    Ok(Json(json!({
        "has_owner": a.is_some(),
        "academy": a.map(|(_, slug, name)| json!({"slug": slug, "name": name})),
        "signup": signup_open(),
    })))
}

#[derive(Deserialize)]
pub struct Join {
    role: String,
    name: String,
    email: String,
    password: String,
    #[serde(default)]
    child_code: String,
}

async fn join(State(app): State<App>, Json(i): Json<Join>) -> Result<impl IntoResponse> {
    if !signup_open() {
        return Err(bad("Ro‘yxatdan o‘tish vaqtincha yopilgan."));
    }
    auth::limited(&app, "join-global".into(), 30)?;
    let (aid, _, _) = the_academy(&app.db)
        .await?
        .ok_or_else(|| bad("Akademiya hali ochilmagan."))?;
    let email = i.email.trim().to_lowercase();
    if !matches!(i.role.as_str(), "student" | "teacher" | "parent")
        || !valid_text(&i.name, 2, 80)
        || !auth::email_valid(&email)
    {
        return Err(bad("Ism, email va rolni tekshiring."));
    }
    let child: Option<i64> = if i.role == "parent" {
        let code = i.child_code.trim().to_uppercase();
        if code.len() != 6 {
            return Err(bad("Farzandingizning 6 xonali kodini kiriting."));
        }
        auth::limited(&app, format!("child-code:{email}"), 8)?;
        Some(
            sqlx::query_scalar("SELECT id FROM users WHERE academy_id=$1 AND role='student' AND link_code=$2 AND archived=false")
                .bind(aid).bind(&code).fetch_optional(&app.db).await?
                .ok_or_else(|| bad("Bunday farzand kodi topilmadi. Kodni farzandingiz kabinetidan tekshiring."))?,
        )
    } else {
        None
    };
    let hashed = auth::hash(i.password).await?;
    let pending = i.role == "teacher";
    let mut tx = app.db.begin().await?;
    let id: i64 = sqlx::query_scalar("INSERT INTO users(academy_id,name,email,password_hash,role,pending) VALUES($1,$2,$3,$4,$5,$6) RETURNING id")
        .bind(aid).bind(i.name.trim()).bind(&email).bind(hashed).bind(&i.role).bind(pending)
        .fetch_one(&mut *tx).await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.is_unique_violation() => Error(StatusCode::CONFLICT, "Bu email bilan hisob mavjud. Kirish bo‘limidan foydalaning.".into()),
            e => e.into(),
        })?;
    if let Some(sid) = child {
        sqlx::query("INSERT INTO parent_links(academy_id,parent_id,student_id) VALUES($1,$2,$3)")
            .bind(aid).bind(id).bind(sid).execute(&mut *tx).await?;
    }
    let u = User { id, academy_id: aid, name: i.name.trim().into(), email, role: i.role.clone(), group_id: None };
    if pending {
        sqlx::query("INSERT INTO notifications(academy_id,user_id,title,body) SELECT $1,id,$2,$3 FROM users WHERE academy_id=$1 AND role='owner' AND archived=false")
            .bind(aid).bind("Yangi o‘qituvchi tasdiq kutmoqda")
            .bind(format!("{} ({}) o‘qituvchi sifatida ro‘yxatdan o‘tdi. Bosh sahifada tasdiqlang yoki rad eting.", u.name, u.email))
            .execute(&mut *tx).await?;
        tx.commit().await?;
        return Ok(Json(json!({"pending": true})).into_response());
    }
    audit(&mut tx, &u, "user.joined", &format!("{} o‘zi ro‘yxatdan o‘tdi ({})", u.name, u.role)).await?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query("INSERT INTO sessions(token_hash,user_id,expires_at) VALUES($1,$2,now()+interval '12 hours')")
        .bind(auth::digest(&token)).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok((
        [("set-cookie", auth::session_cookie(&token, app.secure_cookie))],
        Json(json!({"ok": true})),
    ).into_response())
}

async fn pending(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    Ok(Json(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(jsonb_build_object('id',id,'name',name,'email',email,'created_at',created_at) ORDER BY id),'[]') FROM users WHERE academy_id=$1 AND pending=true AND archived=false")
        .bind(u.academy_id).fetch_one(&app.db).await?))
}

async fn approve(State(app): State<App>, h: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    let mut tx = app.db.begin().await?;
    let name: String = sqlx::query_scalar("UPDATE users SET pending=false WHERE academy_id=$1 AND id=$2 AND pending=true AND archived=false RETURNING name")
        .bind(u.academy_id).bind(id).fetch_optional(&mut *tx).await?.ok_or_else(missing)?;
    sqlx::query("INSERT INTO notifications(academy_id,user_id,title,body) VALUES($1,$2,'Hisobingiz tasdiqlandi','Endi kabinetga kirib ishlashingiz mumkin.')")
        .bind(u.academy_id).bind(id).execute(&mut *tx).await?;
    audit(&mut tx, &u, "user.approved", &format!("{name} o‘qituvchi sifatida tasdiqlandi")).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok": true})))
}

async fn reject(State(app): State<App>, h: HeaderMap, Path(id): Path<i64>) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    let mut tx = app.db.begin().await?;
    // Email bo'shatiladi, shunda odam qayta ro'yxatdan o'ta oladi.
    let name: String = sqlx::query_scalar("UPDATE users SET archived=true,pending=false,email=email||'#rad-'||id WHERE academy_id=$1 AND id=$2 AND pending=true AND archived=false RETURNING name")
        .bind(u.academy_id).bind(id).fetch_optional(&mut *tx).await?.ok_or_else(missing)?;
    audit(&mut tx, &u, "user.rejected", &format!("{name} so‘rovi rad etildi")).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok": true})))
}

fn new_code() -> String {
    const A: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    Uuid::new_v4().as_bytes()[..6].iter().map(|b| A[(*b as usize) % A.len()] as char).collect()
}

async fn link_code(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    if u.role != "student" {
        return Err(forbidden());
    }
    for _ in 0..6 {
        let code: Option<String> = sqlx::query_scalar("SELECT link_code FROM users WHERE academy_id=$1 AND id=$2")
            .bind(u.academy_id).bind(u.id).fetch_one(&app.db).await?;
        if let Some(c) = code {
            return Ok(Json(json!({"code": c})));
        }
        let r = sqlx::query("UPDATE users SET link_code=$3 WHERE academy_id=$1 AND id=$2 AND link_code IS NULL")
            .bind(u.academy_id).bind(u.id).bind(new_code()).execute(&app.db).await;
        if let Err(sqlx::Error::Database(ref d)) = r {
            if d.is_unique_violation() { continue; }
        }
        r?;
    }
    Err(bad("Kod yaratib bo‘lmadi. Qayta urinib ko‘ring."))
}

#[derive(Deserialize)]
pub struct LinkChild {
    child_code: String,
}
/// Mavjud ota-ona yana bir farzandini kod bilan bog'laydi.
async fn link_child(State(app): State<App>, h: HeaderMap, Json(i): Json<LinkChild>) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    if u.role != "parent" {
        return Err(forbidden());
    }
    auth::limited(&app, format!("child-code:{}", u.id), 8)?;
    let sid: i64 = sqlx::query_scalar("SELECT id FROM users WHERE academy_id=$1 AND role='student' AND link_code=$2 AND archived=false")
        .bind(u.academy_id).bind(i.child_code.trim().to_uppercase()).fetch_optional(&app.db).await?
        .ok_or_else(|| bad("Bunday farzand kodi topilmadi."))?;
    sqlx::query("INSERT INTO parent_links(academy_id,parent_id,student_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(u.academy_id).bind(u.id).bind(sid).execute(&app.db).await?;
    Ok(Json(json!({"ok": true})))
}

/* ------------------------------ O'yin ------------------------------ */

#[derive(sqlx::FromRow)]
struct Row {
    id: i64,
    name: String,
    group_id: Option<i64>,
    subs: i64,
    score_sum: i64,
    best_score: i32,
    present: i64,
    late: i64,
    game_xp: i64,
    games: i64,
    words_best: i32,
}

/// Darajalar chegarasi: 0, 100, 300, 600, 1000, 1500 ... XP
pub fn level_of(xp: i64) -> (i64, i64, i64) {
    let mut lvl = 1;
    while 50 * lvl * (lvl + 1) <= xp {
        lvl += 1;
    }
    let start = 50 * (lvl - 1) * lvl;
    let next = 50 * lvl * (lvl + 1);
    (lvl, start, next)
}

fn stats(r: &Row) -> Value {
    let xp = r.subs * 10 + r.score_sum / 5 + r.present * 5 + r.late * 3 + r.game_xp;
    let (level, start, next) = level_of(xp);
    let badges = [
        ("first", "Birinchi qadam", "Birinchi topshiriqni yubordi", r.subs >= 1),
        ("worker", "Mehnatkash", "10 ta topshiriq yubordi", r.subs >= 10),
        ("star", "A’lochi", "90+ ball oldi", r.best_score >= 90),
        ("attend", "Doim darsda", "10 marta darsga keldi", r.present >= 10),
        ("words", "So‘z ustasi", "So‘z o‘yinida 20+ to‘g‘ri javob", r.words_best >= 20),
        ("gamer", "O‘yinchi", "10 marta o‘ynadi", r.games >= 10),
    ];
    json!({
        "id": r.id, "name": r.name, "group_id": r.group_id,
        "xp": xp, "level": level, "level_start": start, "level_next": next,
        "submissions": r.subs, "best_score": r.best_score, "present": r.present,
        "games": r.games, "words_best": r.words_best,
        "badges": badges.iter().map(|(k, t, d, e)| json!({"key": k, "title": t, "desc": d, "earned": e})).collect::<Vec<_>>(),
    })
}

async fn all_stats(app: &App, aid: i64) -> Result<Vec<Value>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.id,s.name,s.group_id,
          (SELECT count(*) FROM submissions x WHERE x.academy_id=$1 AND x.student_id=s.id) AS subs,
          (SELECT COALESCE(sum(score),0)::bigint FROM submissions x WHERE x.academy_id=$1 AND x.student_id=s.id) AS score_sum,
          (SELECT COALESCE(max(score),0) FROM submissions x WHERE x.academy_id=$1 AND x.student_id=s.id) AS best_score,
          (SELECT count(*) FROM attendance a WHERE a.academy_id=$1 AND a.student_id=s.id AND a.status='present') AS present,
          (SELECT count(*) FROM attendance a WHERE a.academy_id=$1 AND a.student_id=s.id AND a.status='late') AS late,
          (SELECT COALESCE(sum(xp),0)::bigint FROM game_scores g WHERE g.academy_id=$1 AND g.user_id=s.id) AS game_xp,
          (SELECT count(*) FROM game_scores g WHERE g.academy_id=$1 AND g.user_id=s.id) AS games,
          (SELECT COALESCE(max(score),0) FROM game_scores g WHERE g.academy_id=$1 AND g.user_id=s.id AND g.game='words') AS words_best
         FROM users s WHERE s.academy_id=$1 AND s.role='student' AND s.archived=false AND s.pending=false",
    )
    .bind(aid)
    .fetch_all(&app.db)
    .await?;
    let mut v: Vec<Value> = rows.iter().map(stats).collect();
    v.sort_by(|a, b| b["xp"].as_i64().cmp(&a["xp"].as_i64()));
    Ok(v)
}

async fn game(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    let all = all_stats(&app, u.academy_id).await?;
    let children: Vec<i64> = if u.role == "parent" {
        sqlx::query_scalar("SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$2")
            .bind(u.academy_id).bind(u.id).fetch_all(&app.db).await?
    } else {
        vec![]
    };
    let me = all.iter().find(|s| s["id"].as_i64() == Some(u.id)).cloned();
    let kids: Vec<Value> = all.iter().filter(|s| children.contains(&s["id"].as_i64().unwrap_or(0))).cloned().collect();
    // Reyting: o'quvchi o'z guruhini, ota-ona farzandi guruhini, xodimlar butun akademiyani ko'radi.
    let scope: Option<Option<i64>> = match u.role.as_str() {
        "student" => Some(u.group_id),
        "parent" => kids.first().map(|k| k["group_id"].as_i64()),
        _ => None,
    };
    let board: Vec<Value> = all
        .iter()
        .filter(|s| match scope {
            None => true,
            Some(g) => s["group_id"].as_i64() == g,
        })
        .take(20)
        .enumerate()
        .map(|(n, s)| json!({
            "rank": n + 1, "id": s["id"], "name": s["name"], "xp": s["xp"], "level": s["level"],
            "me": s["id"].as_i64() == Some(u.id) || children.contains(&s["id"].as_i64().unwrap_or(0)),
        }))
        .collect();
    let today_xp: i64 = sqlx::query_scalar("SELECT COALESCE(sum(xp),0)::bigint FROM game_scores WHERE academy_id=$1 AND user_id=$2 AND created_at>=date_trunc('day',now())")
        .bind(u.academy_id).bind(u.id).fetch_one(&app.db).await?;
    Ok(Json(json!({"me": me, "children": kids, "board": board, "today_game_xp": today_xp, "daily_game_cap": DAILY_CAP})))
}

const DAILY_CAP: i64 = 60;

#[derive(Deserialize)]
pub struct Score {
    game: String,
    score: i32,
}

async fn score(State(app): State<App>, h: HeaderMap, Json(i): Json<Score>) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    if i.game != "words" || !(0..=200).contains(&i.score) {
        return Err(bad("O‘yin natijasi noto‘g‘ri."));
    }
    if u.role != "student" {
        return Ok(Json(json!({"saved": false, "xp": 0})));
    }
    auth::limited(&app, format!("game:{}", u.id), 4)?;
    let mut tx = app.db.begin().await?;
    crate::v4::lock(&mut tx, u.academy_id).await?;
    let today: i64 = sqlx::query_scalar("SELECT COALESCE(sum(xp),0)::bigint FROM game_scores WHERE academy_id=$1 AND user_id=$2 AND created_at>=date_trunc('day',now())")
        .bind(u.academy_id).bind(u.id).fetch_one(&mut *tx).await?;
    let xp = (i.score.min(30) as i64).min((DAILY_CAP - today).max(0));
    sqlx::query("INSERT INTO game_scores(academy_id,user_id,game,score,xp) VALUES($1,$2,$3,$4,$5)")
        .bind(u.academy_id).bind(u.id).bind(&i.game).bind(i.score).bind(xp as i32).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"saved": true, "xp": xp, "capped": xp < i.score.min(30) as i64})))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn levels() {
        assert_eq!(level_of(0), (1, 0, 100));
        assert_eq!(level_of(99), (1, 0, 100));
        assert_eq!(level_of(100), (2, 100, 300));
        assert_eq!(level_of(650), (4, 600, 1000));
    }
    #[test]
    fn codes() {
        let c = new_code();
        assert_eq!(c.len(), 6);
        assert!(c.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()));
    }
}
