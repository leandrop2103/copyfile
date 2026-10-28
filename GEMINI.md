# Regras de Desenvolvimento e Diretrizes do Projeto (`GEMINI.md`)

Este arquivo define os padrões obrigatórios e automáticos que o assistente de IA (Antigravity/Gemini) deve seguir **sempre que fizer qualquer alteração ou build neste repositório**.

---

## 📌 1. Regra de Versionamento Obrigatório (SemVer)
- **Toda alteração de código, nova funcionalidade, refatoração ou correção de bug DEVE obrigatoriamente incrementar o número da versão** no arquivo `Cargo.toml`.
- Exemplo: de `1.0.0` para `1.0.1` (patch) ou `1.1.0` (minor).
- **Nunca gerar uma nova compilação sem incrementar a versão**, pois o auto-atualizador (`updater.rs`) depende de versões estritamente crescentes para que os usuários detectem e recebam as atualizações automáticas via GitHub Releases.

---

## 🛠️ 2. Regra de Compilação e Qualidade de Código
Antes de publicar qualquer release:
1. **Formatação de Código:** Executar `cargo fmt`.
2. **Linter Estrito:** Executar `cargo clippy --all-targets -- -D warnings` (deve ter 0 warnings).
3. **Testes Automatizados:** Executar `cargo test --verbose` (todos os testes unitários e de integração devem passar).
4. **Build Otimizado:** Executar `cargo build --release`.

---

## 📂 3. Cópia e Sincronização dos Binários
Após o build concluído com sucesso, o binário de saída deve ser sincronizado na pasta `dist/` e na raiz:
```powershell
if (-not (Test-Path "dist")) { New-Item -ItemType Directory -Force -Path "dist" }
Copy-Item -Force target\release\copyfile.exe dist\copyfile.exe
Copy-Item -Force target\release\copyfile.exe copyfile.exe
```

---

## 🚀 4. Regra de Publicação e Release no GitHub
Sempre que concluir um ciclo de melhorias ou correção:

### Passo A: Repositório de Código-Fonte (`leandrop2103/copyfile`)
```powershell
git add Cargo.toml Cargo.lock copyfile.exe dist/copyfile.exe src/ build_and_release.ps1 AGENTS.md GEMINI.md gimini.md
git commit -m "release: v<VERSÃO> - <descrição das alterações>"
git push origin main
git tag -a v<VERSÃO> -m "Release v<VERSÃO>"
git push origin v<VERSÃO>
```

### Passo B: Criação de Release no GitHub com Binário
O executável atualizado deve ser publicado como uma nova release no repositório GitHub para alimentar o auto-updater:
```powershell
gh release create "v<VERSÃO>" "target\release\copyfile.exe" --repo "leandrop2103/copyfile" --title "CopyFile v<VERSÃO>" --notes "<Notas da versão>"
```

*(Ou simplesmente execute `.\build_and_release.ps1 -Notes "<Notas>"` para realizar todos os passos automaticamente).*

---

## 🔄 5. Arquitetura do Auto-Updater (`src/core/updater.rs`)
1. **Consulta GitHub Releases:** Verifica a API pública `https://api.github.com/repos/leandrop2103/copyfile/releases/latest`.
2. **Comparação SemVer:** Compara a tag remota com a versão embutida em tempo de compilação `env!("CARGO_PKG_VERSION")`.
3. **Download com Streaming Seguro:** Baixa o executável para `copyfile.exe.new` com callback de progresso em tempo real e validação do cabeçalho PE (assinatura `MZ`).
4. **Substituição a Quente no Windows:**
   - Renomeia `copyfile.exe` -> `copyfile.exe.old`
   - Move `copyfile.exe.new` -> `copyfile.exe`
   - Spawna o novo processo com a flag `--cleanup-old`
   - Encerra o processo anterior.
5. **Limpeza Automática:** Na inicialização em `main.rs`, `clean_old_update_files()` aguarda a liberação e remove `copyfile.exe.old`.

---

## 🦀 6. Boas Práticas e Estabilidade
1. **0 Panics e Mutex Seguro:** Nunca quebrar a aplicação com `.unwrap()` em fluxos de rede ou concorrência.
2. **Interface Gráfica Responsiva:** Todas as operações de I/O, hash e rede do auto-updater devem rodar em threads separadas, mantendo a GUI sempre a 60 FPS.
3. **Clippy 100% Limpo:** Manter o projeto com 0 warnings no Clippy.
