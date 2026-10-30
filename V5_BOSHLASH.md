# EduCore V5.1 — o‘rnatish va topshirish

V5.1 Claude Opus 5.5 ulanishini qo‘shadi. V5dan yangilash uchun CLAUDE_ULASH.md ni o‘qing. Yangi DB migratsiyasi talab qilinmaydi; mavjud 001–004 fayllar o‘zgarmagan.

V5 Rust server + PostgreSQL + brauzer interfeysidan iborat. Yangi qismlar: ustoz AI, oylik hisoblar va qisman to‘lovlar, sabab bilan yozuvni bekor qilish, CSV hisobot, e’tibor markazi, akademiya profili va 3D CSS interfeys. V4 kurslari, speaking studio, audio izoh, taqvim, davomat, ota-ona kabineti va ishlar tarixi saqlangan.

## Ishlayotgan V4dan yangilash — Windows PowerShell

1. Eski serverni Ctrl+C bilan to‘xtating. Eski papka va `.env`ni saqlang. Avval DB + audio backupini oling. Mahalliy nusxa uchun BACKUP.md; Supabase uchun CLOUD_DEPLOY.md.
2. ZIPni yangi papkaga oching. PowerShellda uning `educore` papkasiga kiring. `Test-Path .\Cargo.toml` True chiqsin.
3. Oldingi ishlayotgan `.env`ni yangi `educore` papkasiga **nusxalang**. Ishlayotgan DATABASE_URL username/parol/bazani o‘zgartirmang. `.env.example` bilan eski parolingizni ustidan yozmang.
4. Media mahalliy bo‘lsa eski `data\media` papkasini yangi loyihaga nusxalang yoki MEDIA_DIRni eski papkaning to‘liq manziliga yo‘naltiring. DB va media birga saqlanishi kerak. Supabase bucket ishlatilsa o‘sha sozlamalarni saqlang.
5. Yangi papkada bajaring:

```powershell
Test-Path .\Cargo.toml
Test-Path .\.env
Remove-Item Env:DATABASE_URL -ErrorAction SilentlyContinue
cargo run --locked -j 1
```

6. `http://127.0.0.1:3000/app`ni oching. Oldingi hisoblar bilan kiring. 004 migratsiya avtomatik ishlaydi; eski o‘quvchilar va natijalar o‘chirilmaydi.

Birinchi o‘rnatishda `.env.example`dan `.env` yarating, PostgreSQLda avval yaratilgan user/parol/baza bilan DATABASE_URLni to‘ldiring. Ulanuvchi user bazadagi jadvallarning egasi bo‘lishi va migratsiya yaratish huquqiga ega bo‘lishi kerak. Supabase uchun Connect → Session pooler → 5432 yo‘lini ishlating.

`rust-toolchain` tekshirilgan Rust versiyasini belgilaydi; Rustup birinchi ishga tushirishda yuklashi mumkin. Windows/8 GB qurilmada birinchi build vaqt oladi. `-j 1` xotira sarfini kamaytiradi. Localda hamma funksiyani test qilish uchun Docker kerak emas.

## Ustoz AI ni ulash

AI tanlangan Anthropic yoki Groq API orqali haqiqiy modelga ulanadi. Model o‘rniga soxta javob qaytarilmaydi. Kalit bo‘lmasa UI “AI hali ulanmagan” deb ko‘rsatadi; boshqa funksiyalar ishlaydi.

1. https://platform.claude.com/settings/keys da Claude API key yarating. Tarif, balans va foydalanish limitini Claude Console hisobida tekshiring. Kalit yaratishda kerakli workspace’ni tanlang. API hisob-kitobi Claude chat obunasidan alohida.
2. Mahalliy `.env` yoki Render → Environmentga kiriting:

```dotenv
AI_PROVIDER=anthropic
ANTHROPIC_API_KEY=your_anthropic_api_key
AI_MODEL=claude-opus-5-5
AI_DAILY_LIMIT=10
```

3. Serverni qayta ishga tushiring / Renderda redeploy qiling.
4. Ustoz yoki rahbar → Ustoz AI → dars rejasi, mashq/test, yozma izoh yoki speaking faoliyatini tanlang. Mavzu, CEFR daraja, bolalar/kattalar va davomiylikni kiriting.
5. Qoralamani yarating. O‘quvchi matni va ustoz izohini tekshiring. Javoblar kaliti ustoz izohida turadi.
6. “Tahrirlab topshiriqqa aylantirish” → guruh/ko‘nikma/muddatni tanlang → tekshirib yuboring. Faqat o‘quvchi matni yuboriladi. 4000 belgidan uzun matnni qisqartirish kerak.

