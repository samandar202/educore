# EduCore V5.1 — Render + Supabase + Groq

Bu repo Rust server, frontend va 001–004 migratsiyalarni o‘z ichiga oladi. Render/Groq sozlamalari bepul rejada boshlash uchun tayyorlangan. Haqiqiy hisoblar, baza paroli va API kalitlari repo ichida yo‘q; ularni Render Environment’da o‘zingiz kiritasiz. Jonli deploy, haqiqiy AI va bucket hali hisobingizda tekshirilmagan.

## Hozirgi mahalliy baza

`127.0.0.1:5432/educore_v4` kompyuteringizdagi PostgreSQLdir. Render’dan Supabase’ning Session pooler URI manziliga ulanamiz. Mahalliy hisoblar, parollar, natijalar va audio fayllari bulutga avtomatik ko‘chmaydi. Mahalliy baza va papkangizni saqlang.

Bo‘sh Supabase bazasi bilan yangi akademiya ochish mumkin. Mavjud mahalliy ma’lumotlarni saqlab ko‘chirish kerak bo‘lsa, bulutda yangi hisoblar yaratishdan oldin DB dump + barcha audio fayllari uchun alohida migratsiya rejasini bajaring. Local audio yozuvlari Supabase obyektlariga avtomatik aylantirilmaydi. `scripts/backup.ps1` mahalliy backup uchun, bulutga import vositasi emas.

## 1. Supabase tayyorgarligi

Avval yaratgan Supabase loyihangizdan foydalaning:

- Connect → Session pooler, 5432 port. URI ichidagi placeholderni loyiha database paroliga almashtiring; maxsus belgilar URL-encoded bo‘lsin. Oxirida `sslmode=require` parametri bo‘lsin. Ulanish useri/baza/hostini Dashboard berganidek qoldiring. Bu `127.0.0.1` manzili emas.
- Storage → `educore-audio` bucket: Private (Public o‘chiq).
- Loyiha URLi va server uchun `sb_secret_...` kalitini oling. Publishable/anon kaliti bu server sozlamasiga mos emas.
- Data API’ni o‘chiring; bu ilova DBga SQLx orqali ulanadi, Storage API alohida ishlaydi. Migratsiyalar ilova jadvallarida RLSni ham yoqadi.

Parol, kalit va to‘liq DATABASE_URLni chatga yoki GitHubga yubormang.

## 2. Render xizmatini ulash

https://dashboard.render.com → New → Web Service → Git Provider → GitHub → `samandar202/educore`.

Agar shu repo uchun xizmat allaqachon mavjud bo‘lsa, o‘sha xizmatning Settings/Environment bo‘limlarini yangilang; yangi xizmat yaratish shart emas.

| Maydon | Qiymat |
|---|---|
| Language | Rust |
| Branch | main |
| Root Directory | Bo‘sh; Cargo.toml repo ildizida |
| Build Command | `cargo build --release --locked -j 1` |
| Start Command | `./target/release/educore` |
| Instance Type | Free |
| Health Check Path | `/health` |

Qo‘llanma manual Web Service yo‘lini ko‘rsatadi. `render.yaml` Blueprint ham bor; bitta yo‘ldan foydalaning. Blueprint keyin yana qo‘llansa, qo‘lda o‘zgartirilgan qiymatlarni YAML bilan mos tuting.

## 3. Environment

Mahalliy `.env`ni serverga ko‘chirmang. Render Environment’da kiriting:

| Key | Value |
|---|---|
| DATABASE_URL | Supabase Session pooler URI, database paroli va sslmode=require bilan |
| SUPABASE_URL | Shu loyihaning https://PROJECT_REF.supabase.co manzili |
| SUPABASE_SECRET_KEY | Shu loyihaning server sb_secret_... kaliti |
| SUPABASE_STORAGE_BUCKET | educore-audio |
| MEDIA_BACKEND | supabase |
| COOKIE_SECURE | true |
| DB_POOL_SIZE | 2 |
| MEDIA_QUOTA_MB | 200 |
| MAIL_MODE | disabled |
| REGISTRATION_ENABLED | false |
| AI_PROVIDER | groq |
| AI_MODEL | openai/gpt-oss-20b |
| AI_DAILY_LIMIT | 10 |
| GROQ_API_KEY | Groq kalitingiz; hali yo‘q bo‘lsa bu envni kiritmay turing, AI o‘chiq bo‘ladi |
| APP_BASE_URL | Render xizmatining haqiqiy https://....onrender.com manzili |

