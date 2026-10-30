# EduCore V5.1 — tekshiruv qaydi

Sana: 2026-10-30. Linux, Rust 1.99.1. Faqat sun’iy hisoblar, to‘lovlar va audio ma’lumotlari ishlatildi.

## Avtomatik tekshiruvlar

V5.1 uchun jami 19 ta Rust test funksiyasi bajarilib o‘tdi: 12 ta library unit/HTTP mock testi, 1 ta parol testi va 6 ta DB integratsiya testi. Oddiy `cargo test --locked` DB testlarini ignored ko‘rsatadi; ular TEST_DATABASE_URL bilan alohida, ketma-ket bajarildi. V5dagi DB sinovlari V5.1 kodi bilan qayta bajarildi.

- Oldingi rol/tenant o‘quv jarayoni, sessiyalarni bekor qilish va parolsiz import hisobini bloklash.
- V4 audio, kurs/dars, taqvim, javoblar tarixi, qayta topshirish va parolni tiklash tokenlari.
- 23 ilova jadvalida RLS; anon/authenticated users SELECT ruxsatiga ega emas. Tasodifiy SELECT grant berilganda ham RLS satrlarni yashiradi.
- V5 hisob va qisman to‘lov, takroriy request_id, ortiqcha summani rad etish, begona akademiya/student/teacher ruxsatlari, ota-ona ko‘rishi.
- To‘lovni sabab bilan bekor qilish, qoldiq qayta hisoblanishi, to‘lovsiz hisobni bekor qilish va ayni oy uchun to‘g‘rilangan yangi hisob yaratish.
- CSV formula boshlovchi belgilarini himoyalash; ownerdan boshqa rolda eksport rad etilishi.
- Akademiya profilini faqat owner yangilashi.
- AI qoralamasini begona akademiya, boshqa teacher va o‘quvchidan yashirish. Kalitsiz generation 503; o‘quvchi generation 403.
- AI HTTP mock: Bearer header, JSON Object Mode, o‘quvchi/ustoz matnlarini ajratib o‘qish, provider xato tanasidagi maxfiy matnni yashirish.
- Claude Messages HTTP mock: `claude-opus-5-5`, Bearer kalit, `anthropic-version: 2026-10-21`, workspace header, 4096 max_tokens, low effort, JSON Schema. Groq formatidagi maydonlar, thinking’ni o‘chirish yoki forced tool use yuborilmasligi tekshirildi.
- Claude thinking/signature bloklari yakuniy matnga o‘tmasligi; tugallanmagan, refusal, tools stop, noto‘g‘ri JSON, noma’lum maydon va matnsiz javobni rad etish.
- 400/401/402/403/429/500/529 xatolarida xom provider matni, kalit va URL qaytarilmasligi; 96 KiBdan katta javob va redirectni rad etish.
- Provayder/model mosligi, eski V5 uchun Groq defaulti, kalitning faqat tanlangan env nomidan olinishi va status JSON ichida kalit bo‘lmasligi.
- `/api/v5` tanlangan Claude modelini qaytarishi, kalitsiz `enabled=false` va ANTHROPIC_API_KEY uchun aniq setup xabari; generation 503. Bu kalit yoki balansning haqiqiyligini tasdiqlamaydi.
- AI kunlik limiti, parallel so‘rovni rad etish, takroriy request_id bilan yangi rezervatsiya qilmaslik. Limit PostgreSQL jadvalida saqlanadi.
- Private Storage mock: audio baytlari, private bucket, no-upsert, authenticated download, delete payload, katta javob va noto‘g‘ri obyekt yo‘lini rad etish.

DB sinovlari SQLx PostgreSQL drayveri orqali **PGlite PostgreSQL WASM + TCP socket**, pool=1 bilan bajarildi. Bu native PostgreSQL 18 yoki real Supabase sinovi emas.

## Migratsiya va yig‘ish

- 001–004 migratsiyalar saqlangan V5 paketi bilan baytma-bayt bir xil. V5.1 yangi DB migratsiyasini talab qilmaydi.
- Bazaviy V5da minimal V4 bazaga 004 qo‘llanganda akademiya, user va audio metadata saqlanib qoldi. Local audio backend o‘zgarmadi. Bu to‘liq real backupni tiklash sinovi emas.
- cargo fmt --check, uchta JS faylning node --check tekshiruvi o‘tdi.
- render.yaml YAML sifatida o‘qildi, runtime=rust va plan=free tekshirildi. Render API validator ishlatilmadi.
- cargo build --release --locked muvaffaqiyatli bajarildi.
- Optimized server yangi DBda PORT orqali ishga tushdi; /health 0.5.1 va /app tekshirildi. CSP / X-Frame-Options saqlandi, private API 401 qaytardi, previewdagi QA yo‘llari haqiqiy serverda mavjud emasligi tekshirildi.

## Bazaviy V5dagi brauzer tekshiruvi

Quyidagilar oldingi V5 paketida tekshirilgan. V5.1da umumiy sahifa tuzilishi saqlangan; yangi AI status matni va sozlama izohlari uchun JS sintaksisi va backend status javobi tekshirildi. V5.1 uchun yangi vizual brauzer sinovi bajarilmadi.

Chrome brauzeri haqiqiy Rust serverga ulandi. Haqiqiy foydalanuvchi credentiallari ishlatilmadi; sinov harnessi faqat sun’iy bazadagi fixture sessiyalarini tayyorladi. Harness va test auth yo‘llari ZIPga qo‘shilmadi.

- Landing, ustoz bosh sahifasi, AI formasi va qoralamalar ko‘rildi.
- Sinov qoralamasidan guruh tanlash → tahrirlab topshiriq yaratish → muvaffaqiyatli saqlanish tekshirildi. Qoralama fixture edi, live AI javobi emas.
- O‘quvchi shu topshiriqning faqat o‘quvchi matnini ko‘rdi; teacher_notes/kalit chiqarilmadi.
- Rahbar qisman to‘lov kiritdi: 500 000 hisob / 200 000 to‘langan holatga 100 000 qo‘shilib, qoldiq 200 000 bo‘ldi.
- 390 px iframe viewportda ustoz bosh sahifasi va AI formasi tekshirildi: body clientWidth=375 va scrollWidth=375 (scrollbar bilan); gorizontal overflow yo‘q.
- Mobil QA iframe uchun faqat tashqi test proxy same-origin framingga ruxsat berdi. Shipped serverda DENY va frame-ancestors 'none' saqlangan.

## Yakuniy qabulda qoladigan ishlar

Haqiqiy ANTHROPIC_API_KEY yoki GROQ_API_KEY kiritilmadi: real model sifati, hisobingizdagi model ruxsati, workspace, balans/kvota va live javoblar tekshirilmagan. AI integratsiyasi kod va HTTP mock bilan tekshirildi. API kaliti bo‘lmasa UI buni ochiq ko‘rsatadi, soxta javob qaytarmaydi. Claude API pullik; ilova so‘rov limiti pul budjeti vazifasini bajarmaydi.

Haqiqiy Render/Supabase deployi, TLS/gateway, tashqi email yetkazish, native Windows skriptlari, real mikrofon, Safari/iOS, native PostgreSQL parallel yuklama, penetration audit va cloud backup/restore bajarilmadi. Email disabled holatda xat yubormaydi. To‘lovlar faqat ichki qo‘lda qayd, kartadan pul yechilmaydi. V5_BOSHLASH.md dagi qabul ro‘yxati haqiqiy hisoblaringizda bajarilishi kerak.