Claude API pullik. 2026-10-30dagi rasmiy standart tarif: 1 million kirish tokeni uchun $4, chiqish tokeni uchun $20; hisob-kitobning joriy shartlarini provayderda tekshiring. Ilova har so‘rovda `max_tokens=4096`, `effort=low` ishlatadi. Bu bepul limit yoki qat’iy pul budjeti emas; thinking ham token sarflashi mumkin. Provayder hisobida alohida xarajat limitini belgilang. Ilova xatoda avtomatik boshqa modelga o‘tmaydi yoki yangi so‘rov yubormaydi.

Groq’ni ishlatish uchun `AI_PROVIDER=groq`, `GROQ_API_KEY=...`, `AI_MODEL=openai/gpt-oss-20b` yozing. Groq Free tier mavjud, lekin model ruxsati va kvotalar hisobingizga bog‘liq. Eski V5 `.env`ida AI_PROVIDER umuman bo‘lmasa, Groq tanlanadi; faqat Anthropic kalitining mavjudligi provayderni o‘zgartirmaydi. `AI_MODEL` tanlangan provayderga mos bo‘lishi shart. Kalit workspace’ga biriktirilmagan bo‘lsa, `ANTHROPIC_WORKSPACE_ID`ni ham kiriting.

Standart limit: ustozga 10 ta/kun, akademiyaga jami 50 ta/kun (AI_DAILY_LIMIT × 5). Kun UTCda yangilanadi. Xato bilan tugagan so‘rov ham limitga kiradi. Bir ustoz bir paytda bitta so‘rov yuboradi. Limitlar bazada saqlanadi, restartda yo‘qolmaydi. Bir xil request_id bilan qayta yuborish yangi AI so‘rovini yaratmaydi.

Kalit faqat serverda saqlanadi. Uni chat, GitHub, brauzer yoki ZIPga qo‘ymang. AIga faqat kiritilgan mavzu va qo‘shimcha matn yuboriladi; o‘quvchi bazasi avtomatik yuborilmaydi. Qo‘shimcha maydonga ism, telefon, email va boshqa shaxsiy ma’lumot kiritmang.

AI xato qilishi mumkin: u avtomatik baho qo‘ymaydi, topshiriq yoki xabar yubormaydi. Nashr qilishni ustoz tasdiqlaydi. Claude JSON Schema, Groq JSON Object Mode ishlatadi; qaytgan matn serverda ham tekshiriladi. Claude thinking bloklari saqlanmaydi va foydalanuvchiga yuborilmaydi. Claude so‘rovi 90 soniyada, Groq 40 soniyada timeout beradi; xato so‘rov ham provayderda hisoblanishi mumkin.

## To‘lovlar

- Rahbar bir o‘quvchiga bir oy uchun bitta faol oylik hisob yaratadi.
- Haqiqatda olingan to‘lovni naqd/o‘tkazma/karta deb qo‘lda qayd etadi. Qisman to‘lash mumkin.
- Qoldiqdan katta to‘lov rad etiladi; takroriy bir xil so‘rov ikki marta yozilmaydi.
- Xato yozuv sabab bilan bekor qilinadi. Asl yozuv va audit qoladi; hisob qoldig‘i qayta ochiladi. Bu pulni avtomatik qaytarmaydi.
- Faol to‘lovsiz hisob bekor qilinishi mumkin; tarixdan o‘chmaydi. Keyin o‘sha oy uchun to‘g‘rilangan yangi hisob yaratsa bo‘ladi.
- O‘quvchi o‘z, ota-ona faqat bog‘langan farzandining hisoblarini ko‘radi. O‘qituvchi moliyaviy hisoblarni ko‘rmaydi.
- Ekranda so‘nggi 1000 hisob va 2000 to‘lov ko‘rinadi; CSV so‘nggi 10 000 hisobni oladi. Qoldiq har bir hisobning barcha bekor qilinmagan to‘lovlari bo‘yicha hisoblanadi.
- CSV UTF-8 BOM bilan Excel uchun tayyorlanadi. Foydalanuvchi matnidagi formula boshlovchi belgilar himoyalangan.

Bu ichki to‘lov qaydi. Bank kartadan pul yechish, Click/Payme va fiskal chek integratsiyalari qo‘shilmagan.

## E’tibor markazi va profil

