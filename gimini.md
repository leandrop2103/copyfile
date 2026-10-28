# Regras de Desenvolvimento e Diretrizes do Projeto (`GEMINI.md` / `gimini.md`)

Este arquivo define os padrões obrigatórios e automáticos que o assistente de IA (Antigravity/Gemini) deve seguir **sempre que fizer qualquer alteração ou build neste repositório**.

Para o documento completo, consulte [GEMINI.md](file:///d:/src/copyfile/GEMINI.md) e [AGENTS.md](file:///d:/src/copyfile/AGENTS.md).

---

## 📌 1. Regra de Versionamento Obrigatório (SemVer)
- **Toda alteração de código, nova funcionalidade, refatoração ou correção de bug DEVE obrigatoriamente incrementar o número da versão** no arquivo `Cargo.toml`.
- Exemplo: de `1.0.0` para `1.0.1` (patch) ou `1.1.0` (minor).
- **Nunca gerar uma nova compilação sem incrementar a versão**, pois o auto-atualizador (`updater.rs`) depende de versões estritamente crescentes para que os usuários detectem e recebam as atualizações automáticas via GitHub Releases.

---

## 🛠️ 2. Regra de Compilação e Qualidade de Código
Antes de publicar qualquer release:
1. `cargo fmt`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --verbose`
4. `cargo build --release`

---

## 📂 3. Cópia e Sincronização dos Binários
```powershell
if (-not (Test-Path "dist")) { New-Item -ItemType Directory -Force -Path "dist" }
Copy-Item -Force target\release\copyfile.exe dist\copyfile.exe
Copy-Item -Force target\release\copyfile.exe copyfile.exe
```

---

## 🚀 4. Regra de Publicação e Release no GitHub
```powershell
.\build_and_release.ps1 -Notes "<Notas da versão>"
```
Ou manualmente:
```powershell
git add Cargo.toml Cargo.lock copyfile.exe dist/copyfile.exe src/ build_and_release.ps1 AGENTS.md GEMINI.md gimini.md
git commit -m "release: v<VERSÃO> - <descrição das alterações>"
git push origin main
git tag -a v<VERSÃO> -m "Release v<VERSÃO>"
git push origin v<VERSÃO>
gh release create "v<VERSÃO>" "target\release\copyfile.exe" --repo "leandrop2103/copyfile" --title "CopyFile v<VERSÃO>" --notes "<Notas da versão>"
```
