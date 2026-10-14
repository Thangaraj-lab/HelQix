$ErrorActionPreference = 'Stop'
$gatewayDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $gatewayDir '..\..')).Path
$env:HELQIX_WORKSPACE_ROOT = $repoRoot
if (-not $env:HELQIX_ALLOWED_ORIGINS) {
  $allowedOrigins = @(
    'http://localhost:3000',
    'http://127.0.0.1:3000',
    'http://localhost:5173',
    'http://127.0.0.1:5173',
    'http://localhost:5174',
    'http://127.0.0.1:5174',
    'http://localhost:5175',
    'http://127.0.0.1:5175',
    'http://localhost:5176',
    'http://127.0.0.1:5176',
    'http://localhost:5177',
    'http://127.0.0.1:5177',
    'http://localhost:5178',
    'http://127.0.0.1:5178',
    'http://localhost:4173',
    'http://127.0.0.1:4173'
  )
  $env:HELQIX_ALLOWED_ORIGINS = $allowedOrigins -join ','
}
Set-Location $gatewayDir
$python = Join-Path $gatewayDir '.venv\Scripts\python.exe'
if (-not (Test-Path $python)) { $python = 'python' }
& $python -m uvicorn app:app --host 127.0.0.1 --port 8091
