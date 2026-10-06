<p align="center">
  <img src="assets/app.png" width="128" height="128" alt="RustNet Monitor Logo" />
</p>

<h1 align="center">RustNet Monitor</h1>

<p align="center">
  <strong>Monitor de conectividade de rede nativo, moderno e ultra-leve para Windows escrito 100% em Rust.</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT" /></a>
  <img src="https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011%20(x64)-0078D6.svg?logo=windows" alt="Platform" />
  <img src="https://img.shields.io/badge/Rust-2024%20Edition-DEA584.svg?logo=rust" alt="Rust 2024" />
  <img src="https://img.shields.io/badge/GUI-eframe%20%2F%20egui-blueviolet.svg" alt="GUI: eframe/egui" />
  <img src="https://img.shields.io/badge/Database-SQLite%20(WAL%20Bundled)-003B57.svg?logo=sqlite" alt="SQLite Bundled" />
</p>

---

## 📋 Visão Geral

O **RustNet Monitor** é uma aplicação desktop nativa desenvolvida em **Rust** voltada para o monitoramento contínuo da estabilidade e qualidade da conexão de rede no Windows. Projetado sob princípios rigorosos de eficiência:

* **Baixíssimo consumo**: CPU rotineiramente abaixo de 0.1% e uso mínimo de memória RAM.
* **100% Portátil (*Portable*)**: Executável único estático (`RustNetMonitor.exe` ~7 MB), sem necessidade de instalação, instaladores MSI ou permissões de administrador.
* **Zero dependências externas**: Motor SQLite embutido diretamente no código de máquina (*bundled*), sem exigência de .NET Runtime, bibliotecas redistribuíveis do Visual C++ ou DLLs avulsas.
* **Chamadas nativas do Windows**: Todas as inspeções de adaptadores, tabelas de roteamento e envio de pacotes ICMP utilizam diretamente as APIs Win32 e IP Helper (`iphlpapi.dll`), sem gerar subprocessos externos (`ping.exe`, `ipconfig.exe`, `route.exe`, `powershell.exe`).

---

## ✨ Funcionalidades Principais

### 🖥️ Interface Gráfica Moderna (GUI)
- Desenvolvida com `eframe` (egui) acelerada por GPU (OpenGL Glow).
- **Sem janela preta do CMD**: Configurada com `#![windows_subsystem = "windows"]`, iniciando diretamente em janela limpa ao clicar no executável.
- **Ícone Nativo do Windows**: Ícone multi-resolução embutido na seção de recursos PE (`.rsrc`), exibido com fidelidade no Windows Explorer, barra de tarefas e título da janela.
- **Victor Mono Nerd Font Embutida**: Fonte profissional de alta fidelidade embutida diretamente no executável via compressão zlib (`assets/victor_mono.deflate`), com suporte nativo a glifos, ícones Nerd Fonts e caracteres especiais sem depender de fontes instaladas no sistema operacional.
- **Temas**: Alternância instantânea entre modo Escuro (*Dark*) e Claro (*Light*).

### 🔍 Detecção Inteligente de Rede & Filtro de Adaptadores Virtuais
- Consulta nativa de interfaces via `GetAdaptersAddresses` e `GetIfEntry2`.
- **Discriminação de Adaptadores**: Avalia flags de hardware NDIS (`HardwareInterface == 1`, `ConnectorPresent == 1`) e padrões conhecidos de fornecedores para ignorar interfaces virtuais (VMware, VirtualBox, Hyper-V, Tailscale, Radmin VPN, Fortinet, WireGuard, Docker, WSL, etc.).
- **Identificação do Gateway Padrão**: Consulta à tabela de rotas via `GetIpForwardTable2` para identificar o gateway ativo com menor métrica combinada (rota + interface).

### ⚡ Motor ICMP & Estatísticas RFC 3550
- Envio de requisições de eco ICMP assíncronas via `IcmpCreateFile` e `IcmpSendEcho` sem requerer privilégios de administrador.
- Monitoramento simultâneo do Gateway Padrão dinâmico, servidores DNS globais (Google `8.8.8.8`, Cloudflare `1.1.1.1`) e hosts personalizados.
- **Cálculo de Jitter RFC 3550**: Diferença ponderada entre amostras consecutivas de latência:
  $$\Delta = |RTT_i - RTT_{i-1}|, \quad J_i = J_{i-1} + \frac{|\Delta| - J_{i-1}}{16}$$
