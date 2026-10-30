# EduCore V4 — Samandar uchun yangilash

Sizda PostgreSQL 18 o‘rnatilgan. Docker kerak emas.

## V3 ishlayotgan bo‘lsa

1. Eski server terminalida **Ctrl+C** bosing.
2. Eski `C:\educore` papkasini saqlang. PostgreSQL bazasi papkaning ichida emas: undan ham zaxira oling. `BACKUP.md`dagi buyruqlarni bajaring.
3. V4 ZIPni yangi papkaga oching. Ichidagi `educore` papkasini `C:\educore-v4` deb joylashtiring. `C:\educore-v4\Cargo.toml` borligini tekshiring.
4. **Eski `.env`ni yangi papkaga nusxalang.** DATABASE_URL, login yoki bazani qayta yaratish kerak emas:

```powershell
Copy-Item C:\educore\.env C:\educore-v4\.env
cd C:\educore-v4
cargo test --locked
cargo run --locked
```

Birinchi ishga tushishda `002_learning.sql` avtomatik bajariladi. Eski guruhlar, hisoblar, topshiriqlar va baholar saqlanadi. `001_core.sql`ni tahrirlamang va `_sqlx_migrations` jadvalini o‘chirmang.

5. Brauzerda **http://127.0.0.1:3000/app** oching. Eski slug, email va parolingiz bilan kiring. Kerak bo‘lsa Ctrl+F5 bosing.
6. Tekshirib bo‘lgach ham eski papka va backupni saqlang. V3 kodini V4ga migratsiya qilingan bazaga qaytarib ulash tavsiya qilinmaydi; qaytish uchun backupdan alohida bazaga tiklang.

Agar `.env`dagi `MEDIA_DIR` nisbiy yo‘l bo‘lsa, keyingi yangilashlarda `data\media`ni ham ko‘chiring. V3da audio bo‘lmaganligi sabab birinchi V4 yangilashda bu papka yo‘q bo‘lishi normal.

## V4da birinchi mashg‘ulot

1. **Kurslar → Yangi kurs**: nom, daraja, tavsif.
2. Kursda **+ Dars**: modul nomi, dars matni, ixtiyoriy HTTPS material havolasi.
3. **Guruhlar → Tahrirlash / kurs**: kursni guruhga biriktiring.
4. **Topshiriqlar → + Topshiriq**: `Speaking`, javob shakli `Audio`, kerakli dars.
5. O‘quvchi boshqa browser profili/inkognitoda kirsin. Mikrofon bilan yozsin yoki audio fayl tanlasin, keyin **Javob yuborish**.
6. **Tekshirish**: ustoz baho va yozma/audio izoh qo‘yadi. Istasa **Qayta topshirishga ruxsat berish**ni belgilaydi.
7. O‘quvchi qayta topshirgach, **O‘sish tarixi**da birinchi va hozirgi urinish yonma-yon ko‘rinadi.
8. **Kalendar**da dars qo‘shing. Bir ustoz yoki guruhga ustma-ust dars qo‘yilmaydi.

Mikrofon localhost yoki HTTPSda ishlaydi. HTTP orqali boshqa qurilmadan kirishda fayl yuklashdan foydalaning yoki HTTPS ulang. Yozuvchi 3 daqiqada to‘xtaydi; server har bir audio faylni 8 MB bilan cheklaydi.

## Birinchi marta o‘rnatish

PostgreSQL 18 xizmatini ishga tushiring. SQL Shell yoki pgAdmin orqali bir marta:

```sql
CREATE USER educore WITH PASSWORD 'BU_YERGA_UZUN_TASODIFIY_PAROL';
CREATE DATABASE educore OWNER educore;
```

PowerShell:

```powershell
cd C:\educore-v4
Copy-Item .env.example .env
notepad .env
```

DATABASE_URLdagi parolni o‘zingiz qo‘ygan parolga almashtiring. So‘ng:

```powershell
cargo run --locked
```

`http://127.0.0.1:3000/app?register=1`dan akademiya oching. Keyingi safar faqat `cargo run --locked` yetarli. `start.ps1` ham mavjud `.env` bilan mahalliy serverni ishga tushiradi.

## Xato bo‘lsa

- `DATABASE_URL kerak`: `.env` Cargo.toml yonida bo‘lsin, terminal shu papkada tursin.
- `password authentication failed`: `.env`dagi PostgreSQL paroli noto‘g‘ri. Bu kabinet paroli emas.
- `connection refused`: PostgreSQL xizmati va 5432 portini tekshiring.
- `address already in use`: eski Rust serverni Ctrl+C bilan to‘xtating.
- Terminalda eski DATABASE_URL qolgan bo‘lsa `.env`dan ustun turadi. Uni olib tashlash faylni o‘chirmaydi: `Remove-Item Env:DATABASE_URL -ErrorAction SilentlyContinue`.
- `.ps1` bloklansa, `cargo run --locked`ni to‘g‘ridan-to‘g‘ri bajaring.
