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
