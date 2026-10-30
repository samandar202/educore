# V4 Cloud (0.4.1)

Bulutga chiqarish: [CLOUD_DEPLOY.md](CLOUD_DEPLOY.md). Quyidagi eski V4 yo‘riqnomasi mahalliy foydalanish uchun.

# EduCore V4 — Learning Studio

Rust + PostgreSQL asosidagi akademiya boshqaruvi va o‘quv jarayoni. Windows + mahalliy PostgreSQL 18 uchun qadamlar: **BOSHLASH.md**. Ishga tushirish: `.env`ni sozlang, `cargo run --locked`, `http://127.0.0.1:3000/app`.

## V4da ishlaydi

- Speaking Studio: mikrofon orqali yozish yoki WebM/Ogg/WAV/MP3/M4A yuklash. Ustoz yozma yoki audio izoh bilan baholaydi. Har fayl 8 MB, standart akademiya kvotasi 200 MB.
- Qayta topshirishga ustoz ruxsati. Har qayta yuborilganda oldingi matn, audio, baho va izoh o‘zgarmas tarixga olinadi. Birinchi/hozirgi natijani solishtirish, browserdan chop etish.
- Kurs → nomlangan modul → tartiblangan dars. Matnli material va HTTPS havolalar. Kursni guruhga, darsni topshiriqqa bog‘lash. Kurs muallifi yoki rahbar tahrirlaydi; akademiya ustozlari kurslar kutubxonasini ko‘radi.
- Haqiqiy sana/vaqtli haftalik kalendar. Guruh yoki ustoz band bo‘lsa 409 qaytadi. Akademiya qatori tranzaksiya qulfi parallel so‘rovlarni ketma-ket tekshiradi. Yonma-yon tugash/boshlanish ruxsat etiladi. Bitta dars 8 soatdan uzun emas.
- Guruh, hisob va topshiriqni tahrirlash/arxivlash. O‘quvchini guruhlar orasida ko‘chirish. Arxivlangan hisob kira olmaydi; eski ishlar saqlanadi. Ustozni almashtirishdan avval uning kelajak va davom etayotgan darslari bekor qilinadi.
- Yangi topshiriq, baho, jadval o‘zgarishi haqida kabinet ichida bildirishnomalar; o‘quvchi va bog‘langan ota-onaga. Muddati ertaga bo‘lgan bajarilmagan topshiriq uchun o‘quvchiga takrorlanmaydigan eslatma (Asia/Tashkent sanasi).
- Email tasdiqlash va unutilgan parolni tiklash: 30 daqiqalik, bir martalik xeshlangan tokenlar. Email sozlanmasa hisoblar odatdagidek ishlaydi, tiklash xizmatining o‘chirilgani ko‘rsatiladi. Rahbar boshqa hisob parolini qo‘lda yangilashi mumkin.
- Faol sessiyalarni ko‘rish va yopish. Arxivlash/parol almashishi sessiyalarni bekor qiladi.
- PostgreSQLda saqlanuvchi email navbati, STARTTLS SMTP, qayta urinishlar; lokal sinovda xatni faylga yozish. Ichki eslatmalar serverdagi fon vazifasida ishlaydi.
- Kurs, topshiriq va tekshirish ro‘yxatlarida qidiruv va 12 tadan sahifalash. Guruhlar/hisoblar ham qidiriladi.
- V3 imkoniyatlari: akademiyalar ajratilishi, 4 rol, Argon2id, davomad, qabul arizalari, bilim xaritasi, audit, V2 importi.

## Saqlash va sozlamalar

`.env` misoli `.env.example`da. URL parolida maxsus belgilar bo‘lsa percent-encoding ishlating. V3dan yangilashda **eski `.env`ni saqlang**, bazani qayta yaratmang. Migratsiyalar avtomatik va tranzaksion. `001_core.sql` o‘zgarmagan.

| O‘zgaruvchi | Standart / vazifasi |
|---|---|
| DATABASE_URL | PostgreSQL ulanishi, majburiy |
| BIND_ADDR | 127.0.0.1:3000 |
| COOKIE_SECURE | false lokal; HTTPSda true |
| DB_POOL_SIZE | 10; 1–100 |
| MEDIA_DIR | data/media, audio fayllar uchun doimiy papka |
| MEDIA_QUOTA_MB | 200, har akademiyaga |
| APP_BASE_URL | http://127.0.0.1:3000; xatlardagi manzil |
| MAIL_MODE | disabled, local yoki smtp |
| MAIL_DIR | data/mail, faqat lokal xat sinovi |
| SMTP_HOST / SMTP_PORT | SMTP serveri / 587, STARTTLS |
| SMTP_FROM | tasdiqlangan yuboruvchi email |
| SMTP_USER / SMTP_PASSWORD | provayder talab qilgan hisob |

