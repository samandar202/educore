# EduCore V5.1 · Academy OS

Rust + PostgreSQL: ustoz AI, kurslar, speaking studio, taqvim, davomat, ota-ona kabineti, oylik hisoblar va CSV hisobot. 3D CSS interfeys, responsiv ekranlar va Reduced Motion qo‘llovi.

- [Render + Supabase + Groq orqali joylashtirish](DEPLOY_FREE_UZ.md)
- [Boshlash va V4dan yangilash](V5_BOSHLASH.md)
- [Claude kalitini ulash va V5dan yangilash](CLAUDE_ULASH.md)
- [Bulutga chiqarish](CLOUD_DEPLOY.md)
- [Tekshiruv qaydi](V5_VALIDATION.md)

Bepul Groq bilan boshlash: `AI_PROVIDER=groq`, `GROQ_API_KEY`, `AI_MODEL=openai/gpt-oss-20b`. Groq Free rejasi cheklangan kvotalar bilan ishlaydi. Ushbu repo `.env.example` va Render Blueprint’da Groqni tanlaydi.

Ixtiyoriy pullik Claude uchun serverda `AI_PROVIDER=anthropic`, `ANTHROPIC_API_KEY` va `AI_MODEL=claude-opus-5-5` sozlanadi. Claude API pullik; tarif va limitni provayder hisobida tekshiring. Oldingi Groq ulanishi ham saqlangan. Kalitsiz yoki mos kelmaydigan sozlamada sabab UI’da ko‘rsatiladi. Haqiqiy API kaliti va deploy bu paket bilan avtomatik ulanmaydi.

```powershell
cargo run --locked -j 1
```

Avval .env ichidagi PostgreSQL ulanishini sozlang. Mavjud bazaga migratsiyalar avtomatik qo‘llanadi. 001–003 fayllarni o‘zgartirmang. Eski V4 hujjatlari tarixiy ma’lumot uchun saqlangan.
