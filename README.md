# RustNet Monitor

Monitor de conectividade de rede nativo para Windows em Rust, projetado para baixíssimo consumo de recursos (CPU/RAM), executável portátil e monitoramento de conectividade em tempo real sem dependências externas.

## Funcionalidades Principais

- **Interface Gráfica Moderna e Nativa (GUI)**: Janela desktop fluida com `eframe` (egui) acelerada por GPU (OpenGL Glow), tema escuro/claro e carregamento automático da tipografia Segoe UI nativa do Windows com suporte a acentuação em português.
- **Detecção Nativa de Rede no Windows**: Consulta direta às APIs Win32 / IP Helper (`iphlpapi.dll`) sem subprocessos externos (`ipconfig`, `route`, etc.).
- **Filtragem Inteligente de Adaptadores Virtuais**: Desconsidera e classifica automaticamente interfaces virtuais (VMware, VirtualBox, Hyper-V, Tailscale, Radmin VPN, Fortinet, WireGuard, Docker, WSL, etc.) usando flags de hardware NDIS (`HardwareInterface`, `ConnectorPresent`).
- **Detecção Automática do Gateway Padrão**: Identifica o gateway padrão ativo via `GetIpForwardTable2` com cálculo combinado de métricas de rota e interface.
- **Motor de Ping ICMP Nativo**: Pings assíncronos e paralelos usando a API Win32 ICMP (`IcmpCreateFile`, `IcmpSendEcho`, `IcmpCloseHandle`), sem requerer elevação de privilégios de administrador.
- **Cálculo de Jitter RFC 3550 & Métricas**: Jitter estático ponderado ($D = |RTT_i - RTT_{i-1}|$, $J_i = J_{i-1} + \frac{|D| - J_{i-1}}{16}$), latência mínima/máxima/média, perda de pacotes (%) e classificação visual de qualidade.
- **Gráficos em Tempo Real**: Curvas de latência interativas por host com `egui_plot` (zoom, pan, alternância de séries).
- **Persistência Local de Alto Desempenho**: Banco de dados SQLite embutido compilado no próprio executável (`rusqlite` bundled, zero DLLs), com modo WAL (`Write-Ahead Logging`), transações assíncronas em lote via worker thread desacoplado e expurgo automático por retenção.
- **Histórico & Eventos de Conectividade**: Histórico de latência com filtros (1h, 6h, 24h, 7d, 30d) e auditoria visual de eventos de rede (quedas, restabelecimentos, degradação).
- **100% Portátil e Leve**: Executável único estático (~7 MB) na raiz do projeto (`RustNetMonitor.exe`), sem instalador e sem dependências de runtime (.NET ou C++ redistributable).
- **Modos de Execução**: Abre a GUI interativa por padrão; suporta execução CLI para servidores ou diagnóstico via `--cli`, `--diagnostic` ou `--once`.

## Estrutura do Projeto

```text
src/
├── main.rs                 # Ponto de entrada (GUI por padrão, suporte a flags CLI)
├── config/                 # Gerenciamento de configurações (config.toml, serde)
│   ├── mod.rs
│   └── settings.rs
├── database/               # Persistência SQLite local
│   ├── mod.rs
│   ├── connection.rs       # Conexão WAL, synchronous=NORMAL, busy_timeout
│   ├── migrations.rs       # Tabelas hosts, latency_samples, network_events
│   ├── repository.rs       # Operações relacionais (upsert, batch insert, queries)
│   └── worker.rs           # Thread assíncrona desacoplada via mpsc::channel
├── network/                # Camada nativa Windows de rede (IP Helper / NetIO / ICMP)
│   ├── mod.rs
│   ├── types.rs            # Structs de Interface, Gateway, Route, AdapterType
│   ├── adapters.rs         # GetAdaptersAddresses (IPs, MAC, Status, DNS, Gateways)
│   ├── routes.rs           # GetIpForwardTable2 (tabela de rotas e default route)
│   ├── detector.rs         # Orquestrador de detecção e correlação da interface ativa
│   ├── virtual_filter.rs   # Classificação e filtro de interfaces virtuais vs físicas
│   ├── icmp.rs             # Win32 IcmpSendEcho nativo
│   ├── stats.rs            # Jitter RFC 3550, packet loss, médias e limites
│   └── pinger.rs           # Motor de ping periódico multithread
├── gui/                    # Interface Gráfica eframe / egui
│   ├── mod.rs              # Setup de fontes nativas (Segoe UI)
│   ├── app.rs              # Loop principal eframe::App e abas de navegação
│   ├── dashboard.rs        # Interface ativa e tabela de hosts em tempo real
│   ├── charts.rs           # Gráficos de linhas em tempo real com egui_plot
│   ├── history.rs          # Consulta e análise estatística do histórico SQLite
│   ├── events.rs           # Registro de eventos de conexão/desconexão
│   └── settings.rs         # Configuração de temas, intervalos e thresholds
└── utils/                  # Utilitários gerais
    ├── mod.rs
    └── logging.rs          # Logging estruturado (console + logs/rustnet.log)
```

## Como Compilar

Execute o script de build automatizado do PowerShell que compila em release otimizado e copia o executável diretamente para a raiz:

```powershell
.\build.ps1
```

Ou manualmente via Cargo:

```powershell
cargo build --release
```

## Modos de Uso

### 1. Interface Gráfica (Padrão)
Basta dar duplo clique em `RustNetMonitor.exe` ou executar:
```powershell
.\RustNetMonitor.exe
```

### 2. Painel CLI no Terminal
```powershell
.\RustNetMonitor.exe --cli
```

### 3. Diagnóstico de Interfaces e Rotas
```powershell
.\RustNetMonitor.exe --diagnostic
```

### 4. Ciclo Único de Teste (Script / CI)
```powershell
.\RustNetMonitor.exe --once
```

## Licença

Distribuído sob a licença MIT. Veja `LICENSE` para mais informações.
