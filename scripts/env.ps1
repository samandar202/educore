# Parse .env as data, never execute it. Existing process variables take priority.
function Load-EduCoreEnv([string]$Path) {
    if (-not (Test-Path $Path)) { throw '.env topilmadi.' }
    foreach ($line in Get-Content $Path) {
        if ($line -match '^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$') {
            $key = $Matches[1]; $value = $Matches[2]
            if ($value.Length -ge 2 -and (($value.StartsWith('"') -and $value.EndsWith('"')) -or ($value.StartsWith("'") -and $value.EndsWith("'")))) { $value = $value.Substring(1, $value.Length - 2) }
            if (-not [Environment]::GetEnvironmentVariable($key, 'Process')) { [Environment]::SetEnvironmentVariable($key, $value, 'Process') }
        }
    }
}
