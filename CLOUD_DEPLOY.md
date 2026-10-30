> Joriy repo uchun DEPLOY_FREE_UZ.md ni o‘qing: Render + Supabase + Groq sozlangan. Ushbu tarixiy V4 yo‘riqnomasi saqlangan; V5.1ning bepul AI sozlamalari yangi qo‘llanmada. Ixtiyoriy pullik Claude uchun CLAUDE_ULASH.md mavjud.

# EduCore V4 Cloud (0.4.1): Render + Supabase

Bu paket bulutga chiqarish uchun moslashtirilgan. Hisoblaringizga hali deploy qilinmagan. Hozirgi ishlayotgan mahalliy papka va bazani saqlang. Quyidagi yo‘l yangi bulut bazasi bilan boshlaydi; eski o‘quvchilar, parollar va audiolar avtomatik ko‘chmaydi. Mavjud ma’lumotlarni ko‘chirish alohida ish: database dump bilan birga audio fayllari ham ko‘chishi kerak.

## 1. Supabase sozlamalari

1. Loyiha → Connect → **Session pooler**, port **5432**. Dashboard bergan URI manzilini oling. Transaction pooler / 6543 ni tanlamang.
2. `[YOUR-PASSWORD]` o‘rniga Supabase loyiha **database paroli**ni qo‘ying. Bu saytga kirish paroli emas. Qavslarni olib tashlang. Paroldagi maxsus belgilar URL-encoded bo‘lishi kerak: `@` → `%40`, `#` → `%23` va hokazo.
3. Oxiriga `?sslmode=require` qo‘shing. URI allaqachon `?` saqlasa `&sslmode=require` qo‘shiladi. Dashboarddagi host/user/bazani o‘zgartirmang. User odatda `postgres.PROJECT_REF`, baza `postgres`.
4. Storage → bucket `educore-audio`: **PRIVATE**, Public o‘chiq. Browser uchun read/upload policy kerak emas; Rust server o‘z ruxsatlarini tekshiradi.
5. Loyiha URLini oling: `https://PROJECT_REF.supabase.co`.
6. Settings → API Keys → Secret keys: `sb_secret_...` server kalitini oling, mavjud bo‘lmasa yarating. Publishable/anon kaliti mos emas. Secretni chatga, frontendga yoki GitHubga joylamang.
7. Data API sozlamasida **Enable Data API** ni o‘chiring. Rust SQL orqali ulanadi; Storage API alohida. Migratsiya ham ilova jadvallarida RLSni yoqib, anon/authenticated ruxsatlarini bekor qiladi.

URI namunasi:

```text
postgresql://postgres.PROJECT_REF:ENCODED_DB_PASSWORD@YOUR_SESSION_POOLER_HOST:5432/postgres?sslmode=require
```

## 2. GitHubga yuklash

1. ZIPni yangi papkaga oching. Eski ishlayotgan papkangiz ustiga yozmang.
2. GitHub → New repository → nomi `educore` → **Private** → Create repository.
3. `educore` papkasining ICHIDAGI fayllarni repository ildiziga yuklang. `Cargo.toml`, `Cargo.lock`, `render.yaml`, `rust-toolchain`, `src`, `web`, `migrations` ildizda tursin. ZIPning o‘zini yuklamang.
4. `.env`, `target`, `data`, `backups`ni yuklamang. Paketda ular yo‘q. `.env.example` namuna; `.gitignore`ni ham yuklang.
5. Commit changes.

## 3. Render Web Service

1. New → Web Service → GitHubdagi `educore` reposini ulang.
2. Runtime: **Rust**. Root Directory: `Cargo.toml` ildizda bo‘lsa bo‘sh qoldiring. Agar `educore/Cargo.toml` bo‘lsa `educore` yozing.
3. Build Command: `cargo build --release --locked -j 1`
4. Start Command: `./target/release/educore`
5. Instance Type: **Free**. Health Check Path: `/health`.
6. Environment bo‘limida quyidagilarni kiriting:

| Key | Value |
|---|---|
| DATABASE_URL | Session pooler URI, haqiqiy database paroli va sslmode=require bilan |
| SUPABASE_URL | https://PROJECT_REF.supabase.co |
| SUPABASE_SECRET_KEY | sb_secret_... server kaliti |
| SUPABASE_STORAGE_BUCKET | educore-audio |
| MEDIA_BACKEND | supabase |
| COOKIE_SECURE | true |
| DB_POOL_SIZE | 2 |
| MEDIA_QUOTA_MB | 200 |
| MAIL_MODE | disabled |
| REGISTRATION_ENABLED | false |
| APP_BASE_URL | Render bergan https://SIZNING-NOM.onrender.com |

`BIND_ADDR` va `PORT`ni qo‘lda kiritmang. Server Render bergan PORTda 0.0.0.0 bilan tinglaydi. APP_BASE_URL manzili hali noma’lum bo‘lsa xizmat URLi paydo bo‘lgach haqiqiy URLni saqlab qayta deploy qiling.

