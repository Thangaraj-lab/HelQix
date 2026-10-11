$ErrorActionPreference = 'Stop'
$gatewayDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $gatewayDir '..\..')).Path
$env:HELQIX_WORKSPACE_ROOT = $repoRoot
Set-Location $gatewayDir
$python = Join-Path $gatewayDir '.venv\Scripts\python.exe'
if (-not (Test-Path $python)) { $python = 'python' }
& $python -m uvicorn app:app --host 127.0.0.1 --port 8091
