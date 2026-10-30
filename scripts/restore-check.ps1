param(
    [Parameter(Mandatory=$true)][string]$Backup,
    [Parameter(Mandatory=$true)][string]$TargetDatabaseUrl,
    [Parameter(Mandatory=$true)][string]$TargetMedia,
    [string]$PgBin = 'C:\Program Files\PostgreSQL\18\bin'
)
$ErrorActionPreference = 'Stop'
$Backup = (Resolve-Path $Backup).Path
if (-not (Test-Path (Join-Path $Backup 'COMPLETE.txt'))) { throw 'Tugallangan backup belgisi topilmadi.' }
if (Test-Path $TargetMedia) { throw 'TargetMedia mavjud bolmasin: yangi alohida papka tanlang.' }
$psql = Join-Path $PgBin 'psql.exe'; $restore = Join-Path $PgBin 'pg_restore.exe'
if (-not (Test-Path $psql) -or -not (Test-Path $restore)) { throw 'PostgreSQL bin papkasini tekshiring.' }
$index = Join-Path $Backup 'media-index.txt'
if (Test-Path $index) {
    foreach ($entry in Get-Content $index) {
        if (-not $entry) { continue }
        $parts = $entry.Split('|'); $file = Join-Path (Join-Path $Backup 'media') $parts[0]
        if (-not (Test-Path $file) -or (Get-Item $file).Length -ne [long]$parts[1]) { throw 'Backupdagi audio fayllar toliq emas.' }
    }
}
$previous = $env:PGDATABASE
try {
    $env:PGDATABASE = $TargetDatabaseUrl
    $count = & $psql -X -v ON_ERROR_STOP=1 -tAc "SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog','information_schema');"
    if ($LASTEXITCODE -ne 0 -or "$count".Trim() -ne '0') { throw 'Maqsad baza bosh emas yoki ulanish xato. Tiklash toxtatildi.' }
    & $restore --dbname $TargetDatabaseUrl --no-owner --no-privileges --exit-on-error (Join-Path $Backup 'database.dump')
    if ($LASTEXITCODE -ne 0) { throw 'Tiklash muvaffaqiyatsiz. Test bazani tekshiring.' }
    if (Test-Path (Join-Path $Backup 'media')) { Copy-Item (Join-Path $Backup 'media') $TargetMedia -Recurse } else { New-Item -ItemType Directory -Path $TargetMedia | Out-Null }
    & $psql -X -v ON_ERROR_STOP=1 -c 'SELECT (SELECT count(*) FROM users) AS users, (SELECT count(*) FROM submissions) AS submissions;'
    if ($LASTEXITCODE -ne 0) { throw 'Tiklangan jadvallar tekshirilmadi.' }
    Write-Host 'Alohida bazaga tiklandi. Serverni test portida ulab, kabinet va audiolarni tekshiring.' -ForegroundColor Green
} finally { $env:PGDATABASE = $previous }
