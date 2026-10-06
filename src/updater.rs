//! # Módulo de Atualização Automática (`updater.rs`)
//!
//! Gerencia a verificação de versões, download e substituição a quente
//! do executável no Windows a partir das Releases do GitHub (`lpl2103/rustnet-monitor`).
//!
//! ## Técnica de Substituição Quente no Windows:
//! No Windows, um executável em execução não pode ser sobrescrito diretamente.
//! 1. O download é gravado em arquivo temporário `RustNetMonitor.exe.new`.
//! 2. Renomeamos o executável atual para `RustNetMonitor.exe.old`.
//! 3. Movemos `RustNetMonitor.exe.new` para `RustNetMonitor.exe`.
//! 4. Disparamos o novo executável com a flag `--cleanup-old`.
//! 5. O novo processo remove `RustNetMonitor.exe.old` do disco.

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::process::Command;
use tracing::{info, warn};

pub const GITHUB_REPO: &str = "lpl2103/rustnet-monitor";
pub const GITHUB_API_LATEST: &str =
    "https://api.github.com/repos/lpl2103/rustnet-monitor/releases/latest";
pub const GITHUB_DIRECT_DOWNLOAD: &str =
    "https://github.com/lpl2103/rustnet-monitor/releases/latest/download/RustNetMonitor.exe";

/// Informações sobre uma versão remota disponível
#[derive(Debug, Clone)]
pub struct RemoteVersionInfo {
    pub version: String,
    pub download_url: String,
    pub release_notes: String,
}

/// Status do processo de atualização
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    /// Ocioso / Pronto para checar
    Idle,
    /// Verificando se há atualizações
    Checking,
    /// Nenhuma atualização encontrada
    UpToDate,
    /// Baixando o novo executável (Progresso de 0.0 a 1.0)
    Downloading(f32),
    /// Atualização concluída com sucesso (Aguardando reinicialização)
    Success(String),
    /// Falha no processo de atualização (Mensagem de erro)
    Error(String),
}

/// Compara duas versões em formato SemVer ("0.1.1" vs "0.1.0").
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let clean = v.trim().trim_start_matches(['v', 'V']);
    let mut parts = clean.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer_version(remote: &str, current: &str) -> bool {
    if let (Some(r), Some(c)) = (parse_version(remote), parse_version(current)) {
        r > c
    } else {
        remote.trim() != current.trim()
    }
}