7. Deploy Web Service. Birinchi build vaqt olishi mumkin. Migratsiyalar avtomatik ishlaydi. Bucket/kalit/private sozlamasi xato bo‘lsa server ishga tushmay, sababni logda ko‘rsatadi.

Muqobil: repo ildizidagi `render.yaml` bilan New → Blueprint. Bitta usulni tanlang, ikki xizmat yaratmang. Blueprint ishlatilsa keyingi environment o‘zgarishlarini YAMLda ham yangilang.

## 4. Egasi hisobini yaratish

1. Sayt Live bo‘lgach `REGISTRATION_ENABLED=true` qilib saqlang va redeploy tugashini kuting.
2. Sayt `/app` sahifasida akademiyangiz va egasi hisobini yarating. Alohida kuchli parol ishlating.
3. Darhol `REGISTRATION_ENABLED=false` qilib qayta deploy qiling. Faqat yangi akademiya ochish yopiladi; egasi kabinetdan o‘qituvchi/o‘quvchi/ota-ona qo‘shishi davom etadi.
4. Hisoblar Supabase Authdan emas, EduCore kabinetidan yaratiladi.

## 5. Haqiqiy deploy sinovi

- Egasi: guruh, o‘qituvchi va o‘quvchi yarating, o‘quvchini guruhga qo‘shing.
- O‘qituvchi audio topshiriq yaratsin.
- O‘quvchi HTTPS orqali mikrofon ruxsatini berib javob yuborsin.
- O‘qituvchi eshitib, baho va audio izoh yuborsin.
- O‘quvchi / bog‘langan ota-ona natijani ko‘rsin; begona o‘quvchi audioni ocha olmasin.
- Render restart/redeploydan keyin o‘sha audioni qayta eshiting. Bucketdagi obyekt yo‘li `AKADEMIYA_ID/AUDIO_ID` bo‘ladi.
- Chiqib qayta kirish, davomat, kurs, taqvim va boshqa telefondan kirishni tekshiring.
- Shu sinovlar haqiqiy hisoblaringizda o‘tgach foydalanuvchilarga manzil bering.

## Bepul pilot chegaralari

Render Free 15 daqiqa faol bo‘lmasa uxlaydi; birinchi kirish sekin bo‘lishi mumkin. Supabase faol bo‘lmagan loyihani pauza qilishi mumkin. Kvotalarni Dashboarddan kuzating. Bir necha akademiya umumiy Storage kvotasini bo‘lishadi; MEDIA_QUOTA_MB har bir akademiya uchun. Bu konfiguratsiya bitta Render instance uchun.

MAIL_MODE=disabled: email tasdiqlash va email orqali parol tiklash xatlari yuborilmaydi. Render Free uchun SMTPni yoqmang; keyin HTTPS email provayderi integratsiyasi kerak. Administrator hisobiga kirishni saqlang.

Render diski vaqtinchalik. Audio Supabase bucketga, ma’lumotlar PostgreSQLga yoziladi. Eski local audio yozuvlari avtomatik Supabasega aylanmaydi. Bucketni public qilmang yoki faol obyektlarni o‘chirmang.

## Backup

`scripts/backup.ps1` mahalliy fayllar uchun; supabase rejimida to‘xtaydi. Cloud backup avtomatlashtirilmagan. Yozuvlarni vaqtincha to‘xtatib, PostgreSQL `pg_dump` nusxasi VA `educore-audio` bucketning barcha obyektlarini birgalikda saqlash kerak. Obyekt yo‘llari saqlansin. Haqiqiy o‘quvchilarni ko‘chirishdan oldin alohida loyihaga tiklash sinovini bajaring.

DB commit natijasi noma’lum bo‘lsa audio avtomatik o‘chirilmaydi; shu sabab kamdan-kam yetim obyekt qolishi mumkin. Avtomatik tozalash yo‘q.

## Xatolar

- Authentication/non-UTF-8: DATABASE_URL username, database paroli va URL encodingni tekshiring.
- Storage 401/403: secret kaliti va URL bir loyihadan bo‘lsin.
- Storage 404: bucket nomini tekshiring.
- PRIVATE: bucketdagi Publicni o‘chiring.
- Eski Rust: `rust-toolchain` fayli yuklanganini tekshiring. Renderdagi RUSTUP_TOOLCHAIN qiymati fayldagi qiymatdan ustun.
- Xatoni yuborishda parol, cookie va secretni yashiring.

Manbalar (2026-10-29):
- https://render.com/docs/free
- https://render.com/docs/web-services
- https://render.com/docs/rust-toolchain
- https://render.com/docs/blueprint-spec
- https://supabase.com/docs/guides/database/connecting-to-postgres
- https://supabase.com/docs/guides/getting-started/api-keys
- https://supabase.com/docs/guides/api/securing-your-api
- https://supabase.com/docs/guides/storage/serving/downloads
