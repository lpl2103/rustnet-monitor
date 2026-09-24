# Changelog

Todas as mudanças notáveis deste projeto serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [0.1.0] - 2026-09-23

### Adicionado
- Estrutura base do projeto Rust e repositório Git.
- Perfil de compilação release otimizado para binário reduzido e alto desempenho (`lto = true`, `opt-level = "z"`, `strip = true`).
- Módulo de configuração portátil via `config.toml` (com suporte a fallback e geração automática).
- Sistema de telemetria e logging estruturado com gravação simultânea no console e em `logs/rustnet.log`.
- Camada nativa Windows para consulta a adaptadores de rede (`GetAdaptersAddresses`).
- Detecção nativa de propriedades NDIS e hardware flags (`GetIfEntry2` com `HardwareInterface` e `ConnectorPresent`).
- Consulta à tabela de rotas do Windows (`GetIpForwardTable2`) para identificação da rota padrão ativa e gateway.
- Algoritmo multinível de discriminação entre interfaces físicas e interfaces virtuais (VMware, VirtualBox, Hyper-V, Tailscale, Radmin VPN, etc.).
- Interface CLI de diagnóstico para inspeção em tempo real das interfaces e rotas.
- Motor de ping ICMP nativo via Windows API (`IcmpCreateFile`, `IcmpSendEcho`, `IcmpCloseHandle`) sem necessidade de privilégios de administrador.
- Monitoramento contínuo de múltiplos alvos: Gateway dinâmico, Google DNS (`8.8.8.8`), Cloudflare DNS (`1.1.1.1`) e hosts personalizados.
- Cálculo de métricas estatísticas de rede: RTT atual, mínimo, médio, máximo, perda de pacotes (%) e Jitter conforme RFC 3550.
- Classificação de qualidade visual (BOM / MÉDIO / ALTO / OFFLINE) baseada em limites configuráveis.
- Painel em tempo real no console com suporte a encerramento gracioso via Ctrl+C.
- Camada de persistência local em SQLite (`rusqlite` com SQLite embutido, zero DLLs externas).
- Criação idempotente de tabelas relacionais (`hosts`, `latency_samples`, `network_events`) com índices otimizados para séries temporais.
- Ativação do modo WAL (`PRAGMA journal_mode = WAL`) e `PRAGMA synchronous = NORMAL` para alto desempenho de disco.
- Database Worker assíncrono desacoplado via canal `mpsc::channel` com gravações agrupadas em transações atômicas (*batch writes*).
- Política de retenção automática configurável (padrão 30 dias) para limpeza de amostras antigas em background.
