//! Descoberta das instalações efetivas do Steam (nativo, Flatpak, Snap) e
//! das ferramentas de compatibilidade que vieram delas.
//!
//! Porta de `src-tauri/src/system/steam.rs` do repositório Gamefic,
//! incluindo só o necessário à detecção do catálogo (Protons oficiais e
//! customizados). Sem serde: os tipos não cruzam fronteira de processo.
//!
//! Nenhum caminho é presumido fixo: a raiz do Steam é detectada, as
//! bibliotecas vêm de `steamapps/libraryfolders.vdf` e cada instalação é
//! validada (contém o executável `proton` da própria ferramenta) antes de
//! qualquer operação.

use std::fs;
use std::path::{Path, PathBuf};

/// ferramenta de compatibilidade local detectada em bibliotecas Steam
#[derive(Debug, Clone)]
pub struct ProtonLocal {
    pub nome: String,
    pub versao: Option<String>,
    pub caminho: String,
    pub biblioteca: String,
    /// "oficial" (gerenciado pelo cliente Steam) | "custom" (compatibilitytools.d)
    pub tipo: String,
}

/// raízes de dados do Steam em instalações nativas, Flatpak e Snap
pub fn raizes_steam() -> Vec<PathBuf> {
    let mut raizes = Vec::new();
    let home = std::env::var("HOME").ok();
    if let Some(h) = &home {
        let h = PathBuf::from(h);
        raizes.push(h.join(".local/share/Steam"));
        raizes.push(h.join(".steam/steam"));
        raizes.push(h.join(".var/app/com.valvesoftware.Steam/data/Steam"));
        raizes.push(h.join("snap/steam/common/.local/share/Steam"));
    }
    raizes
        .into_iter()
        .filter(|r| r.is_dir())
        .collect()
}

/// bibliotecas de uma raiz: a própria + as listadas em libraryfolders.vdf
pub fn bibliotecas(raiz: &Path) -> Vec<PathBuf> {
    let mut libs = vec![raiz.to_path_buf()];
    let vdf = raiz.join("steamapps/libraryfolders.vdf");
    if let Ok(texto) = fs::read_to_string(&vdf) {
        for linha in texto.lines() {
            let l = linha.trim();
            // linha do formato:        "path"        "/mnt/dados/Steam"
            let Some(resto) = l.strip_prefix("\"path\"") else { continue };
            let caminho = resto.trim().trim_matches('"').trim();
            if caminho.is_empty() {
                continue;
            }
            let p = PathBuf::from(caminho);
            if p.is_dir() && !libs.contains(&p) {
                libs.push(p);
            }
        }
    }
    libs
}

/// versão do diretório (arquivo `version` que acompanha builds do Proton),
/// ex.: "1738410425 proton-9.0-3" → "proton-9.0-3"
fn versao_de_diretorio(dir: &Path) -> Option<String> {
    let texto = fs::read_to_string(dir.join("version")).ok()?;
    let parte = texto
        .split_whitespace()
        .nth(1)
        .unwrap_or(texto.trim())
        .trim();
    (!parte.is_empty()).then(|| parte.to_string())
}

fn dir_e_proton(dir: &Path) -> bool {
    dir.is_dir() && dir.join("proton").exists()
}

/// Protons oficiais (gerenciados pelo cliente Steam): em cada biblioteca,
/// pasta em `steamapps/common` cujo nome começa com "Proton"
pub fn protons_oficiais() -> Vec<ProtonLocal> {
    let mut out = Vec::new();
    for raiz in raizes_steam() {
        for lib in bibliotecas(&raiz) {
            let common = lib.join("steamapps/common");
            let Ok(entries) = fs::read_dir(&common) else { continue };
            for e in entries.flatten() {
                let dir = e.path();
                let nome = e.file_name().to_string_lossy().to_string();
                if !nome.starts_with("Proton") || !dir_e_proton(&dir) {
                    continue;
                }
                out.push(ProtonLocal {
                    versao: versao_de_diretorio(&dir),
                    nome,
                    caminho: dir.to_string_lossy().to_string(),
                    biblioteca: lib.to_string_lossy().to_string(),
                    tipo: "oficial".to_string(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.nome.cmp(&b.nome));
    out.dedup_by(|a, b| a.caminho == b.caminho);
    out
}

/// Protons/ferramentas customizadas em `compatibilitytools.d` de cada raiz
pub fn protons_custom() -> Vec<ProtonLocal> {
    let mut out = Vec::new();
    for raiz in raizes_steam() {
        let raiz_root = raiz.join("compatibilitytools.d");
        let Ok(entries) = fs::read_dir(&raiz_root) else { continue };
        for e in entries.flatten() {
            let dir = e.path();
            let nome = e.file_name().to_string_lossy().to_string();
            if !dir_e_proton(&dir) {
                continue;
            }
            out.push(ProtonLocal {
                versao: versao_de_diretorio(&dir),
                nome,
                caminho: dir.to_string_lossy().to_string(),
                biblioteca: raiz_root.to_string_lossy().to_string(),
                tipo: "custom".to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.nome.cmp(&b.nome));
    out.dedup_by(|a, b| a.caminho == b.caminho);
    out
}
