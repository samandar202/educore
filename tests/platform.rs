use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use educore::{migrate, router, App};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn app() -> App {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must name a dedicated PostgreSQL test database");
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    migrate(&db).await.unwrap();
    App::new(db, false)
}
async fn req(
    app: &App,
    method: &str,
    path: &str,
    cookie: &str,
    payload: Value,
) -> (StatusCode, Value, String) {
    let res = router(app.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("cookie", cookie)
                .header("content-type", "application/json")
                .header("x-educore", "1")
                .body(if payload.is_null() {
                    Body::empty()
                } else {
                    Body::from(payload.to_string())
                })
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .to_string();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"body":String::from_utf8_lossy(&bytes)}));
    (status, value, cookie)
}
async fn register(app: &App) -> (String, String, i64) {
    let slug = format!("test-{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    let (s,b,c)=req(app,"POST","/api/register","",json!({"slug":slug,"academy_name":"Test Academy","name":"Owner","email":"owner@example.com","password":"StrongTestPass123!"})).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    (slug, c, b["user"]["id"].as_i64().unwrap())
}
async fn login(app: &App, slug: &str, email: &str) -> String {
    let (s, b, c) = req(
        app,
        "POST",
        "/api/login",
        "",
        json!({"slug":slug,"email":email,"password":"StrongTestPass123!"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    c
}
#[tokio::test]
async fn password_hashes_are_salted_and_validated() {
    let a = educore::auth::hash("StrongTestPass123!".into())
        .await
        .unwrap();
    let b = educore::auth::hash("StrongTestPass123!".into())
        .await
        .unwrap();
    assert_ne!(a, b);
    assert!(a.starts_with("$argon2id$"));
    assert!(educore::auth::hash("short".into()).await.is_err());
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL; run cargo test -- --ignored --test-threads=1"]
async fn tenant_roles_full_learning_flow_and_revocation() {
    let app = app().await;
    let (slug, owner, oid) = register(&app).await;
    let (_, other, other_oid) = register(&app).await;
    let (s,_,_)=req(&app,"POST","/api/groups",&owner,json!({"name":"Invalid","level":"A1","audience":"child","teacher_id":other_oid,"schedule":""})).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let(s,g,_)=req(&app,"POST","/api/groups",&owner,json!({"name":"Kids A1","level":"A1","audience":"child","teacher_id":oid,"schedule":"Monday 16:00"})).await;
    assert_eq!(s, StatusCode::OK, "{g}");
    let gid = g["id"].as_i64().unwrap();
    let(s,student,_)=req(&app,"POST","/api/users",&owner,json!({"name":"Ali","email":"ali@example.com","password":"StrongTestPass123!","role":"student","group_id":gid})).await;
    assert_eq!(s, StatusCode::OK, "{student}");
    let sid = student["id"].as_i64().unwrap();
    let(s,_,_)=req(&app,"POST","/api/users",&owner,json!({"name":"Parent","email":"parent@example.com","password":"StrongTestPass123!","role":"parent","student_id":sid})).await;
    assert_eq!(s, StatusCode::OK);
    let(s,_,_)=req(&app,"POST","/api/users",&owner,json!({"name":"Unassigned Teacher","email":"teacher@example.com","password":"StrongTestPass123!","role":"teacher"})).await;
    assert_eq!(s, StatusCode::OK);
    let stu = login(&app, &slug, "ali@example.com").await;
    let parent = login(&app, &slug, "parent@example.com").await;
    let teacher = login(&app, &slug, "teacher@example.com").await;
    let(s,a,_)=req(&app,"POST","/api/assignments",&owner,json!({"group_id":gid,"title":"My family","instructions":"Write three sentences about your family.","skill":"writing"})).await;
    assert_eq!(s, StatusCode::OK, "{a}");
    let aid = a["id"].as_i64().unwrap();
    let path = format!("/api/assignments/{aid}/answer");
    assert_eq!(
        req(
            &app,
            "POST",
            &path,
            &parent,
            json!({"answer":"Not allowed"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(req(&app,"POST","/api/assignments",&teacher,json!({"group_id":gid,"title":"No access","instructions":"Cannot assign this group","skill":"writing"})).await.0,StatusCode::NOT_FOUND);
    assert_eq!(
        req(
            &app,
            "POST",
            &path,
            &stu,
            json!({"answer":"My family is kind."})
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, dash, _) = req(&app, "GET", "/api/bootstrap", &owner, Value::Null).await;
    let sub = &dash["submissions"][0];
    let review = format!("/api/submissions/{}/review", sub["id"]);
    assert_eq!(
        req(
            &app,
            "PATCH",
            &review,
            &other,
            json!({"score":90,"feedback":"Cross tenant","revision":1})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        req(
            &app,
            "POST",
            &path,
            &stu,
            json!({"answer":"My family is kind. We love reading."})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(
            &app,
            "PATCH",
            &review,
            &owner,
            json!({"score":90,"feedback":"Stale","revision":1})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        req(
            &app,
            "PATCH",
            &review,
            &owner,
            json!({"score":90,"feedback":"Excellent work!","revision":2})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(
            &app,
            "POST",
            &path,
            &stu,
            json!({"answer":"Changed after review"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (_, p, _) = req(&app, "GET", "/api/bootstrap", &parent, Value::Null).await;
    assert_eq!(p["submissions"][0]["score"], 90);
    assert_eq!(p["users"].as_array().unwrap().len(), 2);
    assert_eq!(p["audit"], json!([]));
    let (_, o, _) = req(&app, "GET", "/api/bootstrap", &other, Value::Null).await;
    assert_eq!(o["groups"], json!([]));
    assert_eq!(o["submissions"], json!([]));
    let (_, t, _) = req(&app, "GET", "/api/bootstrap", &teacher, Value::Null).await;
    assert_eq!(t["groups"], json!([]));
    assert_eq!(t["submissions"], json!([]));
    assert_eq!(
        req(
            &app,
            "POST",
            "/api/attendance",
            &owner,
            json!({"group_id":gid,"student_id":sid,"lesson_date":"2026-10-28","status":"present"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(
            &app,
            "POST",
            "/api/attendance",
            &other,
            json!({"group_id":gid,"student_id":sid,"lesson_date":"2026-10-28","status":"present"})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (_, d, _) = req(&app, "GET", "/api/bootstrap", &stu, Value::Null).await;
    assert_eq!(d["attendance"][0]["status"], "present");
    let(_,d,_)=req(&app,"POST",&format!("/api/public/{slug}"),"",json!({"name":"Applicant","phone":"+998901234567","audience":"adult","goal":"Learn","consent":true})).await;
    assert!(d.get("error").is_none());
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/users/{sid}/password"),
            &owner,
            json!({"password":"ChangedPassword123!"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(&app, "GET", "/api/bootstrap", &stu, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let res = router(app.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/logout")
                .header("cookie", &owner)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        req(&app, "POST", "/api/logout", &owner, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        req(&app, "GET", "/api/me", &owner, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    app.db.close().await;
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL"]
async fn uninitialized_import_account_cannot_login() {
    let app = app().await;
    let (slug, owner, _) = register(&app).await;
    let (_, d, _) = req(&app, "GET", "/api/me", &owner, Value::Null).await;
    sqlx::query("INSERT INTO users(academy_id,name,email,password_hash,role) VALUES($1,'Imported','import@example.com','','student')").bind(d["academy_id"].as_i64().unwrap()).execute(&app.db).await.unwrap();
    assert_eq!(
        req(
            &app,
            "POST",
            "/api/login",
            "",
            json!({"slug":slug,"email":"import@example.com","password":"dummy-password-value"})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    app.db.close().await;
}

async fn media_req(
    app: &App,
    method: &str,
    path: &str,
    cookie: &str,
    bytes: Vec<u8>,
    purpose: &str,
    range: &str,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("cookie", cookie)
        .header("x-educore", "1")
        .header("content-type", "audio/wav")
        .header("x-media-purpose", purpose);
    if !range.is_empty() {
        builder = builder.header("range", range)
    }
    let res = router(app.clone())
        .oneshot(builder.body(Body::from(bytes)).unwrap())
        .await
        .unwrap();
    let status = res.status();
    (
        status,
        res.into_body().collect().await.unwrap().to_bytes().to_vec(),
    )
}
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL; dedicated test database only"]
async fn v4_audio_courses_calendar_history_and_recovery() {
    let app = app().await;
    let (slug, c, owner) = register(&app).await;
    let (_, foreign, _) = register(&app).await;
    let media_dir = std::env::temp_dir().join(format!("educore-media-{}", uuid::Uuid::new_v4()));
    let mail_dir = std::env::temp_dir().join(format!("educore-mail-{}", uuid::Uuid::new_v4()));
    std::env::set_var("MEDIA_DIR", &media_dir);
    std::env::set_var("MAIL_MODE", "local");
    std::env::set_var("MAIL_DIR", &mail_dir);
    std::env::set_var("MEDIA_QUOTA_MB", "1");
    let(s,g,_)=req(&app,"POST","/api/groups",&c,json!({"name":"V4 Group","level":"A1","audience":"child","teacher_id":owner,"schedule":"Monday"})).await;
    assert_eq!(s, StatusCode::OK, "{g}");
    let gid = g["id"].as_i64().unwrap();
    let(s,stu,_)=req(&app,"POST","/api/users",&c,json!({"name":"Audio Student","email":"audio@example.com","password":"StrongTestPass123!","role":"student","group_id":gid})).await;
    assert_eq!(s, StatusCode::OK);
    let sid = stu["id"].as_i64().unwrap();
    let student = login(&app, &slug, "audio@example.com").await;
    req(&app,"POST","/api/users",&c,json!({"name":"Other Student","email":"other@example.com","password":"StrongTestPass123!","role":"student","group_id":gid})).await;
    let other = login(&app, &slug, "other@example.com").await;
    req(&app,"POST","/api/users",&c,json!({"name":"Audio Parent","email":"parent@example.com","password":"StrongTestPass123!","role":"parent","student_id":sid})).await;
    let parent = login(&app, &slug, "parent@example.com").await;
    let (s, course, _) = req(
        &app,
        "POST",
        "/api/courses",
        &c,
        json!({"title":"First conversations","description":"Speak daily","level":"A1"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{course}");
    let cid = course["id"].as_i64().unwrap();
    let course_edit = json!({"title":"Changed","description":"No","level":"A1","archived":false});
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/courses/{cid}"),
            &foreign,
            course_edit.clone()
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/courses/{cid}"),
            &student,
            course_edit
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let(s,lesson,_)=req(&app,"POST","/api/lessons",&c,json!({"course_id":cid,"module_title":"Introductions","title":"Say hello","content":"Talk about your day","resource_url":"https://example.com/lesson","position":1})).await;
    assert_eq!(s, StatusCode::OK, "{lesson}");
    let lid = lesson["id"].as_i64().unwrap();
    let group = json!({"name":"V4 Group","level":"A1","audience":"child","teacher_id":owner,"schedule":"Monday","course_id":cid,"archived":false});
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/groups/{gid}"),
            &c,
            group.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    let(s,task,_)=req(&app,"POST","/api/assignments",&c,json!({"group_id":gid,"title":"My day","instructions":"Speak about your typical day.","skill":"speaking","answer_mode":"audio","lesson_id":lid,"due_date":"2027-01-01"})).await;
    assert_eq!(s, StatusCode::OK, "{task}");
    let tid = task["id"].as_i64().unwrap();
    let state = req(&app, "GET", "/api/v4", &student, Value::Null).await;
    assert_eq!(state.0, StatusCode::OK, "{}", state.1);
    assert_eq!(state.1["courses"].as_array().unwrap().len(), 1);
    assert!(!state.1["notifications"].as_array().unwrap().is_empty());
    let event = json!({"group_id":gid,"title":"Speaking session","starts_at":"2027-01-01T10:00:00+05:00","ends_at":"2027-01-01T11:00:00+05:00"});
    let (s, ev, _) = req(&app, "POST", "/api/calendar", &c, event.clone()).await;
    assert_eq!(s, StatusCode::OK, "{ev}");
    assert_eq!(
        req(&app, "POST", "/api/calendar", &c, event.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    let mut adjacent = event.clone();
    adjacent["starts_at"] = json!("2027-01-01T11:00:00+05:00");
    adjacent["ends_at"] = json!("2027-01-01T12:00:00+05:00");
    assert_eq!(
        req(&app, "POST", "/api/calendar", &c, adjacent).await.0,
        StatusCode::OK
    );
    let mut wav = vec![0u8; 40000];
    wav[0..4].copy_from_slice(b"RIFF");
    wav[8..12].copy_from_slice(b"WAVE");
    assert_eq!(
        media_req(
            &app,
            "POST",
            "/api/media",
            &student,
            b"<html>invalid</html>".to_vec(),
            "answer",
            ""
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        media_req(
            &app,
            "POST",
            "/api/media",
            &parent,
            wav.clone(),
            "answer",
            ""
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (s, b) = media_req(
        &app,
        "POST",
        "/api/media",
        &student,
        wav.clone(),
        "answer",
        "",
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let audio: Value = serde_json::from_slice(&b).unwrap();
    let aid = audio["id"].as_str().unwrap();
    assert_eq!(
        media_req(
            &app,
            "GET",
            &format!("/api/media/{aid}"),
            &c,
            vec![],
            "",
            ""
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/assignments/{tid}/answer"),
            &other,
            json!({"answer":"","audio_id":aid})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/assignments/{tid}/answer"),
            &student,
            json!({"answer":"First attempt","audio_id":aid})
        )
        .await
        .0,
        StatusCode::OK
    );
    for cookie in [&c, &parent, &student] {
        let (s, b) = media_req(
            &app,
            "GET",
            &format!("/api/media/{aid}"),
            cookie,
            vec![],
            "",
            "bytes=0-11",
        )
        .await;
        assert_eq!(s, StatusCode::PARTIAL_CONTENT);
        assert_eq!(b.len(), 12);
    }
    for cookie in [&other, &foreign] {
        assert_eq!(
            media_req(
                &app,
                "GET",
                &format!("/api/media/{aid}"),
                cookie,
                vec![],
                "",
                ""
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
    let boot = req(&app, "GET", "/api/bootstrap", &c, Value::Null).await.1;
    let sub = boot["submissions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["assignment_id"] == tid)
        .unwrap();
    let subid = sub["id"].as_i64().unwrap();
    let (s, b) = media_req(&app, "POST", "/api/media", &c, wav.clone(), "feedback", "").await;
    assert_eq!(s, StatusCode::OK);
    let feedback: Value = serde_json::from_slice(&b).unwrap();
    let review = json!({"score":62,"feedback":"Try adding details","feedback_audio_id":feedback["id"],"revision":1,"retry_allowed":true});
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/submissions/{subid}/review"),
            &c,
            review.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/assignments/{tid}/answer"),
            &student,
            json!({"answer":"Second attempt","audio_id":aid})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/submissions/{subid}/review"),
            &c,
            review
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let state = req(&app, "GET", "/api/v4", &parent, Value::Null).await.1;
    assert_eq!(state["history"][0]["answer"], "First attempt");
    assert_eq!(state["history"][0]["score"], 62);
    assert_eq!(state["history"][0]["feedback_audio_id"], feedback["id"]);
    let notice = state["notifications"][0]["id"].as_i64().unwrap();
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/notifications/{notice}/read"),
            &other,
            json!({})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/notifications/{notice}/read"),
            &parent,
            json!({})
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut large = wav.clone();
    large.resize(1024 * 1024, 0);
    assert_eq!(
        media_req(&app, "POST", "/api/media", &student, large, "answer", "")
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    // Overlap is rejected even across two groups taught by the same teacher.
    let (_,g2,_)=req(&app,"POST","/api/groups",&c,json!({"name":"Second group","level":"A2","audience":"adult","teacher_id":owner,"schedule":""})).await;
    let gid2 = g2["id"].as_i64().unwrap();
    let mut collision = event.clone();
    collision["group_id"] = json!(gid2);
    assert_eq!(
        req(&app, "POST", "/api/calendar", &c, collision).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(req(&app,"POST","/api/assignments",&c,json!({"group_id":gid2,"title":"Wrong lesson","instructions":"Read this lesson.","skill":"reading","lesson_id":lid})).await.0,StatusCode::BAD_REQUEST);
    let edit_task = json!({"title":"My day","instructions":"Speak about your typical day.","due_date":"2027-01-01","archived":true,"lesson_id":lid});
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/assignments/{tid}"),
            &c,
            edit_task
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/assignments/{tid}/answer"),
            &student,
            json!({"answer":"third","audio_id":aid})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // Group transfer preserves the learner's history and old evidence access.
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/users/{sid}"),
            &c,
            json!({"name":"Audio Student","group_id":gid2,"archived":false})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(&app, "GET", "/api/v4", &student, Value::Null).await.1["history"][0]["score"],
        62
    );
    assert_eq!(
        media_req(
            &app,
            "GET",
            &format!("/api/media/{aid}"),
            &student,
            vec![],
            "",
            ""
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/users/{sid}"),
            &c,
            json!({"name":"Audio Student","group_id":gid,"archived":false})
        )
        .await
        .0,
        StatusCode::OK
    );
    // Reminder worker is idempotent and uses Tashkent's next calendar date.
    let tomorrow: String = sqlx::query_scalar(
        "SELECT to_char((now() AT TIME ZONE 'Asia/Tashkent')::date+1,'YYYY-MM-DD')",
    )
    .fetch_one(&app.db)
    .await
    .unwrap();
    req(&app,"POST","/api/assignments",&c,json!({"group_id":gid,"title":"Reminder test","instructions":"Write about tomorrow.","skill":"writing","due_date":tomorrow})).await;
    educore::mail::reminders(&app).await.unwrap();
    educore::mail::reminders(&app).await.unwrap();
    let reminders: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM notifications WHERE user_id=$1 AND dedupe LIKE 'due:%'",
    )
    .bind(sid)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(reminders, 1);
    // A reset token is hashed in the database, expires and is consumed once.
    assert_eq!(
        req(
            &app,
            "POST",
            "/api/password/request",
            "",
            json!({"slug":slug,"email":"audio@example.com"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let body: String = sqlx::query_scalar(
        "SELECT body FROM mail_jobs WHERE recipient='audio@example.com' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&app.db)
    .await
    .unwrap();
    let token = body
        .split("#reset=")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    assert!(educore::mail::process_one(&app).await.unwrap());
    assert!(std::fs::read_dir(&mail_dir).unwrap().next().is_some());
    let reset = json!({"token":token,"password":"ChangedPass123!"});
    assert_eq!(
        req(&app, "POST", "/api/password/confirm", "", reset.clone())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        req(&app, "POST", "/api/password/confirm", "", reset)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        req(&app, "GET", "/api/me", &student, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (s, _, student) = req(
        &app,
        "POST",
        "/api/login",
        "",
        json!({"slug":slug,"email":"audio@example.com","password":"ChangedPass123!"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let expired = "a".repeat(64);
    sqlx::query("INSERT INTO account_tokens(token_hash,user_id,purpose,expires_at) VALUES($1,$2,'verify',now()-interval '1 minute')").bind(educore::auth::digest(&expired)).bind(sid).execute(&app.db).await.unwrap();
    assert_eq!(
        req(
            &app,
            "POST",
            "/api/email/confirm",
            "",
            json!({"token":expired})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        req(&app, "POST", "/api/email/request", &student, json!({}))
            .await
            .0,
        StatusCode::OK
    );
    let body: String = sqlx::query_scalar(
        "SELECT body FROM mail_jobs WHERE recipient='audio@example.com' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&app.db)
    .await
    .unwrap();
    let token = body
        .split("#verify=")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    assert_eq!(
        req(
            &app,
            "POST",
            "/api/email/confirm",
            "",
            json!({"token":token})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(&app, "GET", "/api/v4", &student, Value::Null).await.1["profile"]["verified"],
        true
    );
    // Archive revokes existing sessions and blocks new login; historical records survive.
    assert_eq!(
        req(
            &app,
            "PATCH",
            &format!("/api/users/{sid}"),
            &c,
            json!({"name":"Audio Student","group_id":gid,"archived":true})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(&app, "GET", "/api/me", &student, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        req(
            &app,
            "POST",
            "/api/login",
            "",
            json!({"slug":slug,"email":"audio@example.com","password":"ChangedPass123!"})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let mut archived = group;
    archived["archived"] = json!(true);
    assert_eq!(
        req(&app, "PATCH", &format!("/api/groups/{gid}"), &c, archived)
            .await
            .0,
        StatusCode::OK
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM calendar_events WHERE group_id=$1 AND cancelled=false",
    )
    .bind(gid)
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(n, 0);
    std::fs::remove_dir_all(media_dir).unwrap();
    std::fs::remove_dir_all(mail_dir).unwrap();
}

#[tokio::test]
#[ignore = "requires dedicated TEST_DATABASE_URL with anon/authenticated roles"]
async fn cloud_rls_and_registration_closed() {
    let app = app().await;
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relkind='r' AND c.relrowsecurity").fetch_one(&app.db).await.unwrap();
    assert_eq!(n, 23);
    for role in ["anon", "authenticated"] {
        let allowed: bool =
            sqlx::query_scalar("SELECT has_table_privilege($1,'public.users','SELECT')")
                .bind(role)
                .fetch_one(&app.db)
                .await
                .unwrap();
        assert!(!allowed);
    }
    let mut tx = app.db.begin().await.unwrap();
    sqlx::query("GRANT SELECT ON public.users TO anon")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SET LOCAL ROLE anon")
        .execute(&mut *tx)
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM public.users")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 0);
    tx.rollback().await.unwrap();
    std::env::set_var("REGISTRATION_ENABLED", "false");
    let (status,_,_)=req(&app,"POST","/api/register","",json!({"academy_name":"Blocked academy","slug":"blocked-cloud","name":"Owner","email":"owner@example.com","password":"strong-test-password"})).await;
    std::env::remove_var("REGISTRATION_ENABLED");
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires dedicated TEST_DATABASE_URL"]
async fn v5_finance_settings_ai_permissions_and_reversal() {
    let app = app().await;
    let (slug, owner, oid) = register(&app).await;
    let (_, other, _) = register(&app).await;
    let(s,g,_)=req(&app,"POST","/api/groups",&owner,json!({"name":"V5 A1","level":"A1","audience":"child","teacher_id":oid,"schedule":"Monday"})).await;
    assert_eq!(s, StatusCode::OK, "{g}");
    let gid = g["id"].as_i64().unwrap();
    let mut sid = 0;
    for (name, email, role) in [
        ("=FORMULA()", "v5s@test.invalid", "student"),
        ("Other student", "v5s2@test.invalid", "student"),
        ("V5 teacher", "v5t@test.invalid", "teacher"),
    ] {
        let(s,b,_)=req(&app,"POST","/api/users",&owner,json!({"name":name,"email":email,"role":role,"password":"StrongTestPass123!","group_id":gid})).await;
        assert_eq!(s, StatusCode::OK, "{b}");
        if email == "v5s@test.invalid" {
            sid = b["id"].as_i64().unwrap();
        }
    }
    assert_eq!(req(&app,"POST","/api/users",&owner,json!({"name":"V5 parent","email":"v5p@test.invalid","role":"parent","student_id":sid,"password":"StrongTestPass123!"})).await.0,StatusCode::OK);
    let stu = login(&app, &slug, "v5s@test.invalid").await;
    let parent = login(&app, &slug, "v5p@test.invalid").await;
    let teacher = login(&app, &slug, "v5t@test.invalid").await;
    let stu2 = login(&app, &slug, "v5s2@test.invalid").await;
    let inv = json!({"student_id":sid,"title":"English November","period":"2026-11","amount":500000,"due_date":"2026-11-05"});
    assert_eq!(
        req(&app, "POST", "/api/invoices", &teacher, inv.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        req(&app, "POST", "/api/invoices", &other, inv.clone())
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (s, b, _) = req(&app, "POST", "/api/invoices", &owner, inv.clone()).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let id = b["id"].as_i64().unwrap();
    assert_eq!(
        req(&app, "POST", "/api/invoices", &owner, inv).await.0,
        StatusCode::CONFLICT
    );
    let pid = uuid::Uuid::new_v4().simple().to_string();
    let p = json!({"request_id":pid,"amount":200000,"method":"cash","note":"Receipt 1"});
    let path = format!("/api/invoices/{id}/payments");
    assert_eq!(
        req(&app, "POST", &path, &owner, p.clone()).await.0,
        StatusCode::OK
    );
    let (s, b, _) = req(&app, "POST", &path, &owner, p).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["duplicate"], true);
    assert_eq!(req(&app,"POST",&path,&owner,json!({"request_id":uuid::Uuid::new_v4().simple().to_string(),"amount":400000,"method":"cash","note":"overpay"})).await.0,StatusCode::BAD_REQUEST);
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/invoices/{id}/void"),
            &owner,
            json!({})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    for c in [&stu, &parent] {
        let (s, b, _) = req(&app, "GET", "/api/v5", c, Value::Null).await;
        assert_eq!(s, StatusCode::OK, "{b}");
        assert_eq!(b["invoices"][0]["paid"], 200000);
        assert_eq!(b["drafts"], json!([]));
    }
    for c in [&stu2, &teacher, &other] {
        let (s, b, _) = req(&app, "GET", "/api/v5", c, Value::Null).await;
        assert_eq!(s, StatusCode::OK, "{b}");
        assert_eq!(b["invoices"], json!([]));
    }
    let (s, b, _) = req(&app, "GET", "/api/export/finance.csv", &owner, Value::Null).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b["body"].as_str().unwrap().contains("'=FORMULA()"));
    assert_eq!(
        req(&app, "GET", "/api/export/finance.csv", &stu, Value::Null)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        req(
            &app,
            "PATCH",
            "/api/academy",
            &owner,
            json!({"name":"V5 School","description":"New description","contact":"@school"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        req(
            &app,
            "PATCH",
            "/api/academy",
            &teacher,
            json!({"name":"Hacked","description":"","contact":""})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let dp = uuid::Uuid::new_v4().simple().to_string();
    let aid: i64 = sqlx::query_scalar("SELECT academy_id FROM users WHERE id=$1")
        .bind(oid)
        .fetch_one(&app.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ai_drafts(id,academy_id,user_id,kind,input_hash,topic,level,audience,duration,status,title,student_content,teacher_notes) VALUES($1,$2,$3,'quiz','test','Test topic','A1','child',45,'ready','Private quiz','Student question','PRIVATE ANSWER KEY')").bind(&dp).bind(aid).bind(oid).execute(&app.db).await.unwrap();
    let url = format!("/api/ai/drafts/{dp}");
    assert_eq!(
        req(&app, "GET", &url, &owner, Value::Null).await.0,
        StatusCode::OK
    );
    for c in [&teacher, &other] {
        assert_eq!(
            req(&app, "GET", &url, c, Value::Null).await.0,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        req(&app, "GET", &url, &stu, Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    let ai = json!({"request_id":uuid::Uuid::new_v4().simple().to_string(),"kind":"quiz","topic":"Present Simple","level":"A1","audience":"child","duration":45});
    assert_eq!(
        req(&app, "POST", "/api/ai/generate", &stu, ai.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    std::env::remove_var("GROQ_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");
    let old_provider = std::env::var("AI_PROVIDER").ok();
    let old_model = std::env::var("AI_MODEL").ok();
    std::env::set_var("AI_PROVIDER", "anthropic");
    std::env::set_var("AI_MODEL", "claude-opus-5-5");
    let (status, bootstrap, _) = req(&app, "GET", "/api/v5", &owner, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bootstrap["ai"]["provider"], "anthropic");
    assert_eq!(bootstrap["ai"]["model"], "claude-opus-5-5");
    assert_eq!(bootstrap["ai"]["enabled"], false);
    assert!(bootstrap["ai"]["setup_message"]
        .as_str()
        .unwrap()
        .contains("ANTHROPIC_API_KEY"));
    assert_eq!(
        req(&app, "POST", "/api/ai/generate", &owner, ai).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    for (name, value) in [("AI_PROVIDER", old_provider), ("AI_MODEL", old_model)] {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/payments/{pid}/reverse"),
            &owner,
            json!({"reason":"Correcting duplicate receipt"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, b, _) = req(&app, "GET", "/api/v5", &owner, Value::Null).await;
    assert_eq!(b["invoices"][0]["paid"], 0);
    assert!(b["payments"][0]["voided_at"].is_string());
    assert_eq!(
        req(
            &app,
            "POST",
            &format!("/api/invoices/{id}/void"),
            &owner,
            json!({})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(req(&app,"POST",&path,&owner,json!({"request_id":uuid::Uuid::new_v4().simple().to_string(),"amount":1,"method":"cash","note":""})).await.0,StatusCode::BAD_REQUEST);
    let (status,body,_) = req(&app,"POST","/api/invoices",&owner,json!({"student_id":sid,"title":"Corrected November invoice","period":"2026-11","amount":450000,"due_date":"2026-11-10"})).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "A voided invoice must allow a corrected one: {body}"
    );
}
