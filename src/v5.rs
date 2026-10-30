use crate::{auth::current, *};
use axum::extract::Path;
use serde::Deserialize;
pub fn routes() -> Router<App> {
    Router::new()
        .route("/api/v5", get(bootstrap))
        .route("/api/academy", patch(settings))
        .route("/api/invoices", post(invoice))
        .route("/api/invoices/{id}/payments", post(payment))
        .route("/api/invoices/{id}/void", post(void_invoice))
        .route("/api/payments/{id}/reverse", post(reverse_payment))
        .route("/api/export/finance.csv", get(finance_csv))
        .merge(crate::ai::routes())
}
async fn bootstrap(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    let invoices:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.id DESC),'[]') FROM (SELECT i.*,s.name AS student_name,COALESCE((SELECT sum(p.amount) FROM payments p WHERE p.academy_id=i.academy_id AND p.invoice_id=i.id AND p.voided_at IS NULL),0)::bigint AS paid FROM invoices i JOIN users s ON s.id=i.student_id AND s.academy_id=i.academy_id WHERE i.academy_id=$1 AND ($2='owner' OR i.student_id=$3 OR ($2='parent' AND i.student_id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$3))) ORDER BY i.id DESC LIMIT 1000) x").bind(u.academy_id).bind(&u.role).bind(u.id).fetch_one(&app.db).await?;
    let payments:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(to_jsonb(x) ORDER BY x.created_at DESC),'[]') FROM (SELECT p.* FROM payments p JOIN invoices i ON i.id=p.invoice_id AND i.academy_id=p.academy_id WHERE p.academy_id=$1 AND ($2='owner' OR i.student_id=$3 OR ($2='parent' AND i.student_id IN(SELECT student_id FROM parent_links WHERE academy_id=$1 AND parent_id=$3))) ORDER BY p.created_at DESC LIMIT 2000) x").bind(u.academy_id).bind(&u.role).bind(u.id).fetch_one(&app.db).await?;
    let drafts = if u.staff().is_ok() {
        crate::ai::list(&app, &u).await?
    } else {
        json!([])
    };
    let used: i64 = if u.staff().is_ok() {
        sqlx::query_scalar("SELECT count(*) FROM ai_drafts WHERE academy_id=$1 AND user_id=$2 AND created_at >= date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC'").bind(u.academy_id).bind(u.id).fetch_one(&app.db).await?
    } else {
        0
    };
    let mut ai = crate::ai::status();
    ai["used"] = json!(used);
    ai["daily_limit"] = json!(crate::ai::daily_limit());
    Ok(Json(
        json!({"invoices":invoices,"payments":payments,"drafts":drafts,"ai":ai,"setup":if u.role=="owner"{json!({"storage":crate::storage::backend().unwrap_or_default(),"secure_cookie":app.secure_cookie,"mail_enabled":crate::mail::enabled(),"registration_open":std::env::var("REGISTRATION_ENABLED").unwrap_or_else(|_|"true".into())=="true"})}else{Value::Null}}),
    ))
}
#[derive(Deserialize)]
struct AcademyInput {
    name: String,
    description: String,
    contact: String,
}
async fn settings(
    State(app): State<App>,
    h: HeaderMap,
    Json(i): Json<AcademyInput>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    if !valid_text(&i.name, 2, 80)
        || !valid_text(&i.description, 0, 1000)
        || !valid_text(&i.contact, 0, 120)
    {
        return Err(bad("Akademiya maydonlarini tekshiring."));
    }
    let mut tx = app.db.begin().await?;
    sqlx::query("UPDATE academies SET name=$1,description=$2,contact=$3 WHERE id=$4")
        .bind(i.name.trim())
        .bind(i.description.trim())
        .bind(i.contact.trim())
        .bind(u.academy_id)
        .execute(&mut *tx)
        .await?;
    audit(
        &mut tx,
        &u,
        "academy.updated",
        "Akademiya ma’lumotlari yangilandi",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
struct InvoiceInput {
    student_id: i64,
    title: String,
    period: String,
    amount: i64,
    due_date: String,
}
async fn invoice(
    State(app): State<App>,
    h: HeaderMap,
    Json(i): Json<InvoiceInput>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    if !valid_text(&i.title, 2, 120)
        || !(1000..=100000000).contains(&i.amount)
        || !crate::platform::date_valid(&i.due_date)
        || i.period.len() != 7
        || !crate::platform::date_valid(&format!("{}-01", i.period))
    {
        return Err(bad("Summa, oy va muddatni tekshiring."));
    }
    let mut tx = app.db.begin().await?;
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM users WHERE academy_id=$1 AND id=$2 AND role='student' AND archived=false").bind(u.academy_id).bind(i.student_id).fetch_one(&mut *tx).await?;
    if n == 0 {
        return Err(missing());
    }
    let id:i64=sqlx::query_scalar("INSERT INTO invoices(academy_id,student_id,title,period,amount,due_date,created_by) VALUES($1,$2,$3,$4,$5,$6::text::date,$7) RETURNING id").bind(u.academy_id).bind(i.student_id).bind(i.title.trim()).bind(&i.period).bind(i.amount).bind(&i.due_date).bind(u.id).fetch_one(&mut *tx).await?;
    audit(
        &mut tx,
        &u,
        "invoice.created",
        &format!("Hisob #{id} / {} UZS", i.amount),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)]
struct PaymentInput {
    request_id: String,
    amount: i64,
    method: String,
    note: String,
}
async fn payment(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<PaymentInput>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    if !crate::ai::valid_id(&i.request_id)
        || !(1..=100000000).contains(&i.amount)
        || !matches!(i.method.as_str(), "cash" | "transfer" | "card")
        || !valid_text(&i.note, 0, 200)
    {
        return Err(bad("To‘lov maydonlarini tekshiring."));
    }
    let mut tx = app.db.begin().await?;
    let row: Option<(i64, bool)> = sqlx::query_as(
        "SELECT amount,voided FROM invoices WHERE academy_id=$1 AND id=$2 FOR UPDATE",
    )
    .bind(u.academy_id)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let (amount, voided) = row.ok_or_else(missing)?;
    let prev: Option<(i64, i64, String, String)> = sqlx::query_as(
        "SELECT invoice_id,amount,method,note FROM payments WHERE academy_id=$1 AND id=$2",
    )
    .bind(u.academy_id)
    .bind(&i.request_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((pid, a, m, n)) = prev {
        if pid == id && a == i.amount && m == i.method && n == i.note.trim() {
            return Ok(Json(json!({"ok":true,"duplicate":true})));
        }
        return Err(bad("So‘rov identifikatori avval ishlatilgan."));
    }
    let paid:i64=sqlx::query_scalar("SELECT COALESCE(sum(amount),0)::bigint FROM payments WHERE academy_id=$1 AND invoice_id=$2 AND voided_at IS NULL").bind(u.academy_id).bind(id).fetch_one(&mut *tx).await?;
    if voided || i.amount > amount - paid {
        return Err(bad("Hisob yopilgan yoki to‘lov qoldiqdan katta."));
    }
    sqlx::query("INSERT INTO payments(id,academy_id,invoice_id,amount,method,note,recorded_by) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(&i.request_id).bind(u.academy_id).bind(id).bind(i.amount).bind(&i.method).bind(i.note.trim()).bind(u.id).execute(&mut *tx).await?;
    audit(
        &mut tx,
        &u,
        "payment.recorded",
        &format!("Hisob #{id} / {} UZS", i.amount),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true,"duplicate":false})))
}
async fn void_invoice(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    let mut tx = app.db.begin().await?;
    let found: Option<i64> =
        sqlx::query_scalar("SELECT id FROM invoices WHERE academy_id=$1 AND id=$2 FOR UPDATE")
            .bind(u.academy_id)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    if found.is_none() {
        return Err(missing());
    }
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM payments WHERE academy_id=$1 AND invoice_id=$2 AND voided_at IS NULL",
    )
    .bind(u.academy_id)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if n > 0 {
        return Err(bad("To‘lov mavjud hisobni bekor qilib bo‘lmaydi."));
    }
    sqlx::query("UPDATE invoices SET voided=true WHERE academy_id=$1 AND id=$2")
        .bind(u.academy_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit(&mut tx, &u, "invoice.voided", &format!("Hisob #{id}")).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
struct Reversal {
    reason: String,
}
async fn reverse_payment(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(i): Json<Reversal>,
) -> Result<Json<Value>> {
    let u = current(&app, &h).await?;
    u.owner()?;
    if !valid_text(&i.reason, 5, 200) {
        return Err(bad("Bekor qilish sababini 5–200 belgi bilan kiriting."));
    }
    let mut tx = app.db.begin().await?;
    let inv: Option<i64> =
        sqlx::query_scalar("SELECT invoice_id FROM payments WHERE academy_id=$1 AND id=$2")
            .bind(u.academy_id)
            .bind(&id)
            .fetch_optional(&mut *tx)
            .await?;
    let inv = inv.ok_or_else(missing)?;
    sqlx::query("SELECT id FROM invoices WHERE academy_id=$1 AND id=$2 FOR UPDATE")
        .bind(u.academy_id)
        .bind(inv)
        .execute(&mut *tx)
        .await?;
    let changed=sqlx::query("UPDATE payments SET voided_at=now(),void_reason=$1 WHERE academy_id=$2 AND id=$3 AND voided_at IS NULL").bind(i.reason.trim()).bind(u.academy_id).bind(&id).execute(&mut *tx).await?.rows_affected();
    if changed > 0 {
        audit(
            &mut tx,
            &u,
            "payment.reversed",
            &format!("To‘lov {id}: {}", i.reason.trim()),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({"ok":true})))
}
fn csv_cell(s: &str) -> String {
    let s = if s.trim_start().starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.into()
    };
    format!("\"{}\"", s.replace('"', "\"\""))
}
async fn finance_csv(State(app): State<App>, h: HeaderMap) -> Result<impl IntoResponse> {
    let u = current(&app, &h).await?;
    u.owner()?;
    let rows:Vec<(i64,String,String,String,i64,i64,bool)>=sqlx::query_as("SELECT i.id,s.name,i.period,i.due_date::text,i.amount,COALESCE((SELECT sum(p.amount) FROM payments p WHERE p.academy_id=i.academy_id AND p.invoice_id=i.id AND p.voided_at IS NULL),0)::bigint,i.voided FROM invoices i JOIN users s ON s.academy_id=i.academy_id AND s.id=i.student_id WHERE i.academy_id=$1 ORDER BY i.id DESC LIMIT 10000").bind(u.academy_id).fetch_all(&app.db).await?;
    let mut csv = String::from("\u{feff}ID,Oquvchi,Oy,Muddat,Summa UZS,Tolangan UZS,Holat\r\n");
    for (id, n, p, d, a, paid, v) in rows {
        csv.push_str(&format!(
            "{id},{},{},{},{a},{paid},{}\r\n",
            csv_cell(&n),
            csv_cell(&p),
            csv_cell(&d),
            csv_cell(if v {
                "Bekor"
            } else if paid >= a {
                "Tolangan"
            } else {
                "Qoldiq"
            })
        ));
    }
    Ok((
        [
            ("content-type", "text/csv; charset=utf-8"),
            (
                "content-disposition",
                "attachment; filename=educore-finance.csv",
            ),
        ],
        csv,
    ))
}
#[cfg(test)]
mod tests {
    #[test]
    fn csv_formula_escape() {
        assert_eq!(super::csv_cell("=cmd()"), "\"'=cmd()\"");
        assert_eq!(super::csv_cell("  +1"), "\"'  +1\"");
        assert_eq!(super::csv_cell("A\"B"), "\"A\"\"B\"");
    }
}
