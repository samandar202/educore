param([string]$PgBin = 'C:\Program Files\PostgreSQL\18\bin')
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
. "$PSScriptRoot\env.ps1"
Load-EduCoreEnv '.env'
if ($env:MEDIA_BACKEND -eq 'supabase') { throw 'Cloud backup uchun database VA bucket nusxasi kerak. CLOUD_DEPLOY.md ni koring.' }
if (-not $env:DATABASE_URL) { throw 'DATABASE_URL kerak.' }
$dump = Join-Path $PgBin 'pg_dump.exe'
$psql = Join-Path $PgBin 'psql.exe'
if (-not (Test-Path $dump)) { throw 'pg_dump topilmadi. -PgBin orqali PostgreSQL bin papkasini korsating.' }
$dest = Join-Path (Get-Location) ('backups\' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $dest | Out-Null
$previous = $env:PGDATABASE
try {
    $env:PGDATABASE = $env:DATABASE_URL
    & $dump --format=custom --file (Join-Path $dest 'database.dump')
    if ($LASTEXITCODE -ne 0) { throw 'pg_dump bajarilmadi. Bu backup yaroqli emas.' }
    $media = if ($env:MEDIA_DIR) { $env:MEDIA_DIR } else { 'data\media' }
    $hasMedia = & $psql -X -v ON_ERROR_STOP=1 -tAc "SELECT to_regclass('public.media_files') IS NOT NULL;"
    if ($LASTEXITCODE -ne 0) { throw 'Media jadvali tekshirilmadi.' }
    if ("$hasMedia".Trim() -eq 't') {
        $entries = & $psql -X -v ON_ERROR_STOP=1 -At -F '|' -c 'SELECT id,size_bytes FROM media_files ORDER BY id;'
        if ($LASTEXITCODE -ne 0) { throw 'Media royxati olinmadi.' }
        foreach ($entry in $entries) {
            if (-not $entry) { continue }
            $parts = $entry.Split('|'); $file = Join-Path $media $parts[0]
            if (-not (Test-Path $file) -or (Get-Item $file).Length -ne [long]$parts[1]) { throw "Audio fayl yoq yoki hajmi mos emas: $($parts[0])" }
        }
        $entries | Set-Content (Join-Path $dest 'media-index.txt')
    }
    if (Test-Path $media) { Copy-Item $media (Join-Path $dest 'media') -Recurse }
    Copy-Item '.env' (Join-Path $dest 'server.env')
    'Backup completed. Keep database.dump and media together.' | Set-Content (Join-Path $dest 'COMPLETE.txt')
    Write-Host "Backup: $dest" -ForegroundColor Green
} finally { $env:PGDATABASE = $previous }
