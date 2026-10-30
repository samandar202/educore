//! Explicit one-time read-only V2 SQLite -> PostgreSQL transfer.
use sqlx::{PgPool, Row};
use std::collections::HashMap;
pub async fn run(pg: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("IMPORT_V2_PATH")
        .map_err(|_| "IMPORT_V2_PATH: eski academy.db yo‘lini kiriting")?;
    let slug = std::env::var("IMPORT_ACADEMY_SLUG").map_err(|_| "IMPORT_ACADEMY_SLUG kerak")?;
    if !std::path::Path::new(&path).is_file() {
        return Err("SQLite fayli topilmadi. Import to‘xtadi.".into());
    }
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .read_only(true)
        .create_if_missing(false);
    let sqlite = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let mut source = sqlite.begin().await?;
    let mut tx = pg.begin().await?;
    let aid: i64 = sqlx::query_scalar("SELECT id FROM academies WHERE slug=$1 FOR UPDATE")
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await?;
    let imported: i64 =
        sqlx::query_scalar("SELECT count(*) FROM legacy_imports WHERE academy_id=$1")
            .bind(aid)
            .fetch_one(&mut *tx)
            .await?;
    if imported > 0 {
        return Err(
            "Bu akademiyaga V2 allaqachon import qilingan. Takrorlash bekor qilindi.".into(),
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM study_groups WHERE academy_id=$1")
        .bind(aid)
        .fetch_one(&mut *tx)
        .await?;
    if count > 0 {
        return Err("Import uchun yangi, guruhlari bo‘sh akademiya yarating.".into());
    }
    let owner: i64 = sqlx::query_scalar(
        "SELECT id FROM users WHERE academy_id=$1 AND role='owner' ORDER BY id LIMIT 1",
    )
    .bind(aid)
    .fetch_one(&mut *tx)
    .await?;
    let mut groups = HashMap::new();
    let mut students = HashMap::new();
    let mut assignments = HashMap::new();
    for r in sqlx::query("SELECT * FROM study_groups ORDER BY id")
        .fetch_all(&mut *source)
        .await?
    {
        let old: i64 = r.try_get("id")?;
        let id:i64=sqlx::query_scalar("INSERT INTO study_groups(academy_id,name,level,audience,teacher_id) VALUES($1,$2,'A1',$3,$4) RETURNING id").bind(aid).bind(r.try_get::<String,_>("name")?).bind(r.try_get::<String,_>("audience")?).bind(owner).fetch_one(&mut *tx).await?;
        groups.insert(old, id);
    }
    for r in sqlx::query("SELECT * FROM students ORDER BY id")
        .fetch_all(&mut *source)
        .await?
    {
        let old: i64 = r.try_get("id")?;
        let gid: i64 = r.try_get("group_id")?;
        let id:i64=sqlx::query_scalar("INSERT INTO users(academy_id,name,email,password_hash,role,group_id) VALUES($1,$2,$3,'','student',$4) RETURNING id").bind(aid).bind(r.try_get::<String,_>("name")?).bind(format!("student-{old}@import.local")).bind(*groups.get(&gid).ok_or("V2 guruh bog‘lanishi buzilgan")?).fetch_one(&mut *tx).await?;
        students.insert(old, id);
    }
    for r in sqlx::query("SELECT * FROM assignments ORDER BY id")
        .fetch_all(&mut *source)
        .await?
    {
        let old: i64 = r.try_get("id")?;
        let gid: i64 = r.try_get("group_id")?;
        let id:i64=sqlx::query_scalar("INSERT INTO assignments(academy_id,group_id,title,instructions,skill,created_by,created_at) VALUES($1,$2,$3,$4,'writing',$5,($6::timestamp AT TIME ZONE 'UTC')) RETURNING id").bind(aid).bind(*groups.get(&gid).ok_or("Guruh yo‘q")?).bind(r.try_get::<String,_>("title")?).bind(r.try_get::<String,_>("instructions")?).bind(owner).bind(r.try_get::<String,_>("created_at")?).fetch_one(&mut *tx).await?;
        assignments.insert(old, id);
    }
    for r in sqlx::query("SELECT * FROM submissions ORDER BY id")
        .fetch_all(&mut *source)
        .await?
    {
        let aid_old: i64 = r.try_get("assignment_id")?;
        let sid: i64 = r.try_get("student_id")?;
        sqlx::query("INSERT INTO submissions(academy_id,assignment_id,student_id,answer,revision,score,feedback,submitted_at,reviewed_at) VALUES($1,$2,$3,$4,$5,$6,$7,($8::timestamp AT TIME ZONE 'UTC'),($9::timestamp AT TIME ZONE 'UTC'))")
   .bind(aid).bind(*assignments.get(&aid_old).ok_or("Topshiriq yo‘q")?).bind(*students.get(&sid).ok_or("O‘quvchi yo‘q")?).bind(r.try_get::<String,_>("answer")?).bind(r.try_get::<i32,_>("revision")?).bind(r.try_get::<Option<i32>,_>("score")?).bind(r.try_get::<Option<String>,_>("feedback")?).bind(r.try_get::<String,_>("submitted_at")?).bind(r.try_get::<Option<String>,_>("reviewed_at")?).execute(&mut *tx).await?;
    }
    for r in sqlx::query("SELECT * FROM leads ORDER BY id")
        .fetch_all(&mut *source)
        .await?
    {
        sqlx::query("INSERT INTO leads(academy_id,name,phone,audience,goal,status,created_at) VALUES($1,$2,$3,$4,$5,$6,($7::timestamp AT TIME ZONE 'UTC'))").bind(aid).bind(r.try_get::<String,_>("name")?).bind(r.try_get::<String,_>("phone")?).bind(r.try_get::<String,_>("audience")?).bind(r.try_get::<String,_>("goal")?).bind(r.try_get::<String,_>("status")?).bind(r.try_get::<String,_>("created_at")?).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO legacy_imports(academy_id) VALUES($1)")
        .bind(aid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO audit_log(academy_id,actor_id,action,detail) VALUES($1,$2,'legacy.imported','V2 ma’lumotlari ko‘chirildi')").bind(aid).bind(owner).execute(&mut *tx).await?;
    tx.commit().await?;
    source.rollback().await?;
    sqlite.close().await;
    println!("Import tugadi: {} guruh, {} o‘quvchi, {} topshiriq. Eski fayl o‘zgarmadi. O‘quvchi loginlari student-ID@import.local; rahbar panelida parollarni belgilang. Eski kirish kodlari V3da ishlamaydi. Ota-ona hisoblarini alohida yarating.",groups.len(),students.len(),assignments.len());
    Ok(())
}
