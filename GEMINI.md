# Regras de Desenvolvimento e Diretrizes do Projeto (`GEMINI.md`)

Este arquivo define os padrões obrigatórios e automáticos que o assistente de IA (Antigravity) e desenvolvedores devem seguir **sempre que fizerem qualquer alteração ou build neste repositório**.

---

## 📌 1. Regra de Versionamento Obrigatório (SemVer)
- **Toda alteração de código, melhoria, correção de bug ou build DEVE obrigatoriamente incrementar o número da versão** no arquivo `Cargo.toml`.
- Exemplo: de `0.1.0` para `0.1.1`.
- Nunca gerar uma nova compilação sem incrementar a versão, pois o auto-atualizador (`src/updater.rs`) depende de versões estritamente crescentes para que o aplicativo detecte novas versões no GitHub Releases.

---

## ⚡ 2. Otimização de Binário e Recursos Embutidos
Para manter o executável leve, enxuto e autônomo (sem dependências externas):
1. **Victor Mono Nerd Font Embutida:**
   - A fonte é armazenada pré-comprimida em `assets/victor_mono.deflate` (zlib) e descompactada em memória pelo `miniz_oxide`.
   - A flag `default_fonts` do `eframe` deve permanecer desativada em `Cargo.toml`, economizando espaço significativo no executável.
2. **Perfil de Release em `Cargo.toml`:**
   ```toml
   [profile.release]
   opt-level = "z"     # Otimização máxima voltada para menor tamanho de binário
   lto = true          # Link-Time Optimization entre todas as crates
   codegen-units = 1   # Gera código global unificado para melhor redução de código morto
   panic = "abort"     # Remove tabelas pesadas de unwinding de exceções
   strip = true        # Remove símbolos de depuração do binário final
   ```

---

## 📂 3. Cópia Obrigatória do Binário para a Raiz
Após cada compilação de produção, o executável final deve ser copiado diretamente para a raiz do repositório:
```powershell
Copy-Item -Path "target\release\rustnet-monitor.exe" -Destination "RustNetMonitor.exe" -Force
```

---

## 🚀 4. Regra de Publicação no GitHub e Criação de Release
Sempre que concluir um ciclo de melhorias ou novo build, realize a publicação completa:

### Script Automatizado:
Execute o script oficial de build e release:
```powershell
.\build_and_release.ps1 -Notes "Resumo das melhorias implementadas"
```

### Etapas executadas automaticamente pelo script:
1. Validação de compilação em modo release otimizado.
2. Cópia do binário para `RustNetMonitor.exe`.
3. Adição e commit no Git:
   ```powershell
   git add Cargo.toml Cargo.lock RustNetMonitor.exe src/ assets/ config.toml README.md AGENTS.md GEMINI.md build.ps1 build_and_release.ps1
   git commit -m "release: v<VERSÃO> - <Notas>"
   git push origin main
   ```
4. Criação e push da Tag:
   ```powershell
   git tag -a "v<VERSÃO>" -m "Release v<VERSÃO>"
   git push origin "v<VERSÃO>"
   ```
5. Publicação no GitHub Releases com o executável anexado:
   ```powershell
   gh release create "v<VERSÃO>" "RustNetMonitor.exe" --repo "lpl2103/rustnet-monitor" --title "RustNet Monitor v<VERSÃO>" --notes "<Notas>"
   ```

---

## 🔄 5. Mecanismo de Auto-Atualização (`src/updater.rs`)
- O aplicativo verifica a existência de novas releases via API do GitHub (`lpl2103/rustnet-monitor/releases/latest`).
- Download em streaming com verificação do cabeçalho PE Windows (`MZ` / `0x4D, 0x5A`).
- Troca a quente (`RustNetMonitor.exe.new` -> `RustNetMonitor.exe`, backup temporário em `RustNetMonitor.exe.old`).
- Reinício do processo com o parâmetro `--cleanup-old` para limpeza do backup antigo.

---

## 🦀 6. Padrões de Código e Estabilidade
1. **Clippy e Compilador:** Manter o projeto com 0 erros e 0 warnings no `cargo check` e `cargo clippy`.
2. **Robustez Concorrente:** Manter workers assíncronos desacoplados com canais mpsc e encerramento limpo via RAII.
