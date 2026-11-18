# EduCore — Northflank (bepul, uxlamaydi) + Supabase + Groq

Northflank Sandbox (bepul) rejasi: 2 ta servis, doim yoqiq (uxlamaydi). Baza va audio avvalgidek Supabase'da qoladi.

## 1. Hisob
northflank.com → Sign up (GitHub bilan kirish qulay). Sandbox rejasi tanlansin. Ba'zi hollarda karta tasdig'i so'ralishi mumkin — pul yechilmaydi, lekin Sandbox chegarasidan oshmang.

## 2. Servis yaratish
1. Create project → region: Europe (Supabase loyihangizga yaqin region).
2. Create new → Service → **Combined service** (build + deploy).
3. Repository: GitHub → `samandar202/educore`, branch `main`.
4. Build type: **Dockerfile**, yo'li `/Dockerfile`, context `/`.
5. Resources: Sandbox'dagi eng kichik bepul plan.
6. Networking: port **8080**, protocol HTTP, **Publicly expose** yoqilgan.
7. Health check (ixtiyoriy): HTTP, path `/health`, port 8080.

## 3. Environment variables
| Key | Value |
|---|---|
| DATABASE_URL | Supabase Session pooler URI (5432, sslmode=require) |
| SUPABASE_URL | https://PROJECT_REF.supabase.co |
| SUPABASE_SECRET_KEY | sb_secret_... |
| SUPABASE_STORAGE_BUCKET | educore-audio |
| MEDIA_BACKEND | supabase |
| COOKIE_SECURE | true |
| DB_POOL_SIZE | 2 |
| MEDIA_QUOTA_MB | 200 |
| MAIL_MODE | disabled |
| REGISTRATION_ENABLED | true (o‘quvchi/ota-ona/o‘qituvchi o‘zi ro‘yxatdan o‘tadi; false — hammasini yopadi) |
| AI_PROVIDER | groq |
| AI_MODEL | openai/gpt-oss-20b |
| AI_DAILY_LIMIT | 10 |
| GROQ_API_KEY | console.groq.com → API Keys (bepul) |
| APP_BASE_URL | Northflank bergan https://....code.run manzil |
| KEEP_ALIVE_URL | APP_BASE_URL bilan bir xil (Supabase pauzaga tushmasligi uchun) |

`PORT` Dockerfile'da 8080. `BIND_ADDR` kiritmang. Kalitlarni GitHub'ga yozmang.

## 4. Ishga tushirish
Create service → birinchi Rust build 10–20 daqiqa olishi mumkin. Logda `EduCore V5.1` chiqqach, `https://....code.run/health` ochilsin. Keyin darhol `/app` ga kirib akademiya va egasi (sizning) hisobini yarating. Akademiya faqat bir marta ochiladi — shundan keyin hech kim ikkinchi akademiya yoki egasi hisobini ocha olmaydi.

## Eslatma
- Supabase Free faolsiz loyihani ~1 haftada pauza qiladi; server har 10 daqiqada `SELECT 1` yuborgani uchun bunday bo'lmasligi kerak.
- Render'dagi xizmatni o'chirib qo'ying (ikkalasi bitta bazaga yozmasin).
- Bepul rejalar shartlari o'zgarishi mumkin; Northflank Billing sahifasini kuzating.

## V6: rollar va ro'yxatdan o'tish
- **Egasi (owner)** — bitta, birinchi bo'lib akademiyani ochgan odam.
- **O'quvchi** — `/app?register=1` orqali o'zi ro'yxatdan o'tadi va darhol kiradi. Guruhga egasi biriktiradi.
- **O'qituvchi** — o'zi ro'yxatdan o'tadi, lekin egasi Bosh sahifadagi "Tasdiq kutayotgan o'qituvchilar" panelida tasdiqlamaguncha kira olmaydi.
- **Ota-ona** — farzandining 6 xonali kodi bilan ro'yxatdan o'tadi (kod o'quvchining Bosh sahifasida).
- **AI** faqat o'qituvchilarda. **O'yin markazi**: XP, daraja, nishonlar, reyting va 60 soniyalik so'z o'yini.
