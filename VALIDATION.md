> Asl V4 tekshiruv qaydi. Cloud natijalari CLOUD_VALIDATION.md da.

# EduCore V4 — tekshiruv qaydi

Sana: 2026-10-29. Testlar sun’iy akademiya, hisob va audio ma’lumotlarida bajarildi.

## Bajarilgan

- Linux, Rust 1.99.1: `cargo build --locked`, `cargo fmt`, `cargo test --locked` muvaffaqiyatli. Toza package rebuilddan keyin serverga joylangan JS eng yangi manbaga mosligi tekshirildi.
- 4 mustaqil test: Argon2 salt/parol chegaralari, haqiqiy kalendar sanalari, audio Range chegaralari, HTMLni audio sifatida rad etish.
- 3 DB integratsiya testi alohida bajarilib o‘tdi. Oddiy `cargo test` ularni TEST_DATABASE_URL kerakligi sabab ignored ko‘rsatadi.
- V3dan qolgan rol/tenant/full learning flow/sessiya bekor qilish va parolsiz import hisobini bloklash testlari o‘tdi.
- V4 DB testi: kurs/dars yaratish, begona akademiya va o‘quvchi tahririni rad etish, guruhga tegishli bo‘lmagan darsni topshiriqqa bog‘lashni rad etish.
- Calendar: bir guruhda ustma-ust vaqtni, bir ustozning ikki turli guruhdagi ustma-ust vaqtini rad etish; yonma-yon darsga ruxsat. Arxivlash kelajak tadbirlarini bekor qiladi.
- Media: 32 KBdan katta upload, noto‘g‘ri format/rol, 1 MB test kvotasi, boshqa o‘quvchi audiosini topshirishga urinish, tenant izolyatsiyasi, tegishli ota-ona va ustoz kirishi, HTTP 206 byte-range.
- Audio izoh va qayta topshirish: eski answer/audio/score/feedback tarixda saqlanishi, stale revision bilan baholashni rad etish.
- Topshiriq arxivlanganda topshirish yopilishi, guruh ko‘chirilganda tarix/audio saqlanishi, begona bildirishnomani o‘qildi belgilash rad etilishi.
- Fon eslatmasi ikki marta ishga tushganda dublikat chiqmasligi, Tashkent sanasi bo‘yicha ertangi muddat.
- Parolni tiklash tokenining bir martaligi, email tasdiqlash, muddati tugagan tokenni rad etish, lokal xat navbatining faylga yetkazilishi, eski sessiyalar va arxivlangan login bloklanishi.

## Brauzer va migratsiya

Chromium 130 + Playwright haqiqiy Rust HTTP serveriga ulandi:

1. Avval faqat V3 `001_core.sql` sxemasi, SQLx checksum yozuvi va sinovdagi eski o‘quvchi/topshiriq/88 ballik javob yaratildi. V4 server yoqilgach migratsiya 002 qo‘shildi, eski javob va baho saqlandi.
2. UI: akademiya → guruh → o‘quvchi → ota-ona → kurs → dars → guruhga kurs biriktirish → audio topshiriq.
3. Brauzer `MediaRecorder` bilan WebM yozish/yuklash, ustozning audio izohi, qayta topshirishda WAV fayl tanlash, ikkinchi baho.
4. Mobil ota-ona kabinetida 64/100 → 89/100 taqqoslash. Tarixiy WebM javob, audio izoh va yangi WAV brauzerda `canplay` holatiga yetdi. Ota-onada javob yozish maydoni yo‘q.
5. Calendar yaratish va konflikt xabari, sessiyalar ro‘yxati, xat navbatiga yuborish, bildirishnomani o‘qish, fragmentdagi tiklash tokeni orqali yangi parol o‘rnatish, hisobni arxivlash, barcha navigatsiya ekranlari.
6. JS runtime xatolari qayd etilmadi. Login oldidagi 401 javobi kutilgan holat. HTML pattern muammosi tuzatildi va yakuniy browser testda takrorlanmadi.
7. 1440 px desktop va 390 px mobil. Mobil tarix ekranida gorizontal overflow yo‘q. Kurslar, Speaking Studio, kalendar va mobil tarix ekranlari ko‘zdan kechirildi. Ichki screenshotlar arxivga qo‘shilmadi.
8. PGlite data-directory snapshoti yangi, alohida PGlite bazasiga tiklandi. Eski V3 javobi va yangi V4 urinish tarixi saqlandi.

## Sinov chegaralari

- DB testlari **PGlite PostgreSQL WASM engine + PostgreSQL wire-protocol socket** bilan bajarildi, SQLx PostgreSQL drayveri ishlatildi. PGlite uchun pool=1. Bu native PostgreSQL 18da parallel yuklama yoki Windows xizmati testi emas.
- Ushbu muhitda mikrofon qurilmasi ochilmadi. Browser yozish testida getUserMedia o‘rniga WebAudio orqali sun’iy audio oqimi berildi; haqiqiy MediaRecorder, Blob, upload, server saqlashi va brauzer audio dekodlashi ishladi. Fizik mikrofon, Safari/iOS va boshqa qurilmalar alohida tekshirilishi kerak.
- Faqat lokal email outbox tekshirildi. Tashqi SMTP serveriga xat yuborilmadi; haqiqiy provayder, credentials, deliverability tekshirilmagan.
- Snapshot tiklash testi native `pg_dump`/`pg_restore` testi emas. Windows PowerShell start/backup/restore skriptlari yozildi va ko‘rib chiqildi, bu Linux muhitida bajarilmadi. O‘z qurilmangizda BACKUP.md bo‘yicha alohida bazaga tiklab tekshiring.
- V2 import kodi V3dagi tekshirilgan ko‘rinishda saqlandi; ushbu V4 bosqichida alohida V2 import ssenariysi qayta bajarilmadi.
- Katta yuklama, serverlararo masshtablash, S3, videodars, AI, to‘lov va production penetration testi bajarilmagan. Bootstrapning barcha ruxsatli ma’lumotlarni olishi katta akademiyalar uchun keyingi optimallashtirishdir.

Natija: V4 o‘quv jarayoni ishlaydigan, tekshirilgan pilot. Internetga deploy qilinmagan.
