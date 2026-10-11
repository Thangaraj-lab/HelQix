$ErrorActionPreference = 'Stop'
$gatewayDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = (Resolve-Path (Join-Path $gatewayDir '..\..')).Path
$venvPython = Join-Path $gatewayDir '.venv\Scripts\python.exe'
if (-not (Test-Path $venvPython)) {
  python -m venv (Join-Path $gatewayDir '.venv')
}
& $venvPython -m pip install --upgrade pip
& $venvPython -m pip install -r (Join-Path $gatewayDir 'requirements.txt')
