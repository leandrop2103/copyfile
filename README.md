<div align="center">

# ⚡ CopyFile (v1.0.0)

**Utilitário de Cópia, Backup e Sincronização de Arquivos em Massa de Altíssimo Desempenho para Windows.**

Construído em **Rust**, com aceleração multithread via **Rayon**, verificação criptográfica **xxHash3 (XXH3)**, suporte a **Direct I/O**, interface nativa moderna em **egui** e integração total com o **Windows Explorer**.

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg?style=flat&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6.svg?logo=windows)](https://microsoft.com/windows)
[![Build Status](https://github.com/leandrop2103/copyfile/actions/workflows/build-release.yml/badge.svg)](https://github.com/leandrop2103/copyfile/actions/workflows/build-release.yml)

---

</div>

## 🌟 Principais Recursos

### 🚀 Desempenho Extremo
- **Pipeline Paralelo Rayon**: Distribuição dinâmica de trabalho entre múltiplos núcleos de CPU com buffers otimizados em streaming contínuo.
- **Direct I/O Automático**: Em arquivos grandes (>64 MB), ignora o cache de RAM do Windows para evitar saturação de memória e maximizar a taxa real de leitura/gravação no barramento NVMe / SSD / HD.
- **Integridade Instantânea com xxHash3**: Validação de integridade bit-a-bit a mais de 30 GB/s por núcleo, garantindo que nenhum arquivo seja corrompido durante a transferência.

### 🔄 Modos de Operação
1. **Copiar (Manter Origens)**: Cópia paralela de alto desempenho preservando arquivos de origem.
2. **Mover / Recortar Seguro**: Só deleta o arquivo de origem após a cópia e a validação do hash xxHash3 no destino serem 100% concluídas com sucesso.
3. **Sincronizar (Espelho / Mirror)**: Mantém o destino perfeitamente espelhado à origem, copiando arquivos novos/alterados e removendo automaticamente arquivos obsoletos que foram excluídos da origem.

### 🛡️ Resiliência & Reprocessamento de Erros
- **🔁 Reprocessamento Rápido em 1 Clique (*Retry Failed Files*)**: Caso ocorra erro por bloqueio de antivírus ou permissão, repete apenas os arquivos que falharam sem precisar reiniciar todo o lote.
- **Sobrescrita Inteligente**:
  - *Substituir se mais recente (Timestamp)*.
  - *Verificar xxHash: Pular se idêntico, substituir se diferente*.
  - *Sempre substituir*.
  - *Pular existentes*.
  - *Substituir se tamanho/data diferir*.
- **Preservação Completa de Metadados**: Copia data de criação, última modificação, último acesso e atributos de arquivo nativos do Windows (`Read-Only`, `Hidden`, `System`, `Archive`).

### 📊 Monitoramento & Gráficos em Tempo Real
- **📈 Gráfico de Velocidade Interativo**: Traçado em tempo real com taxa atual (MB/s), média global, marcador de pico máximo e escala dinâmica.
- **Barra de Tarefas do Windows**: Indicador de progresso nativo no ícone da barra de tarefas com estados Verde (Normal), Amarelo (Pausado) e Vermelho (Erro).
- **🔔 Notificações Nativas (Windows Toast)**: Alerta sonoro discreto e notificação ao finalizar backups demorados em segundo plano.

### 📑 Relatórios de Auditoria
- **🌐 Relatório Web HTML5**: Relatório visual independente com barra de busca em tempo real, status e hashes xxHash3 de todos os arquivos.
- **📊 Planilha CSV (UTF-8 BOM)**: Compatibilidade imediata com Microsoft Excel sem problemas de codificação ou acentuação.

### 🗂️ Fila Inteligente de Transferências (*Transfer Queue*)
- Enfileiramento sequencial de múltiplos trabalhos de cópia/backup para não saturar o disco com operações simultâneas concorrentes.

---

## ⌨️ Atalhos de Teclado

| Atalho | Ação |
| :--- | :--- |
| **`F5`** ou **`Ctrl + Enter`** | Iniciar Cópia / Sincronização |
| **`Espaço`** | Pausar / Continuar transferência ativa |
| **`Esc`** | Fechar modais abertos ou Cancelar operação ativa |
| **`Ctrl + O`** | Abrir seletor de pastas de origem |
| **`Ctrl + D`** | Abrir seletor de pasta de destino |
| **`Ctrl + F`** | Focar na aba Lista de Arquivos |

---

## 💻 Uso via Linha de Comando (CLI)

O CopyFile suporta execução headless via terminal:

```bash
# Copiar arquivos com verificação de integridade
copyfile --cli --sources "C:\origem\pasta1" "C:\origem\pasta2" --dest "D:\backup"

# Copiar sem verificação de hash (mais rápido)
copyfile --cli --sources "C:\origem" --dest "D:\backup" --no-verify

# Sobrescrever todos os arquivos existentes
copyfile --cli --sources "C:\origem" --dest "D:\backup" --overwrite always

# Parar na primeira falha
copyfile --cli --sources "C:\origem" --dest "D:\backup" --fail-fast

# Definir número de threads paralelas
copyfile --cli --sources "C:\origem" --dest "D:\backup" --threads 8
```

### Flags Disponíveis
| Flag | Descrição |
|---|---|
| `--cli` | Força modo linha de comando (sem interface gráfica) |
| `--sources` | Caminhos de origem (arquivos ou pastas) |
| `--dest` | Pasta de destino |
| `--threads` | Número de threads paralelas (padrão: auto) |
| `--no-verify` | Desabilita verificação xxHash3 |
| `--fail-fast` | Para na primeira falha de cópia |
| `--overwrite` | Política: `always`, `never`, `if-newer`, `if-differs` |
| `--watch-paste` | Inicia daemon de interceptação de Ctrl+V |

---

## 🖱️ Integração com Windows Explorer, Colar Inteligente & Menu de Contexto
- **⚡ Colar com CopyFile (Estilo TeraCopy)**: Ao clicar com o botão direito no espaço em branco de qualquer pasta, Área de Trabalho ou unidade de disco, selecione **`⚡ Colar com CopyFile`** para colar arquivos copiados na Área de Transferência com validação xxHash3.
- **Diálogo de Confirmação de Colar**: Permite escolher instantaneamente se deseja colar usando o **CopyFile** ou o **Windows Explorer** padrão.
- **📂 Instalação no PATH Global (`C:\Windows\copyfile.exe`)**: Na aba **⚙ Opções**, copie o executável para a pasta do sistema com 1 clique para executar o comando `copyfile` em qualquer terminal (PowerShell, Prompt de Comando, scripts `.bat`).
- **Arrastar e Soltar (*Drag & Drop*)**: Arraste pastas ou arquivos de qualquer janela do Windows Explorer e solte dentro do CopyFile para adicioná-los automaticamente.
- **Menu de Contexto de Pastas e Arquivos**: Adicione *"Copiar com CopyFile"* ao clicar com botão direito em qualquer pasta ou arquivo.

---

## 🛠️ Compilação e Execução

### Pré-requisitos
- [Rust & Cargo](https://rustup.rs/) (versão 1.80+)
- Windows 10 ou Windows 11 (x64)

### Como compilar:
```bash
# Clonar o repositório
git clone https://github.com/leandrop2103/copyfile.git
cd copyfile

# Executar em modo desenvolvimento
cargo run

# Compilar versão final de alta performance
cargo build --release
```
O executável otimizado estará disponível em `target/release/copyfile.exe`.

---

## 📜 Licença

Distribuído sob a licença **MIT**. Veja [`LICENSE`](LICENSE) para mais detalhes.

---

<div align="center">

Desenvolvido por **Leandro** em parceria com **Gemini (Google DeepMind)**.

</div>