- Cálculo de RTT atual, mínimo, médio, máximo e taxa de perda de pacotes (%).
- Classificação visual instantânea com limites configuráveis: **BOM** (Verde), **MÉDIO** (Amarelo), **ALTO** (Laranja) e **OFFLINE** (Vermelho).

### 📈 Gráficos em Tempo Real & Histórico Persistente
- **Gráficos Interativos**: Séries temporais em tempo real com `egui_plot`, suportando zoom pelo scroll do mouse, navegação por arrasto (*pan*) e seleção de séries por host.
- **Persistência SQLite de Alto Desempenho**:
  - Modo WAL (`PRAGMA journal_mode = WAL`) com leituras e gravações concorrentes sem bloqueio.
  - Gravações assíncronas em lote (*batch*) gerenciadas por uma thread dedicada de banco de dados via canal `mpsc`.
  - Expurgo automático com política de retenção configurável (ex: 30 dias).
- **Filtros Históricos**: Consulta por períodos de 1 hora, 6 horas, 24 horas, 7 dias, 30 dias ou histórico completo.
- **Registro de Eventos**: Log estruturado de quedas, reconexões e degradações da conexão.

### 🔄 Manutenção & Zerar Métricas
- Botão dedicado na aba **Configurações** para reinicializar todos os acumuladores de latência, jitter e perdas em tempo real.
- Caixa de confirmação de segurança com opção de excluir também as amostras antigas do banco SQLite (`rustnet.db`).

### 🚀 Atualização Automática Integrada (Hot Reload)
- O aplicativo verifica em background novas versões publicadas no **GitHub Releases** (`lpl2103/rustnet-monitor`).
- Download em streaming com validação do cabeçalho de executável PE (`MZ`).
- Substituição a quente em tempo de execução (`RustNetMonitor.exe.new` -> `RustNetMonitor.exe`) com reinicialização automática e limpeza do backup anterior (`--cleanup-old`).
- Interface gráfica com barra de progresso em tempo real e notas da versão.

---

## 🗂️ Estrutura do Projeto

```text
rustnet-monitor/
├── assets/                  # Ícones da aplicação (app.ico multi-resolução e app.png 256x256)
├── src/
│   ├── main.rs              # Ponto de entrada, subsistema PE e orquestrador híbrido
│   ├── config/              # Leitura, validação e persistência do config.toml
│   │   ├── mod.rs
│   │   └── settings.rs
│   ├── database/            # Camada de persistência local SQLite
│   │   ├── mod.rs
│   │   ├── connection.rs    # Conexão WAL, synchronous=NORMAL, busy_timeout
│   │   ├── migrations.rs    # Esquema relacional (hosts, latency_samples, network_events)
│   │   ├── models.rs        # Estruturas de dados do banco
│   │   ├── repository.rs    # Queries, batch insert e exclusão de amostras
│   │   └── worker.rs        # Thread assíncrona não-bloqueante
│   ├── network/             # Camada nativa Windows de rede e ICMP
│   │   ├── mod.rs
│   │   ├── types.rs         # Estruturas de Adaptadores, Rotas e Diagnósticos
│   │   ├── adapters.rs      # Win32 GetAdaptersAddresses e GetIfEntry2
│   │   ├── routes.rs        # Win32 GetIpForwardTable2
│   │   ├── detector.rs      # Correlação e identificação da conexão ativa
│   │   ├── virtual_filter.rs# Filtragem multinível de interfaces virtuais
│   │   ├── icmp.rs          # Motor Win32 IcmpSendEcho nativo
│   │   ├── stats.rs         # Acumulador estatístico, Jitter RFC 3550 e qualidade
│   │   └── pinger.rs        # Pinger multithread periódico com canal de controle
│   ├── gui/                 # Interface Gráfica nativa (eframe / egui)
│   │   ├── mod.rs           # Setup de tipografia (Segoe UI) e estilos
│   │   ├── app.rs           # Loop principal da janela, abas e eventos
│   │   ├── dashboard.rs     # Aba Dashboard (status da interface e tabela em tempo real)
│   │   ├── charts.rs        # Aba Gráficos (curvas temporais egui_plot)
│   │   ├── history.rs       # Aba Histórico (filtros de período e paginação)
│   │   ├── events.rs        # Aba Eventos de rede
│   │   └── settings.rs      # Aba Configurações e diálogo de zerar métricas
│   └── utils/
│       ├── mod.rs
│       └── logging.rs       # Logging duplo (console + logs/rustnet.log)
├── build.rs                 # Script de compilação de recursos PE do Windows (winres)
├── build.ps1                # Script PowerShell de build release automatizado
├── Cargo.toml               # Dependências e perfil de compilação otimizado
├── CHANGELOG.md             # Histórico de versões (Keep a Changelog)
└── LICENSE                  # Licença MIT
```

