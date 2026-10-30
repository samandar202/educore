# V4 Cloud 0.4.1 — tekshiruv qaydi

2026-10-29. Linux, Rust 1.99.1. Sun’iy hisoblar va audiolar ishlatildi.

## Bajarilgan

- `cargo build --release --locked -j 1` muvaffaqiyatli tugadi. Dastlabki urinishdan so‘ng qayta yig‘ildi; yakuniy optimized build exit=0.
- Aynan release binary yangi test bazasi bilan ishga tushirildi: PORT=3034, /health 0.4.1, /app 200 va migratsiyalar o‘tdi.

- 5 library testi va 1 parol testi o‘tdi.
- 4 DB integratsiya testi alohida `--ignored --exact` bilan o‘tdi: tenant/rol/o‘quv jarayoni; parolsiz import hisobini bloklash; V4 audio/kurs/taqvim/tarix/tiklash; cloud RLS va yopiq registratsiya.
- Storage HTTP mock: apikey, private bucket talabi, upload/no-upsert, authenticated download va audio baytlari, delete payload, public bucketni rad etish, 401 response tanasidagi maxfiy matnni chiqarmaslik, 8 MBdan katta downloadni rad etish.
- Noto‘g‘ri audio identifikatori/path traversal rad etildi.
- DB: PGlite PostgreSQL WASM + TCP socket, SQLx PostgreSQL drayveri, pool=1. Native PostgreSQL/Supabase emas.
- 20 jadvalda RLS yoqildi. anon/authenticated users SELECT ruxsatiga ega emas; testda SELECT qayta berilganda ham RLS ma’lumotni yashirdi.
- Asl 001/002 migratsiyalar baytma-bayt o‘zgarmagan. V4dan 003ga o‘tishda audio yozuvi saqlandi va storage_backend=local bo‘ldi.
- Haqiqiy Rust HTTP serveri PORT=3034, BIND_ADDRsiz ishga tushdi; /health 0.4.1, /app 200. Supabase rollari bo‘lmagan yangi bazada ham migratsiyalar o‘tdi.
- cargo fmt --check o‘tdi. render.yaml YAML sifatida o‘qildi, runtime=rust va plan=free tekshirildi. Render API validator ishlatilmadi.

## Chegaralar

Haqiqiy Supabase/Render hisoblari ulanmagan. Haqiqiy Storage gateway, TLS, restartdan keyingi audio va mikrofon sinovi deploydan keyin CLOUD_DEPLOY.md bo‘yicha bajariladi. Production yuklama yoki pentest auditi bajarilmadi.

Frontend o‘zgartirilmadi, ushbu bosqichda brauzer sinovi qayta bajarilmadi. Avvalgi V4 brauzer qaydi VALIDATION.md da. Windows PowerShell skriptlari bu Linux muhitida bajarilmadi. Cloud backup/restore avtomatlashtirilmagan. MAIL_MODE=disabled email yubormaydi.
