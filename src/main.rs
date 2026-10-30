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
    println!("EduCore V5.1: http://{address} | Kabinet: /app");
    let app = App::new(db, secure);
    let worker = tokio::spawn(educore::mail::worker(app.clone()));
    axum::serve(listener, router(app))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    worker.abort();
    Ok(())
}
