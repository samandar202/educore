# V2 SQLite importi

## V2 ma’lumotlarini saqlab ko‘chirish

1. V2 serverni to‘xtating va butun eski papkadan zaxira nusxa oling. Ayniqsa `academy.db` va qolgan `academy.db-wal` fayllarini yo‘qotmang.
2. V4da **yangi, guruhlari bo‘sh akademiya** oching.
3. V4 serverini Ctrl+C bilan to‘xtating.
4. V4 papkasidagi PowerShell terminalida eski bazaning haqiqiy yo‘lini kiriting:

```powershell
$env:IMPORT_V2_PATH = 'C:\Users\okhun\english_academy\academy.db'
$env:IMPORT_ACADEMY_SLUG = 'samandar-academy'
cargo run --locked -- --import-v2
cargo run --locked
```

Agar V2 bazasi ichki papkada bo‘lsa, yo‘lni shunga moslang. Import eski faylni **read-only** ochadi; barcha ko‘chirish bitta PostgreSQL tranzaksiyasida bajariladi. Xatoda yarim ko‘chirish qolmaydi. Bir akademiyaga takroriy import rad etiladi. Manba fayl yo‘q bo‘lsa yangi SQLite yaratmaydi.

**Ko‘chadi:** guruhlar, o‘quvchilar, topshiriqlar, javoblar, baholar, fikrlar, arizalar va ularning vaqt/status ma’lumotlari. V2da yo‘q maydonlar: daraja `A1`, ko‘nikma `writing`, ustoz — yangi akademiya rahbari sifatida boshlang‘ich belgilanadi. Bu haqiqiy diagnostika natijasi emas.

**Kirish tartibi o‘zgaradi:** import o‘quvchisining email/login’i `student-ESKI_ID@import.local` bo‘ladi. Parol dastlab o‘chirilgan, shu sabab u kira olmaydi. Rahbar **Hisoblar → Parol yangilash** orqali parol belgilaydi. Bu login email yetkazish xizmatiga ulanmagan. V2dagi ota-ona kodi shaxsning ismini/emailini o‘z ichiga olmagan, shuning uchun ota-ona hisoblarini V4da qayta yarating. Eski tokenlar ko‘chirilmaydi.
