> Supabase cloud backupi uchun CLOUD_DEPLOY.md ni o‘qing. Bu skript mahalliy media uchun.

# PostgreSQL va audiolarni birga zaxiralash

**Avval Rust serverni Ctrl+C bilan to‘xtating.** Shunda database.dump va audio nusxalari bir holatni aks ettiradi. PostgreSQL xizmati ishlashda qolsin.

```powershell
cd C:\educore-v4
.\scripts\backup.ps1
```

Skript PostgreSQL 18 `pg_dump`dan foydalanadi. Boshqa joyga o‘rnatilgan bo‘lsa `-PgBin 'D:\PostgreSQL\18\bin'` bering. `backups\sana-vaqt` ichida database.dump, mavjud bo‘lsa media, server.env va COMPLETE.txt yaratiladi. COMPLETE.txt bo‘lmasa backup tugallanmagan. `server.env` parol saqlaydi: backupni ommaga yubormang, alohida xavfsiz diskda saqlang. So‘ng `cargo run --locked` bilan serverni yoqing.

## Alohida bazaga tiklab tekshirish

Asosiy bazani o‘chirmang yoki uning ustiga tiklamang. SQL Shell/pgAdminda yangi bo‘sh baza yarating:

```sql
CREATE DATABASE educore_restore_test OWNER educore;
```

```powershell
.\scripts\restore-check.ps1 -Backup '.\backups\SIZNING_SANA_VAQT' -TargetDatabaseUrl 'postgres://educore:PAROL@127.0.0.1:5432/educore_restore_test' -TargetMedia 'C:\educore-restore-media'
```

Skript mavjud jadvalli bazani va mavjud media papkasini rad etadi. Jadval sonlarini ko‘rsatgach, **yangi PowerShell terminalida**:

```powershell
cd C:\educore-v4
$env:DATABASE_URL = 'postgres://educore:PAROL@127.0.0.1:5432/educore_restore_test'
$env:MEDIA_DIR = 'C:\educore-restore-media'
$env:BIND_ADDR = '127.0.0.1:3001'
$env:MAIL_MODE = 'disabled'
cargo run --locked
```

`http://127.0.0.1:3001/app`da eski login, guruhlar, baholar, tarix va bir nechta audio faylni tekshiring. Tekshiruv tugagach Ctrl+C va shu terminalni yoping. Asosiy terminal/baza sozlamasi o‘zgarmaydi.

V3 backupida media bo‘lmasligi normal. Tiklangan V3 bazasi V4 kodi yoqilganda avtomatik migratsiya qilinadi. Serverdagi barcha akademiyalar bitta database.dumpga kiradi; bu bitta akademiyani alohida export/import qilish vositasi emas.

Windows PowerShell va native pg_dump/pg_restore skriptlari ushbu Linux muhitida bajarilmadi. Shu tekshiruvni o‘z qurilmangizda yakunlang. `VALIDATION.md` sinov chegaralarini ko‘rsatadi.
