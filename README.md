# CopyFile ⚡

Ferramenta de cópia de arquivos e backup ultra-rápida, multiplataforma (Windows 10/11 e Linux), escrita em Rust com foco em alto desempenho, baixa latência e integridade absoluta. Inspirada na usabilidade e fluidez do **TeraCopy**.

---

## 🚀 Principais Recursos

- **Interface Gráfica Moderna (GUI)** construída com `egui/eframe`:
  - Inicialização instantânea (<100ms) e consumo mínimo de memória (<25MB de RAM).
  - Tabela virtualizada com scroll a 60+ FPS para listas com centenas de milhares de arquivos.
  - Seleção flexível de origens (múltiplos arquivos ou pastas simultaneamente) e destino único.
  - Suporte completo a **Arrastar e Soltar (Drag & Drop)** direto para a janela.
- **Identificação Visual por Cores de Status**:
  - 🟢 **Verde (`✔ Concluído`)**: Arquivos copiados e verificados com sucesso.
  - 🟡 **Laranja/Amarelo (`⚠ Ignorado`)**: Arquivos ignorados pelas regras de substituição.
  - 🔴 **Vermelho (`✖ Erro`)**: Arquivos com falha de leitura/gravação/permissão.
  - 🔵 **Azul (`🔄 Copiando`)**: Arquivo em transferência ativa.
- **Verificação Criptográfica de Integridade com xxHash3 (XXH3)**:
  - Checksums calculados com vetorização SIMD em tempo de streaming (>30 GB/s por núcleo).
  - Validação byte-a-byte do destino após a cópia.
- **Cópia Paralela Multi-Thread**:
  - Saturation da largura de banda de SSDs NVMe e HDDs externos com worker pool assíncrono.
  - Buffers reutilizáveis por thread para zero alocações na heap durante a transferência.
- **Preservação Rigorosa de Metadados**:
  - Data e hora de criação (*Birthtime/Creation Time*).
  - Data e hora de modificação e acesso (*Modified & Access Time*).
  - Atributos nativos do sistema de arquivos Windows (Somente leitura, Oculto, Sistema, Arquivo Morto).
- **Tolerância a Falhas e Logs Detalhados**:
  - Opção de **ignorar erros** para não interromper backups longos.
  - Aba de **Registro / Log** com motivo detalhado de cada falha, exportação em `.txt` e cópia para o clipboard.
- **Modo Linha de Comando (Headless CLI)**:
  - Pode ser executado em scripts e automações sem abrir a janela gráfica.

---

## 🛠 Compilação e Otimizações de Release

O projeto já está configurado com `LTO = "fat"`, `opt-level = 3`, `codegen-units = 1`, `panic = "abort"` e compilação com CRT estático no Windows.

### Compilar versão de produção (Release):
```bash
cargo build --release
```

O binário final autônomo será gerado em `target/release/copyfile.exe`.

---

## 💻 Uso via Linha de Comando (CLI)

```bash
# Cópia simples com verificação xxHash e 8 threads
copyfile.exe --cli -s "C:\MinhaPasta" "C:\Documentos" -d "E:\Backup" -t 8

# Pular arquivos existentes e silenciar erros
copyfile.exe --cli -s "D:\Projetos" -d "F:\Backup" --overwrite never
```
