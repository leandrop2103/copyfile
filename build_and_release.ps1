# Script de Build e Publicacao Automatica do CopyFile
param (
    [string]$Notes = "Atualizacao e melhorias de desempenho do CopyFile",
    [switch]$BumpPatch,
    [switch]$BumpMinor
)

$ErrorActionPreference = "Stop"

# 1. Carrega versao do Cargo.toml
$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
    $currentVersion = $matches[1]
    Write-Host "[*] Versao atual no Cargo.toml: v$currentVersion" -ForegroundColor Cyan
} else {
    Write-Error "Nao foi possivel detectar a versao no Cargo.toml"
    exit 1
}

# Bump de versao opcional via parametro
$newVersion = $currentVersion
if ($BumpPatch -or $BumpMinor) {
    $parts = $currentVersion.Split('.')
    $major = [int]$parts[0]
    $minor = [int]$parts[1]
    $patch = [int]$parts[2]

    if ($BumpMinor) {
        $minor++
        $patch = 0
    } elseif ($BumpPatch) {
        $patch++
    }

    $newVersion = "$major.$minor.$patch"
    Write-Host "[*] Incrementando versao: v$currentVersion -> v$newVersion" -ForegroundColor Yellow
    $updatedToml = $cargoToml -replace 'version\s*=\s*"[^"]+"', "version = `"$newVersion`""
    Set-Content -Path "Cargo.toml" -Value $updatedToml -NoNewline
}

# 2. Formatacao e testes
Write-Host "[*] Executando cargo fmt e testes..." -ForegroundColor Cyan
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --verbose

# 3. Compilacao de Producao Release
Write-Host "[*] Compilando release otimizada (LTO Fat + Strip)..." -ForegroundColor Yellow
cargo build --release

$outputBinary = "target\release\copyfile.exe"
if (-not (Test-Path $outputBinary)) {
    Write-Error "Binario de saida nao encontrado em $outputBinary"
    exit 1
}

# 4. Sincroniza executaveis na raiz e em dist/
Write-Host "[*] Sincronizando executaveis..." -ForegroundColor Cyan
if (-not (Test-Path "dist")) {
    New-Item -ItemType Directory -Force -Path "dist" | Out-Null
}
Copy-Item -Force $outputBinary "dist\copyfile.exe"
Copy-Item -Force $outputBinary "copyfile.exe"

# 5. Publicacao no Git (lpl2103/copyfile)
Write-Host "[*] Enviando alteracoes para git (lpl2103/copyfile)..." -ForegroundColor Cyan
git add Cargo.toml Cargo.lock copyfile.exe dist/copyfile.exe src/ build_and_release.ps1 AGENTS.md GEMINI.md
git commit -m "release: v$newVersion - $Notes"
git push origin main

# 6. Criacao e envio da Tag
git tag -a "v$newVersion" -m "Release v$newVersion"
git push origin "v$newVersion"

# 7. Publicacao da Release no GitHub
Write-Host "[*] Criando release v$newVersion no repositorio lpl2103/copyfile..." -ForegroundColor Cyan
gh release create "v$newVersion" "target\release\copyfile.exe" --repo "lpl2103/copyfile" --title "CopyFile v$newVersion" --notes "$Notes"

Write-Host "`n[SUCESSO] Build e publicacao da versao v$newVersion finalizados com sucesso!" -ForegroundColor Green
