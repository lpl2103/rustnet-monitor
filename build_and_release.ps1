# Script de Compilacao Otimizada, Versionamento e Publicacao Automatica no GitHub Releases
param (
    [string]$Notes = "Atualizacao e melhorias no RustNet Monitor",
    [switch]$SkipBump = $false
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "     RustNet Monitor - Build, Bump & GitHub Release       " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Carrega e incrementa a versao exclusivamente na secao [package] do Cargo.toml (SemVer Patch)
$cargoFile = "Cargo.toml"
$lines = Get-Content $cargoFile
$inPackage = $false
$version = $null
$newLines = @()

foreach ($line in $lines) {
    if ($line -match '^\s*\[package\]') {
        $inPackage = $true
        $newLines += $line
    } elseif ($inPackage -and $line -match '^\s*\[') {
        $inPackage = $false
        $newLines += $line
    } elseif ($inPackage -and $line -match '^\s*version\s*=\s*"(\d+)\.(\d+)\.(\d+)"') {
        $major = [int]$matches[1]
        $minor = [int]$matches[2]
        $patch = [int]$matches[3]
        if (-not $SkipBump) {
            $patch++
            $version = "$major.$minor.$patch"
            $newLines += "version = `"$version`""
            Write-Host "[1/6] Versao incrementada automaticamente para: v$version" -ForegroundColor Green
        } else {
            $version = "$major.$minor.$patch"
            $newLines += $line
            Write-Host "[1/6] Utilizando versao atual: v$version" -ForegroundColor Yellow
        }
    } else {
        $newLines += $line
    }
}

if (-not $version) {
    Write-Error "Nao foi possivel detectar o padrao de versao em [package] no Cargo.toml"
    exit 1
}

Set-Content -Path $cargoFile -Value $newLines

# 2. Compilacao em modo Release com perfil ultra-otimizado (LTO, strip, panic abort, opt-level z)
Write-Host "[2/6] Compilando binario em modo Release otimizado..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Falha na compilacao com cargo build --release!"
    exit 1
}

$sourceExe = "target\release\rustnet-monitor.exe"
if (-not (Test-Path $sourceExe)) {
    Write-Error "Binario de release nao encontrado em $sourceExe"
    exit 1
}

# 3. Copia do binario para a raiz do projeto (RustNetMonitor.exe)
$destExe = "RustNetMonitor.exe"
Write-Host "[3/6] Sincronizando binario na raiz do projeto ($destExe)..." -ForegroundColor Yellow
Copy-Item -Path $sourceExe -Destination $destExe -Force

$fileInfo = Get-Item $destExe
$sizeKb = [math]::Round($fileInfo.Length / 1KB, 2)
$sizeMb = [math]::Round($fileInfo.Length / 1MB, 2)
Write-Host "      Tamanho final do executavel: $sizeKb KB ($sizeMb MB)" -ForegroundColor Green

# 4. Commit e sincronizacao no Git (Branch main)
Write-Host "[4/6] Efetuando commit e push no Git (origem: lpl2103/rustnet-monitor)..." -ForegroundColor Yellow
git add Cargo.toml Cargo.lock RustNetMonitor.exe src/ assets/ config.toml README.md AGENTS.md GEMINI.md build.ps1 build_and_release.ps1 build.rs .gitignore
git commit -m "release: v$version - $Notes"
git push origin main

# 5. Criacao da Tag no Git
Write-Host "[5/6] Criando e enviando Tag v$version..." -ForegroundColor Yellow
git tag -a "v$version" -m "Release v$version"
git push origin "v$version"

# 6. Publicacao da Release no GitHub Releases via GitHub CLI (gh)
Write-Host "[6/6] Publicando Release no GitHub Releases..." -ForegroundColor Yellow
gh release create "v$version" $destExe --repo "lpl2103/rustnet-monitor" --title "RustNet Monitor v$version" --notes "$Notes"

Write-Host "==========================================================" -ForegroundColor Green
Write-Host " [SUCESSO] Versao v$version publicada no GitHub Releases!" -ForegroundColor Green
Write-Host " Executavel disponivel para auto-atualizacao: $destExe" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Cyan