`BIND_ADDR`ni kiritmang; eski qiymati bo‘lsa Render’dan olib tashlang. `PORT`ni Render beradi. Rust server shu portda `0.0.0.0` bilan tinglaydi. `RUSTUP_TOOLCHAIN` override bo‘lmasa repodagi `rust-toolchain` ishlatiladi.

Xizmat manzili yaratilgach APP_BASE_URLni o‘sha haqiqiy HTTPS manziliga qo‘ying va saqlab redeploy qiling. Groq Free kvota bilan ishlaydi. Ilovadagi kunlik limit hamma provayderlarda alohida saqlanadi; kalitning borligi uning haqiqiyligini tasdiqlamaydi.

## 4. Birinchi ishga tushirish

Create Web Service / Deploy tugagach logda `EduCore V5.1` va xizmatda Live holatini kuting. `/health` 0.5.1 qaytarishi kerak. Storage/bucket private/kalit mos kelmasa server sabab bilan to‘xtaydi. Noto‘g‘ri kalit bilan davom etib audio yo‘qotish o‘rniga sozlamani tuzating.

Supabase’da EduCore hisoblari hali bo‘lmasa, REGISTRATION_ENABLEDni vaqtincha true qiling va redeployni kuting. `/app` orqali akademiya va egasi hisobini yarating. So‘ng REGISTRATION_ENABLEDni false qilib qayta deploy qiling. O‘qituvchi/o‘quvchi/ota-ona hisoblarini egasi kabinet ichidan yaratadi. EduCore loginlari Supabase Auth’da yaratilmaydi.

Mahalliy bazadan ma’lumot import qilingan bo‘lsa, mavjud EduCore hisobidan kiring va yangi akademiya yaratmang.

## 5. Haqiqiy sinov

- Boshqa qurilma orqali HTTPS sahifaga kiring.
- Egasi guruh, ustoz, o‘quvchi va ota-ona bog‘lanishini tekshirsin.
- O‘quvchi audio yuborsin, ustoz eshitib izoh bersin. Render restartdan keyin ham audio ochilsin.
- AI’ni haqiqiy Groq kaliti bilan sinang; qoralama va javoblar kalitini tekshirmasdan nashr qilmang.
- O‘quvchi begona o‘quvchining ma’lumotini ko‘ra olmasin.
- Baza va bucket backupini birgalikda olib, tiklashni alohida muhitda sinang.

## Bepul rejalar chegarasi

Render Free 15 daqiqa tashrif bo‘lmasa uxlaydi, qayta ochilish sekinlashishi mumkin. Uning lokal diski restart/redeploy/uyquda yo‘qoladi; shu sabab audio Supabase bucketda turishi kerak. Kvota va reja shartlarini Dashboard’da kuzating. Bepul reja doimiy uzluksiz production xizmati uchun kafolat bermaydi.

MAIL_MODE=disabled bo‘lgani uchun email tasdiqlash/parol tiklash xatlari yuborilmaydi; egasi kabinetdan parolni yangilaydi. Render Free SMTP portlarini cheklaydi. Onlayn to‘lov olish integratsiyasi yo‘q; moliyaviy bo‘lim ichki qayd yuritadi.

Rasmiy manbalar (2026-10-30):
- https://render.com/docs/web-services
- https://render.com/docs/free
- https://render.com/docs/rust-toolchain
- https://supabase.com/docs/guides/database/connecting-to-postgres
- https://supabase.com/docs/guides/getting-started/api-keys
- https://supabase.com/docs/guides/storage/buckets/fundamentals
- https://console.groq.com/docs/rate-limits
