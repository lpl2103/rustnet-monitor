# RustNet Monitor

Monitor de conectividade de rede nativo para Windows em Rust, projetado para baixíssimo consumo de recursos (CPU/RAM), executável portátil e monitoramento de conectividade em tempo real sem dependências externas.

## Funcionalidades (Fases 1 e 2)

- **Detecção Nativa de Rede no Windows**: Consulta direta às APIs Win32 / IP Helper (`iphlpapi.dll`) sem execução de subprocessos externos (`ipconfig`, `route`, `powershell`, etc.).
- **Identificação da Interface Física Ativa**: Algoritmo robusto baseado em flags de hardware NDIS (`HardwareInterface`, `ConnectorPresent`) e correlação com a rota padrão de menor métrica.
- **Filtragem Inteligente de Adaptadores Virtuais**: Desconsidera e classifica automaticamente interfaces virtuais (VMware, VirtualBox, Hyper-V, Tailscale, Radmin VPN, Fortinet, WireGuard, Docker, WSL, etc.).
- **Detecção Automática do Gateway Padrão**: Identifica o gateway padrão da conexão ativa e exibe métricas detalhadas de roteamento.
- **Portátil e Leve**: Executável único estático sem dependência de runtimes externos (.NET, C++ Redistributable, etc.).
- **Configuração via TOML**: Suporte a `config.toml` portátil criado automaticamente com valores padrão seguros.
- **Logging Estruturado**: Saída em console e gravação contínua sem bloqueio em `logs/rustnet.log`.

## Estrutura do Projeto

```text
src/
├── main.rs                 # Ponto de entrada e CLI de diagnóstico
├── config/                 # Gerenciamento de configurações (config.toml, serde)
│   ├── mod.rs
│   └── settings.rs
├── network/                # Camada nativa Windows de rede (IP Helper / NetIO)
│   ├── mod.rs
│   ├── types.rs            # Structs de Interface, Gateway, Route, AdapterType
│   ├── adapters.rs         # GetAdaptersAddresses (IPs, MAC, Status, DNS, Gateways)
│   ├── routes.rs           # GetIpForwardTable2 (tabela de rotas e default route)
│   ├── detector.rs         # Orquestrador de detecção e correlação da interface ativa
│   └── virtual_filter.rs   # Classificação e filtro de interfaces virtuais vs físicas
└── utils/                  # Utilitários gerais
    ├── mod.rs
    └── logging.rs          # Inicialização de tracing (console + arquivo logs/rustnet.log)
```

## Como Compilar

### Pré-requisitos
- Rust 1.80+ (com suporte à edição 2024 / MSVC no Windows x64)

### Compilação de Desenvolvimento
```powershell
cargo build
```

### Compilação Otimizada (Release)
```powershell
cargo build --release
```

O binário final é gerado em `target/release/rustnet-monitor.exe` com LTO ativado e símbolos removidos (*stripped*).

## Execução

```powershell
.\RustNetMonitor.exe
```

## Licença

Distribuído sob a licença MIT. Veja `LICENSE` para mais informações.
