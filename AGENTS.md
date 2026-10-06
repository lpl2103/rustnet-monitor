# Regras do Projeto (AGENTS.md)

Este repositório segue estritamente as diretrizes definidas em [GEMINI.md](file:///d:/src/monitoramento/GEMINI.md):

1. **Versionamento Obrigatório (SemVer):** Toda alteração de código, melhoria, correção de bug ou build de produção **DEVE** obrigatoriamente incrementar a versão no `Cargo.toml` (ex: `0.1.0` -> `0.1.1`).
2. **Compilação Otimizada de Produção:**
   - Usar `cargo build --release` (otimizado com `opt-level = "z"`, `lto = true`, `panic = "abort"`, `codegen-units = 1`, `strip = true`).
3. **Cópia de Binários na Raiz:**
   - O binário compilado `target\release\rustnet-monitor.exe` deve ser copiado diretamente para a raiz do repositório como `RustNetMonitor.exe`.
4. **Publicação Automática Completa:**
   - Commit + Push na branch `main` no repositório `lpl2103/rustnet-monitor`.
   - Criação e push da git tag `v<VERSÃO>`.
   - Publicação automática da Release no GitHub anexando o binário `RustNetMonitor.exe` via `gh release create`.
5. **Atualização Automática (Hot Reload / Auto-Updater):**
   - O aplicativo possui auto-atualizador integrado (`src/updater.rs`) que consome as releases de `lpl2103/rustnet-monitor` e substitui o executável a quente no Windows com `--cleanup-old`.