Ko‘rishga ruxsat berilgan o‘quvchilarning kechikkan ishlari, oxirgi 10 ta davomatda kamida 2 marta yo‘qligi yoki 60dan past o‘rtacha bahosi chiqadi. Bu qaydlarga asoslangan signal; AI tashxisi yoki rasmiy til darajasi emas. Sababini avval o‘quvchi bilan aniqlashtiring.

Sozlamalarda akademiya nomi, tavsifi va **ommaviy** kontaktni yangilang. Sinov darsi arizalari havolasi orqali qabul markaziga murojaat yig‘iladi. Kurslar, hisobotlar, arxivlash, sessiyalar, parol almashtirish va audit tarixi saqlangan.

3D effektlar CSS bilan bajarilgan; katta 3D kutubxonasi talab qilinmaydi. Reduced Motion yoqilgan qurilmalarda animatsiya o‘chadi. Tor ekran uchun menyu va kartalar moslashadi.

## Render + Supabase

CLOUD_DEPLOY.md dagi yo‘l V5 uchun ham amal qiladi. GitHubga yangi `src`, `web`, `Cargo.toml`, `Cargo.lock`, `004_workspace.sql` va render.yamlni yuklang. Eski 001–003 migratsiyalarni tahrirlamang. `.env`, `target`, data, backup va kalitlarni yuklamang.

Claude env: AI_PROVIDER=anthropic, ANTHROPIC_API_KEY, AI_MODEL=claude-opus-5-5, AI_DAILY_LIMIT=10. Workspace’ga biriktirilmagan kalitda ANTHROPIC_WORKSPACE_ID ham kerak. MEDIA_BACKEND=supabase, COOKIE_SECURE=true, REGISTRATION_ENABLED=false qiymatlarini saqlang. Yangi akademiya ochish faqat birinchi egasi hisobini yaratish vaqtida vaqtincha yoqiladi.

Mahalliy ma’lumotlar bulutga o‘z-o‘zidan ko‘chmaydi. Baza va media alohida ko‘chiriladi. Free hosting uxlashi, kvotasi tugashi yoki loyiha pauza bo‘lishi mumkin; har doim uyg‘oq turish kafolati yo‘q. Bu konfiguratsiya bitta server instance uchun.

MAIL_MODE=disabled: email tasdiqlash va email orqali parol tiklash ishlamaydi. Rahbar kabinetdan o‘quvchining parolini yangilashi mumkin. Render Free SMTP cheklovini CLOUD_DEPLOY.md dan o‘qing; ishlaydigan email provayderi alohida sozlanadi.

## Topshirishdan oldingi qabul sinovi

1. Rahbar, ustoz, o‘quvchi va ota-ona bilan alohida kirib ko‘ring. Begona ma’lumot ochilmasin.
2. Tanlangan provayderning haqiqiy kaliti bilan bitta dars va test tayyorlang. Mazmunni va javoblar kalitini tekshiring, keyin guruhga yuboring.
3. O‘quvchi javob yuborsin, ustoz baho/izoh bersin; ota-ona natijani ko‘rsin.
4. Telefonning HTTPS sahifasida mikrofon bilan audio yozing. Render restartdan keyin o‘sha audioni qayta eshiting.
5. Sinov hisobi → qisman to‘lov → CSV → sababli bekor qilish oqimini bajaring. Haqiqiy pul avtomatik o‘tkazilmaydi.
6. DB VA media/bucket backupini alohida joyga tiklab ko‘ring.
7. Email yoqilgan bo‘lsa parol tiklash xati haqiqatan kelishini tekshiring. Administrator hisobiga kirishni saqlang.
8. REGISTRATION_ENABLED=false bo‘lsin; kalitlar ochiq repository, brauzer yoki logda chiqmasin.

Haqiqiy Supabase/Render hisoblari va API kaliti bu paket bilan avtomatik ulanmaydi. Native PostgreSQL 18, real AI sifati, real mikrofon va backupni tiklash yakuniy qabulda tekshiriladi. Joriy testlar va ularning chegaralari V5_VALIDATION.md da yozilgan.

Manbalar:
- https://platform.claude.com/docs/en/get-api-key
- https://platform.claude.com/docs/en/models/opus-5-5/overview
- https://platform.claude.com/docs/en/build-with-claude/structured-outputs
- https://platform.claude.com/docs/en/build-with-claude/effort
- https://platform.claude.com/docs/en/api/versioning
- https://console.groq.com/docs/api-reference
- https://console.groq.com/docs/structured-outputs
- https://console.groq.com/docs/models
- https://console.groq.com/docs/rate-limits
- https://console.groq.com/docs/billing-faqs
