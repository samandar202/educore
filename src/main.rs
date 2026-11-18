use educore::{migrate, router, App};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL kerak. .env.example faylidan .env yarating.")?;
    if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
        return Err(
            "V4 PostgreSQL ishlatadi. DATABASE_URL SQLite bo‘lmasin; README ni ko‘ring.".into(),
        );
    }
    let pool_size: u32 = std::env::var("DB_POOL_SIZE")
        .unwrap_or_else(|_| "10".into())
        .parse()?;
    if !(1..=100).contains(&pool_size) {
        return Err("DB_POOL_SIZE 1–100 oralig‘ida bo‘lsin.".into());
    }
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(pool_size)
        .connect(&url)
        .await?;
    migrate(&db).await?;
    if std::env::args().any(|s| s == "--import-v2") {
        educore::import::run(&db).await?;
        return Ok(());
    }
    educore::storage::validate_startup()
        .await
        .map_err(|e| e.1)?;
    let address = std::env::var("BIND_ADDR").unwrap_or_else(|_| {
        std::env::var("PORT")
            .map(|p| format!("0.0.0.0:{p}"))
            .unwrap_or_else(|_| "127.0.0.1:3000".into())
    });
    let secure = std::env::var("COOKIE_SECURE").unwrap_or_default() == "true";
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("EduCore V6: http://{address} | Kabinet: /app");
    let db_keep = db.clone();
    let app = App::new(db, secure);
    let worker = tokio::spawn(educore::mail::worker(app.clone()));
    // Keep-alive: Render Free 15 daqiqa tashrifsiz uxlaydi, Supabase Free esa
    // faolsiz loyihani pauza qiladi. Har 10 daqiqada o'z /health manzilimizga
    // tashqi HTTPS so'rov yuboramiz va bazaga SELECT 1 qilamiz.
    // KEEP_ALIVE=false bilan o'chiriladi; KEEP_ALIVE_URL bilan manzil beriladi.
    let keep_alive_url = std::env::var("KEEP_ALIVE_URL")
        .or_else(|_| std::env::var("RENDER_EXTERNAL_URL"))
        .ok()
        .filter(|u| u.starts_with("https://"));
    let keeper = if std::env::var("KEEP_ALIVE").unwrap_or_default() != "false" {
        let db = db_keep.clone();
        Some(tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default();
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(600));
            tick.tick().await;
            loop {
                tick.tick().await;
                if let Err(e) = sqlx::query("SELECT 1").execute(&db).await {
                    eprintln!("keep-alive DB: {e}");
                }
                if let Some(url) = &keep_alive_url {
                    let target = format!("{}/health", url.trim_end_matches('/'));
                    if let Err(e) = client.get(&target).send().await {
                        eprintln!("keep-alive HTTP: {e}");
                    }
                }
            }
        }))
    } else {
        None
    };
    axum::serve(listener, router(app))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    worker.abort();
    if let Some(k) = keeper {
        k.abort();
    }
    Ok(())
}