/// Remove arquivos temporários de versões anteriores (`.exe.old`).
pub fn clean_old_update_files() {
    if let Ok(current_exe) = std::env::current_exe() {
        let old_exe = current_exe.with_extension("exe.old");
        if old_exe.exists() {
            for _ in 0..15 {
                if fs::remove_file(&old_exe).is_ok() {
                    info!(
                        "Arquivo de backup anterior removido com sucesso: {:?}",
                        old_exe
                    );
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    }
}

/// Verifica se existe uma nova versão disponível no GitHub Releases.
pub fn check_for_updates() -> Option<RemoteVersionInfo> {
    let current_version = env!("CARGO_PKG_VERSION");
    info!(
        "Verificando atualizações no GitHub (versão instalada: v{})...",
        current_version
    );

    let gh_resp = ureq::get(GITHUB_API_LATEST)
        .set("User-Agent", &format!("RustNetMonitor/{}", current_version))
        .set("Accept", "application/vnd.github.v3+json")
        .timeout(std::time::Duration::from_secs(8))
        .call();

    match gh_resp {
        Ok(resp) if resp.status() == 200 => {
            if let Ok(json_str) = resp.into_string()
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str)
            {
                let tag_name = json.get("tag_name").and_then(|v| v.as_str()).unwrap_or("");
                let clean_tag = tag_name.trim().trim_start_matches(['v', 'V']);
                let body = json
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Melhorias e correções de estabilidade.");

                // Encontra a URL do executável nos assets
                let mut dl_url = GITHUB_DIRECT_DOWNLOAD.to_string();
                if let Some(assets) = json.get("assets").and_then(|a| a.as_array()) {
                    for asset in assets {
                        let is_match =
                            asset
                                .get("name")
                                .and_then(|n| n.as_str())
                                .is_some_and(|name| {
                                    name.eq_ignore_ascii_case("RustNetMonitor.exe")
                                        || name.eq_ignore_ascii_case("rustnet-monitor.exe")
                                });
                        if is_match
                            && let Some(url) =
                                asset.get("browser_download_url").and_then(|u| u.as_str())
                        {
                            dl_url = url.to_string();
                            break;
                        }
                    }
                }

                if is_newer_version(clean_tag, current_version) {
                    info!(
                        "Nova versão encontrada no GitHub: v{} (atual: v{})",
                        clean_tag, current_version
                    );
                    return Some(RemoteVersionInfo {
                        version: clean_tag.to_string(),
                        download_url: dl_url,
                        release_notes: body.to_string(),
                    });
                } else {
                    info!(
                        "RustNet Monitor já está na versão mais recente (v{}).",
                        current_version
                    );
                }
            }
        }
        Ok(resp) => {
            warn!(
                "Resposta inesperada do GitHub Releases: status {}",
                resp.status()
            );
        }
        Err(e) => {
            warn!("Não foi possível consultar o GitHub Releases: {}", e);
        }
    }

    None
}

/// Executa o download da nova versão e realiza a substituição do executável no Windows.
pub fn perform_auto_update<F>(
    custom_url: Option<String>,
    progress_callback: F,
) -> Result<(), String>
where
    F: Fn(UpdateStatus) + Send + 'static,
{
    progress_callback(UpdateStatus::Downloading(0.0));

    let download_url = custom_url.unwrap_or_else(|| GITHUB_DIRECT_DOWNLOAD.to_string());
    info!(
        "Iniciando download da atualização a partir de '{}'...",
        download_url
    );

    let agent = ureq::builder()
        .redirects(5)
        .timeout(std::time::Duration::from_secs(90))
        .build();

    let response = agent
        .get(&download_url)
        .set("User-Agent", "RustNetMonitor/Updater")
        .call()
        .map_err(|e| format!("Falha ao conectar no endereço de atualização: {}", e))?;

    let content_length = response
        .header("Content-Length")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(0);

    info!("Tamanho reportado da atualização: {} bytes", content_length);

    // 2. Determina caminhos dos executáveis
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Falha ao determinar caminho do executável atual: {}", e))?;
    let old_exe = current_exe.with_extension("exe.old");
    let new_exe = current_exe.with_extension("exe.new");

    // 3. Gravação em streaming direto para arquivo temporário no disco (zero alocação de memória excessiva)
    {
        let file = File::create(&new_exe)
            .map_err(|e| format!("Falha ao criar arquivo temporário {:?}: {}", new_exe, e))?;
        let mut writer = BufWriter::new(file);
        let mut reader = response.into_reader();
        let mut buffer = [0u8; 16384];
        let mut total_read = 0;
        let mut header_check = [0u8; 2];
        let mut checked_header = false;

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if !checked_header && n >= 2 {
                        header_check[0] = buffer[0];
                        header_check[1] = buffer[1];
                        checked_header = true;
                    }

                    writer
                        .write_all(&buffer[..n])
                        .map_err(|e| format!("Erro ao gravar dados do download no disco: {}", e))?;
                    total_read += n;

                    if content_length > 0 {
                        let progress = (total_read as f32) / (content_length as f32);
                        progress_callback(UpdateStatus::Downloading(progress.min(1.0)));
                    }
                }
                Err(e) => return Err(format!("Erro durante o download do fluxo de dados: {}", e)),
            }
        }

        writer
            .flush()
            .map_err(|e| format!("Falha ao descarregar buffers do executável baixado: {}", e))?;

        // Validação de sanidade do executável PE Windows (deve começar com "MZ" - 0x4D, 0x5A)
        if total_read < 1024 || header_check != [0x4D, 0x5A] {
            let _ = fs::remove_file(&new_exe);
            return Err(
                "Arquivo baixado não é um executável Windows válido (cabeçalho MZ ausente)."
                    .to_string(),
            );
        }

        info!("Download concluído com sucesso: {} bytes.", total_read);
    }

    // 4. Substituição do executável no Windows
    if old_exe.exists() {
        let _ = fs::remove_file(&old_exe);
    }

    // Renomeia atual -> old
    fs::rename(&current_exe, &old_exe)
        .map_err(|e| format!("Falha ao mover executável atual para backup: {}", e))?;

    // Renomeia new -> atual
    if let Err(e) = fs::rename(&new_exe, &current_exe) {
        // Tenta reverter o backup em caso de erro
        let _ = fs::rename(&old_exe, &current_exe);
        return Err(format!("Falha ao posicionar novo executável: {}", e));
    }

    info!("Executável substituído com sucesso por {:?}", current_exe);

    // 5. Reinicia o aplicativo atualizado em novo processo com instrução de limpeza
    Command::new(&current_exe)
        .arg("--cleanup-old")
        .spawn()
        .map_err(|e| format!("Falha ao iniciar processo atualizado: {}", e))?;

    progress_callback(UpdateStatus::Success(
        "Atualização concluída com sucesso! Reiniciando...".to_string(),
    ));

    // Pausa breve para garantir despacho antes de fechar o processo atual
    std::thread::sleep(std::time::Duration::from_millis(300));
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(parse_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("v0.1.1"), Some((0, 1, 1)));
        assert_eq!(parse_version("V1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("2.0"), Some((2, 0, 0)));
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.1.1", "0.1.0"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("v0.2.0", "0.1.9"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "0.1.1"));
        assert!(!is_newer_version("0.0.9", "0.1.0"));
    }
}