---

## 🚀 Como Compilar e Executar

### Pré-requisitos
- **Windows 10** ou **Windows 11** (64-bit)
- **Rust 1.80+** (toolchain `stable-x86_64-pc-windows-msvc`)

### Compilação Automatizada (Recomendado)
Execute o script PowerShell fornecido na raiz. Ele compila o projeto em perfil *release* otimizado com LTO (*Link-Time Optimization*), símbolos removidos (*strip*) e copia o executável final diretamente para a raiz:

```powershell
.\build.ps1
```

O binário gerado estará disponível imediatamente em:
```text
.\RustNetMonitor.exe
```

### Compilação Manual via Cargo
```powershell
cargo build --release
```
O executável compilado será gerado em `target/release/rustnet-monitor.exe`.

---

## 💻 Modos de Uso

### 1. Interface Gráfica (Padrão)
Para abrir a interface gráfica, basta dar um **duplo clique** em `RustNetMonitor.exe` no Windows Explorer ou executá-lo no terminal sem parâmetros:
```powershell
.\RustNetMonitor.exe
```

### 2. Painel Interativo no Terminal (CLI Mode)
Para monitorar em servidores sem interface gráfica ou diretamente no terminal:
```powershell
.\RustNetMonitor.exe --cli
```

### 3. Diagnóstico Único de Rede
Exibe no console os adaptadores físicos detectados, interfaces virtuais ignoradas e tabela de rotas:
```powershell
.\RustNetMonitor.exe --diagnostic
```

### 4. Ciclo Único de Teste (Scripting / CI)
Executa um único ciclo de ping sobre todos os alvos, salva os resultados no SQLite e encerra:
```powershell
.\RustNetMonitor.exe --once
```

---

## ⚙️ Arquivo de Configuração (`config.toml`)

Caso não exista, um arquivo `config.toml` portátil será criado automaticamente na inicialização com valores padrão seguros:

```toml
[general]
theme = "dark"              # "dark", "light" ou "system"
language = "pt-BR"
log_level = "info"
minimize_to_tray = false
start_with_windows = false

[monitoring]
interval_secs = 2           # Intervalo entre rodadas de ping (segundos)
timeout_ms = 1000           # Tempo limite de resposta por ping (milissegundos)
retry_count = 1

[thresholds]
green_max_ms = 40           # Latências abaixo deste valor são "BOM"
yellow_max_ms = 120         # Entre green e yellow são "MÉDIO"; acima são "ALTO"

[database]
path = "rustnet.db"         # Caminho do banco de dados SQLite local
retention_days = 30         # Dias de retenção de histórico (0 para ilimitado)

[[hosts]]
name = "Gateway Padrão"
address = "auto"            # "auto" detecta dinamicamente o gateway físico ativo
enabled = true

[[hosts]]
name = "Google DNS"
address = "8.8.8.8"
enabled = true

[[hosts]]
name = "Cloudflare DNS"
address = "1.1.1.1"
enabled = true
```

---

## 📄 Licença

Este projeto é distribuído sob os termos da licença **MIT**.

A licença MIT é uma licença de software livre permissiva que permite a qualquer pessoa utilizar, copiar, modificar, mesclar, publicar, distribuir, sublicenciar e/ou vender cópias do software, sujeito apenas à inclusão do aviso de direitos autorais original.

Consulte o arquivo [`LICENSE`](LICENSE) para obter o texto integral da licença.

---

<p align="center">
  Desenvolvido por <strong>Leandro Pinheiro</strong> (2026)
</p>