`MAIL_MODE=local` email yubormaydi: havola serverdagi `data/mail/*.txt`da paydo bo‘ladi. Bu fayllar kabinetdan berilmaydi. Lokal rejimni faqat ishlab chiqishda ishlating. `smtp`da xat navbatga qo‘yiladi; yetkazish kafolatlanmaydi, 5 urinishdan keyin `failed`. Navbat yozuvlari 7 kundan keyin tozalanadi. Sent xatning tokenli body qismi bazadan olib tashlanadi. Local fayllarni administrator qo‘lda tozalaydi. Tekshiruvda tashqi email yuborilmagan.

Audio URLlari cookie orqali har safar ruxsat tekshiradi. Faqat yuboruvchi, tegishli ustoz/rahbar va bog‘langan ota-ona tegishli ishni eshitadi. O‘qituvchi audio izoh uchun o‘zi yuklagan fayldan foydalanadi. Fayl nomi UUID; foydalanuvchi yo‘li qabul qilinmaydi. Fayl signaturasi va MIME tekshiriladi; bu to‘liq audio dekodlash yoki zararli fayl skaneri emas.

Kvotaga hali topshiriqqa biriktirilmagan yuklamalar ham kiradi; V4 avtomatik media o‘chirmaydi. Zarur bo‘lsa kvotani oshiring. MEDIA_DIR va bazani birga zaxiralang. Diskda qolgan yetim fayllarni o‘chirishdan avval joriy va tarixiy havolalarni tekshirish kerak.

## Internetga joylashtirish

Cloud 0.4.1 uchun [CLOUD_DEPLOY.md](CLOUD_DEPLOY.md): Render + private Supabase Storage. Local backend uchun doimiy MEDIA_DIR diski talab qilinadi. Haqiqiy deploy hali bajarilmagan.

Mikrofon localhost yoki HTTPS talab qiladi. Yozuvchi 3 daqiqada avtomatik to‘xtaydi, yuklangan fayllarda server faqat bayt hajmini cheklaydi. Brauzer formatni qo‘llamasa mos formatda qayta yuklang. Bolalar audiosini yozish va saqlashni ota-ona bilan kelishing.

## Chegaralar

- Bu funksional pilot; katta ishlab chiqarish yuklamasi, ko‘p serverli deploy yoki to‘liq xavfsizlik auditi bajarilmagan.
- Frontend sahifalash hozir browserda. Bootstrap ruxsatli ma’lumotlarni bir javobda oladi; katta akademiyalar uchun server sahifalashi va alohida dashboard agregatlari keyingi ish. Bildirishnoma 100, audit 100, ariza 500 bilan chegaralangan.
- Modullar nom bilan guruhlanadi; alohida modul boshqaruvi, video hosting, faylli PDF materiallar, real-time chat, AI baholash, to‘lovlar yo‘q.
- V3dan oldingi javob urinishlarini qayta tiklab bo‘lmaydi. Tarix V4da qayta yuborishdan boshlanadi. Bir revision ustoz tomonidan qayta baholansa, joriy baho yangilanadi; bu baholar tahririning to‘liq versiyalash tizimi emas.
- Arxivlash o‘chirish emas. Talaba boshqa guruhga o‘tganda oldingi ishini tarixda ko‘radi; eski guruh ustozi o‘sha topshiriqdagi ishni tekshirish ro‘yxatida saqlab qoladi.
- Email tasdiqlash kirish uchun majburiy emas. Bu mavjud va ota-ona orqali boshqariladigan hisoblarni bloklamaslik uchun.
- Calendar xonalar bandligi, takrorlanuvchi darslar yoki ta’tilni avtomatik hisoblamaydi.

## Testlar

```powershell
cargo test --locked
```

Oddiy testlar mustaqil ishlaydi. Integratsiya testlari alohida **test bazasi** talab qiladi; real bazada ishlatmang:

```powershell
$env:TEST_DATABASE_URL = 'postgres://educore:PAROL@127.0.0.1:5432/educore_test'
cargo test --locked -- --ignored --test-threads=1
```

Tekshiruv natijalari va chegaralari `VALIDATION.md`da. Zaxira va tiklash: `BACKUP.md`. V2 SQLite ko‘chirish: `V2_IMPORT.md`. Docker ixtiyoriy: `.env`dagi POSTGRES_PASSWORD va URL mos bo‘lsa `docker compose up -d --wait db`, keyin Cargo. Mahalliy PostgreSQL va Docker bir portni bir vaqtda ishlatmasin.
