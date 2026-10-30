$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust/Cargo topilmadi. Rust terminalini qayta oching.' }
if (-not (Test-Path '.env')) { throw '.env yoq. BOSHLASH.md boyicha .env.example dan nusxa oling va PostgreSQL parolini kiriting.' }
Write-Host 'Mahalliy PostgreSQL xizmati ishlayotgan bolsin. Docker talab qilinmaydi.'
Write-Host 'EduCore V4: http://127.0.0.1:3000/app' -ForegroundColor Green
& cargo run --locked
if ($LASTEXITCODE -ne 0) { throw 'Server xatosi: yuqoridagi xabarni tekshiring.' }
