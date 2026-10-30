# EduCore V5.1 — Claude Opus 5.5

Bu paket V5 AI studiyasiga Anthropic’ning rasmiy Messages API ulanishini qo‘shadi. Dars rejasi, mashq/test, yozma izoh va speaking faoliyati mavjud. Qoralamani tekshirish va guruhga yuborish ustozning qarori bo‘lib qoladi.

## 1. V5dan yangilash

1. Serverni Ctrl+C bilan to‘xtating. Eski papkani saqlang, DB va audio uchun zaxira oling.
2. ZIPni yangi papkaga oching; ichidagi `educore` papkasiga kiring.
3. Oldingi ishlayotgan `.env`ni shu papkaga nusxalang. `DATABASE_URL`, Supabase va media sozlamalarini saqlang. `.env.example`ni ishlayotgan `.env` ustidan ko‘chirmang.
4. Mahalliy audio ishlatsangiz, eski `data\media`ni nusxalang yoki MEDIA_DIRni eski papkaga yo‘naltiring. Supabase’da o‘sha bucket sozlamalari qoladi.

V5.1 DB sxemasini o‘zgartirmaydi. 001–004 migratsiyalar avvalgidek saqlangan. Agar V4dan kelsangiz, V5_BOSHLASH.md dagi migratsiya yo‘lini bajaring.

## 2. Kalit olish

[Claude API Keys](https://platform.claude.com/settings/keys) sahifasiga kiring. `Create key` orqali kalit yarating, kerakli workspace’ni tanlang va kalitni xavfsiz saqlang. Shaxsiy sinov uchun personal key, umumiy xizmat uchun tegishli huquqli service account key ishlatiladi.

Claude API pullik; tarif, balans va xarajat limitini hisobingizda tekshiring. Claude chat obunasi alohida xizmat. API kalitini chatga, GitHub’ga, brauzer JS kodiga yoki ZIPga qo‘ymang.

## 3. Mahalliy sozlash — Windows PowerShell

`Cargo.toml` turgan papkada:

```powershell
Test-Path .\Cargo.toml
Test-Path .\.env
notepad .env
```

Mavjud AI qatorlarini quyidagicha yangilang. Bir nomdagi o‘zgaruvchini ikki marta yozmang:

```dotenv
AI_PROVIDER=anthropic
ANTHROPIC_API_KEY=olgan_kalitingizni_shu_yerga_yozing
AI_MODEL=claude-opus-5-5
AI_DAILY_LIMIT=10
```

Kalit workspace’ga biriktirilmagan bo‘lsa, Claude Console’dagi workspace IDni qo‘shing:

```dotenv
ANTHROPIC_WORKSPACE_ID=workspace_id
```

Ctrl+S bilan saqlang. Oldin PowerShellda AI sozlamalarini `$env:...` orqali o‘rnatgan bo‘lsangiz, ular `.env`dan ustun turadi. Quyidagisi joriy terminaldagi AI override’larni olib tashlaydi; fayldagi sozlamalar qoladi:

```powershell
Remove-Item Env:AI_PROVIDER, Env:AI_MODEL, Env:ANTHROPIC_API_KEY, Env:ANTHROPIC_WORKSPACE_ID, Env:AI_DAILY_LIMIT -ErrorAction SilentlyContinue
cargo run --locked -j 1
```

http://127.0.0.1:3000/app → rahbar yoki ustoz → Ustoz AI. Statusda `Anthropic Claude · claude-opus-5-5 · Kalit sozlangan` ko‘rinadi. Bu kalitning serverda borligini bildiradi; uning haqiqiyligi birinchi so‘rovda tekshiriladi.

Sinov: dars rejasi → Present Simple → A1 → bolalar → 45 daqiqa. Avval o‘quvchi matni va ustoz izohini o‘qing. Javoblar kaliti o‘quvchi topshirig‘iga qo‘shilmasin.

## 4. Render’da

Yangi manba kodini GitHubga yuklang. Render xizmati → Environment’da yuqoridagi AI_PROVIDER, ANTHROPIC_API_KEY, AI_MODEL, AI_DAILY_LIMITni belgilang. Zarur bo‘lsa ANTHROPIC_WORKSPACE_ID ham qo‘shing. Saqlab redeploy qiling. GitHub’dagi `.env.example`ga haqiqiy kalitni yozmang.

## 5. Limit va xatolar

- Standart: ustozga kuniga 10, akademiyaga jami 50 urinish. Kun UTC bo‘yicha yangilanadi. Bu token yoki pul limiti emas.
- Claude so‘rovida `max_tokens=4096`, `output_config.effort=low`, JSON Schema va API versiyasi `2026-10-21` ishlatiladi. Thinking o‘chirilmaydi; thinking ham token sarflashi mumkin. Thinking bloklari saqlanmaydi yoki brauzerga yuborilmaydi.
- Ulanishda kalit yo‘q bo‘lsa AI tugmasi o‘chiq bo‘ladi. Boshqa bo‘limlar ishlaydi.
- `AI_MODEL ... mos emas`: oldingi `openai/gpt-oss-20b` qatorini `claude-opus-5-5`ga almashtiring va restart qiling.
- Kalit/model/balans xatosida Claude Console’dagi ruxsat, workspace, balans, kalit muddati va modelni tekshiring.
- `429`: provayder kvotasi tugagan; keyinroq urinib ko‘ring. Ilovaning kunlik limiti alohida.
- 90 soniyadan keyin aloqa uzilishi mumkin. Xato so‘rov ham provayderda hisoblanishi mumkin; ilova avtomatik qayta yubormaydi yoki boshqa modelga o‘tmaydi. Qoralamalar tarixini tekshiring.
- Haqiqiy kalit bilan jonli so‘rov ushbu paket tayyorlanayotganda bajarilmadi. HTTP mock testlari format, sarlavhalar, javobni o‘qish va xato himoyasini tekshiradi.

Groq’ni qayta tanlash: `AI_PROVIDER=groq`, `GROQ_API_KEY=...`, `AI_MODEL=openai/gpt-oss-20b`. Eski V5 `.env`ida AI_PROVIDER bo‘lmasa Groq ishlaydi. Provayderlar o‘rtasida avtomatik o‘tish yo‘q.

Rasmiy manbalar (2026-10-30):
- https://platform.claude.com/docs/en/get-api-key
- https://platform.claude.com/docs/en/models/opus-5-5/overview
- https://platform.claude.com/docs/en/api/overview
- https://platform.claude.com/docs/en/api/versioning
- https://platform.claude.com/docs/en/build-with-claude/structured-outputs
- https://platform.claude.com/docs/en/build-with-claude/effort
