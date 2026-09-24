# Build script para RustNet Monitor
# Compila em modo release otimizado e copia o executavel para a raiz do projeto

param (
    [switch]$Debug = $false
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  RustNet Monitor - Build & Package" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

if ($Debug) {
    Write-Host "[1/3] Compilando em modo Debug..." -ForegroundColor Yellow
    cargo build
    $sourceExe = "target\debug\rustnet-monitor.exe"
} else {
    Write-Host "[1/3] Compilando em modo Release otimizado..." -ForegroundColor Yellow
    cargo build --release
    $sourceExe = "target\release\rustnet-monitor.exe"
}

if (-not (Test-Path $sourceExe)) {
    Write-Error "Arquivo executavel nao encontrado em: $sourceExe"
}

$destExe = "RustNetMonitor.exe"
Write-Host "[2/3] Copiando binario para a raiz do projeto: $destExe..." -ForegroundColor Yellow
Copy-Item -Path $sourceExe -Destination $destExe -Force

$fileInfo = Get-Item $destExe
$sizeKb = [math]::Round($fileInfo.Length / 1KB, 2)
$sizeMb = [math]::Round($fileInfo.Length / 1MB, 2)

Write-Host "[3/3] Sucesso!" -ForegroundColor Green
Write-Host "Binario gerado na raiz: $destExe ($sizeKb KB / $sizeMb MB)" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
