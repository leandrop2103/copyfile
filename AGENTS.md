# Regras do Projeto (AGENTS.md)

Este repositório segue estritamente as diretrizes definidas em [GEMINI.md](file:///d:/src/copyfile/GEMINI.md):

1. **Versionamento Obrigatório (SemVer):** Toda alteração de código, melhoria, correção de bug ou build EXIGE incremento de versão no `Cargo.toml` (ex: de `1.0.0` para `1.0.1`).
2. **Build de Produção:** Utilizar sempre `cargo build --release` (otimizado com LTO Fat, codegen-units = 1 e strip).
3. **Cópia e Sincronização de Binários:** Sincronizar `target\release\copyfile.exe` em `dist\copyfile.exe` e `copyfile.exe` na raiz.
4. **Publicação Automática e Tags no GitHub:**
   - Commit + Push + Tag `v<VERSÃO>` no repositório de código `lpl2103/copyfile`.
   - Release com o asset `target/release/copyfile.exe` via `gh release create "v<VERSÃO>" ...`.
5. **Auto-Updater Funcional:** O sistema de auto-atualização (`src/core/updater.rs`) consulta as releases do GitHub (`lpl2103/copyfile`) e substitui o executável a quente com verificação de integridade PE e `--cleanup-old`.
