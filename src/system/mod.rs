//! Camada de detecção do sistema — porto da camada `system/` do repositório
//! Gamefic (branch gamefic-slint), Rust puro: sem Tauri, sem serde, sem
//! dados sintéticos.
//!
//! Neste slice (ações da aba Apps):
//! - `appinfo`: REGISTRY completo (ids confiáveis por aplicativo);
//! - `pkgmgr`: gerenciador nativo da família detectada, consultas reais e
//!   instalar/remover via pkexec;
//! - `tools`: catálogo por categoria + estado de instalação (§2.2 —
//!   sem confirmação, o estado é "desconhecido");
//! - `install`: listagem real de fontes, instalar/remover/atualizar,
//!   abrir launcher e resolução de fonte padrão;
//! - `steam`: protons localizados do Steam (consumo futuro do install);
//! - `hardware`: processador, GPUs, Vulkan e kernel para a aba
//!   Kernels e Drivers (porto de `cpu.rs`/`gpu.rs`/`vulkan.rs`);
//! - `recommendations`: análise de oportunidades e perfis para a aba
//!   Otimização (porto de `recommendations.rs`);
//! - `os`/`spawn`: base de distribuição e execução consciente de sandbox.
//!
//! Fora do escopo por ora (slices seguintes): monitoramento, verificação
//! de versões remotas/has_update (rede) e ativação de MangoHud no abrir.

pub mod appinfo;
pub mod hardware;
pub mod install;
pub mod os;
pub mod pkgmgr;
pub mod recommendations;
pub mod spawn;
pub mod steam;
pub mod tools;

/// distribuição detectada pelo /etc/os-release (honesto: None quando
/// a leitura não é possível)
#[derive(Debug, Clone)]
pub struct DistroInfo {
    pub name: String,
    pub version: Option<String>,
    /// família ampla (Arch, Debian, Red Hat, SUSE, ...) — base de descoberta
    pub family: Option<String>,
}

/// card do catálogo: `estado` é sempre um valor real
/// ("instalado" | "nao_instalado" | "desconhecido")
#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub available: bool,
    pub version: Option<String>,
    pub estado: String,
    pub formato: Option<String>,
}

/// card de ferramenta: `available` é capacidade detectada (binário/flatpak
/// respondendo) — não prova de pacote; a interface trata como
/// "instalado"/"desconhecido" para não inventar estado
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub available: bool,
    pub version: Option<String>,
}

/// resultados da detecção por categoria, no formato consumido pela UI
#[derive(Debug, Default)]
pub struct CatalogSnapshot {
    pub distro: Option<DistroInfo>,
    pub launchers: Vec<AppEntry>,
    pub emuladores: Vec<AppEntry>,
    pub tools: Vec<ToolInfo>,
    pub compatibilidade: Vec<AppEntry>,
    pub criacao_gravacao: Vec<AppEntry>,
    pub criacao_video: Vec<AppEntry>,
    pub criacao_audio: Vec<AppEntry>,
    pub criacao_social: Vec<AppEntry>,
}

/// detecção completa do catálogo em consultas vivas (thread de fundo)
pub fn snapshot() -> CatalogSnapshot {
    CatalogSnapshot {
        distro: os::detect_distro(),
        launchers: tools::launchers(),
        emuladores: tools::emuladores(),
        tools: tools::detect(),
        compatibilidade: tools::compatibility(),
        criacao_gravacao: tools::criacao_gravacao(),
        criacao_video: tools::criacao_video(),
        criacao_audio: tools::criacao_audio(),
        criacao_social: tools::criacao_social(),
    }
}
