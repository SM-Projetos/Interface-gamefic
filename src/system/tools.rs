//! Catálogo da aba Apps e detecção central de estado de instalação.
//!
//! Porta de `src-tauri/src/system/tools.rs` do repositório Gamefic. As
//! diferenças em relação ao original, todas documentadas:
//! - sem ícones em data-URI: a interface Slint resolve os ícones embutidos
//!   pelo próprio `id` (AppIcon); o backend não entrega imagens;
//! - protons do Steam entram aqui via `steam::protons_custom()` e
//!   `steam::protons_oficiais()` (porta do repositório), sem data-URI;
//! - o memo de flatpaks (`flatpak_ids_estado`), que no repositório vive no
//!   install.rs, mora aqui enquanto o install.rs não é portado.
//!
//! Semântica de estado igual ao original (§2.2): sem consulta concluída,
//! o estado é "desconhecido" — nunca "não instalado" por chute.

use super::{AppEntry, ToolInfo};
use Availability::{Binario, PadraoFlatpak};

// ---------- catálogo ----------

pub fn detect() -> Vec<ToolInfo> {
    let definition: &[(&str, &'static str, Availability, &'static str)] = &[
        (
            "gamemode",
            "GameMode",
            either(Binario("gamemoded"), Binario("gamemode")),
            "Daemon de otimização para jogos",
        ),
        (
            "mangohud",
            "MangoHud",
            either(Binario("mangohud"), Binario("mangohud-gl.so")),
            "Overlay de desempenho em jogos",
        ),
        (
            "gamescope",
            "Gamescope",
            Binario("gamescope"),
            "Compositor de sessão de jogos",
        ),
        (
            "lact",
            "LACT",
            Binario("lact"),
            "Controle de GPU/ventoinha (AMD/NVIDIA/Intel)",
        ),
        (
            "goverlay",
            "GOverlay",
            Binario("goverlay"),
            "Configuração gráfica do MangoHud",
        ),
        (
            "winetricks",
            "Winetricks",
            Binario("winetricks"),
            "Utilitário de bibliotecas para prefixos Wine",
        ),
        (
            "protontricks",
            "Protontricks",
            either(
                Binario("protontricks"),
                PadraoFlatpak("com.github.Matoking.protontricks"),
            ),
            "Winetricks dirigido por jogos Steam",
        ),
        (
            "openrgb",
            "OpenRGB",
            either(Binario("openrgb"), PadraoFlatpak("org.openrgb.OpenRGB")),
            "Controle de iluminação RGB",
        ),
        (
            "oversteer",
            "Oversteer",
            PadraoFlatpak("io.github.berarma.Oversteer"),
            "Gerente de volantes de corrida",
        ),
    ];

    definition
        .iter()
        .map(|(key, name, availability, description)| ToolInfo {
            id: (*key).to_string(),
            name: (*name).to_string(),
            description: (*description).to_string(),
            available: availability.clone().check(),
            version: None,
        })
        .collect()
}

pub fn launchers() -> Vec<AppEntry> {
    let definition: &[(&str, &'static str, &'static str)] = &[
        ("steam", "Steam", "Loja e cliente de jogos da Valve"),
        ("lutris", "Lutris", "Gerenciador de jogos Wine/Proton"),
        (
            "heroic",
            "Heroic Games Launcher",
            "Launcher da Epic/GOG/Amazon",
        ),
        ("bottles", "Bottles", "Gerenciador de prefixos Wine"),
        ("itch", "Itch", "Loja indie da itch.io"),
        ("minigalaxy", "Minigalaxy", "Cliente GOG simples"),
    ];

    definition
        .iter()
        .map(|(key, name, description)| {
            let estado = estado_instalacao(key);
            AppEntry {
                id: (*key).to_string(),
                name: (*name).to_string(),
                description: (*description).to_string(),
                available: estado.estado == "instalado",
                version: estado.versao.clone(),
                estado: estado.estado.clone(),
                formato: estado.formato,
            }
        })
        .collect()
}

/// emuladores — estado/versão via `estado_instalacao` (REGISTRY + consulta local)
pub fn emuladores() -> Vec<AppEntry> {
    let definition: &[(&str, &'static str, &'static str)] = &[
        (
            "retroarch",
            "RetroArch",
            "Reúne vários emuladores em uma só interface",
        ),
        ("dolphin", "Dolphin", "Nintendo GameCube e Wii"),
        ("pcsx2", "PCSX2", "PlayStation 2"),
        ("rpcs3", "RPCS3", "PlayStation 3"),
        ("ppsspp", "PPSSPP", "PlayStation Portable (PSP)"),
        ("duck", "DuckStation", "PlayStation 1"),
        ("mgba", "mGBA", "Game Boy Advance"),
        ("mame", "MAME", "Jogos de arcade"),
    ];

    definition
        .iter()
        .map(|(key, name, description)| {
            let estado = estado_instalacao(key);
            AppEntry {
                id: (*key).to_string(),
                name: (*name).to_string(),
                description: (*description).to_string(),
                available: estado.estado == "instalado",
                version: estado.versao.clone(),
                estado: estado.estado.clone(),
                formato: estado.formato,
            }
        })
        .collect()
}

// ---------- criação de conteúdo, mesmo formato dos cards de Apps ----------

pub fn criacao_gravacao() -> Vec<AppEntry> {
    let definition: &[(&str, &'static str, &'static str)] = &[
        ("obs", "OBS Studio", "Gravação e streaming"),
        (
            "gpu-screen-recorder",
            "GPU Screen Recorder",
            "Gravação de gameplay",
        ),
    ];

    definition
        .iter()
        .map(|(key, name, description)| {
            let estado = estado_instalacao(key);
            AppEntry {
                id: (*key).to_string(),
                name: (*name).to_string(),
                description: (*description).to_string(),
                available: estado.estado == "instalado",
                version: estado.versao.clone(),
                estado: estado.estado.clone(),
                formato: estado.formato,
            }
        })
        .collect()
}

pub fn criacao_video() -> Vec<AppEntry> {
    let definition: &[(&str, &'static str, &'static str)] = &[
        ("kdenlive", "Kdenlive", "Edição de vídeo"),
        ("losslesscut", "LosslessCut", "Cortes rápidos"),
    ];

    definition
        .iter()
        .map(|(key, name, description)| {
            let estado = estado_instalacao(key);
            AppEntry {
                id: (*key).to_string(),
                name: (*name).to_string(),
                description: (*description).to_string(),
                available: estado.estado == "instalado",
                version: estado.versao.clone(),
                estado: estado.estado.clone(),
                formato: estado.formato,
            }
        })
        .collect()
}

pub fn criacao_audio() -> Vec<AppEntry> {
    let definition: &[(&str, &'static str, &'static str)] = &[
        ("audacity", "Audacity", "Edição de áudio"),
        (
            "easyeffects",
            "EasyEffects",
            "Processamento de áudio",
        ),
    ];

    definition
        .iter()
        .map(|(key, name, description)| {
            let estado = estado_instalacao(key);
            AppEntry {
                id: (*key).to_string(),
                name: (*name).to_string(),
                description: (*description).to_string(),
                available: estado.estado == "instalado",
                version: estado.versao.clone(),
                estado: estado.estado.clone(),
                formato: estado.formato,
            }
        })
        .collect()
}

pub fn criacao_social() -> Vec<AppEntry> {
    let definition: &[(&str, &'static str, &'static str)] = &[
        ("discord", "Discord", "Voz e chat para comunidades"),
        (
            "teamspeak",
            "TeamSpeak 3",
            "Voz de baixa latência para jogos",
        ),
    ];

    definition
        .iter()
        .map(|(key, name, description)| {
            let estado = estado_instalacao(key);
            AppEntry {
                id: (*key).to_string(),
                name: (*name).to_string(),
                description: (*description).to_string(),
                available: estado.estado == "instalado",
                version: estado.versao.clone(),
                estado: estado.estado.clone(),
                formato: estado.formato,
            }
        })
        .collect()
}

/// compatibilidade: Wine (nativo/direto) + Protons reais das instalações
/// Steam detectadas (customizados em `compatibilitytools.d` primeiro, depois
/// os oficiais gerenciados pelo cliente, sem repetir quem já saiu como
/// custom) — mesma ordem do repositório Gamefic
pub fn compatibility() -> Vec<AppEntry> {
    let mut entries: Vec<AppEntry> = Vec::new();

    let binario = Binario("wine").check();
    entries.push(AppEntry {
        id: "wine".to_string(),
        name: "Wine".to_string(),
        description: "Camada de compatibilidade Windows".to_string(),
        available: binario,
        version: versao("wine", &["--version"]).map(|v| v.lines().next().unwrap_or("").to_string()),
        estado: if binario { "instalado" } else { "desconhecido" }.to_string(),
        formato: binario.then(|| "nativo".to_string()),
    });

    // customizados em compatibilitytools.d (todas as raízes Steam), uma
    // entrada por nome — raízes que apontam para o mesmo lugar não repetem card
    for p in super::steam::protons_custom() {
        let id = format!("proton:{}", p.nome);
        if entries.iter().any(|e| e.id == id) {
            continue;
        }
        entries.push(AppEntry {
            id: format!("proton:{}", p.nome),
            name: p.nome.clone(),
            description: "Proton para Steam (compatibilitytools.d)".to_string(),
            available: true,
            version: p.versao,
            estado: "instalado".to_string(),
            formato: Some("Steam (custom)".to_string()),
        });
    }

    // oficiais gerenciados pelo Steam, em cada biblioteca detectada
    for p in super::steam::protons_oficiais() {
        if entries.iter().any(|e| e.id == format!("proton:{}", p.nome)) {
            continue;
        }
        entries.push(AppEntry {
            id: format!("proton:{}", p.nome),
            name: p.nome.clone(),
            description: "Proton gerenciado pelo Steam".to_string(),
            available: true,
            version: p.versao,
            estado: "instalado".to_string(),
            formato: Some("Steam (oficial)".to_string()),
        });
    }

    entries
}

// ---------- detecção central do estado de instalação (§2) ----------
// Mesma fonte de verdade dos detalhes do repo: pacotes do gerenciador
// nativo (só quando a família aplicável confirma), Flatpak por escopo e
// binário para ids fora do REGISTRY; conclusão sem consulta concluída vira
// "desconhecido", nunca "não instalado".

pub(crate) struct EstadoReal {
    pub estado: String,
    pub formato: Option<String>,
    pub versao: Option<String>,
}

/// detecção real de um aplicativo do catálogo pelo id, usando os
/// identificadores confiáveis do REGISTRY (pacote nativo, app-id do Flatpak)
pub(crate) fn estado_instalacao(id: &str) -> EstadoReal {
    let info = super::appinfo::achado(id);

    // flatpak pelo memo curto (mesma consulta, sem re-spawn por card)
    let flatpak_ids: Option<(Vec<(String, String)>, Vec<(String, String)>)> = flatpak_ids_estado();

    if let Some(i) = info {
        if let Some(fid) = i.flatpak {
            match flatpak_ids {
                None => {
                    if i.nativos
                        .iter()
                        .any(|p| super::pkgmgr::instalado(p.pkg).is_some())
                    {
                        let familia = super::os::detect_distro().and_then(|d| d.family);
                        return EstadoReal {
                            estado: "instalado".into(),
                            formato: Some("nativo".into()),
                            versao: i
                                .pacotes_aptos(familia.as_deref())
                                .find_map(|p| super::pkgmgr::instalado(p)),
                        };
                    }
                    // flatpak presente como única fonte conhecida mas consulta
                    // indisponível: não é possível descartar a instalação
                    return EstadoReal { estado: "desconhecido".into(), formato: None, versao: None };
                }
                Some((sistema, usuario)) => {
                    if let Some((_, v)) = sistema.iter().chain(usuario.iter()).find(|(a, _)| a == fid) {
                        let escopo = sistema
                            .iter()
                            .any(|(a, _)| a == fid)
                            .then(|| "sistema")
                            .unwrap_or("usuário");
                        return EstadoReal {
                            estado: "instalado".into(),
                            formato: Some(format!("Flatpak ({escopo})")),
                            versao: (!v.is_empty()).then(|| v.clone()),
                        };
                    }
                }
            }
        }
        for pkg in i.pacotes_aptos(super::os::detect_distro().and_then(|d| d.family).as_deref()) {
            if let Some(v) = super::pkgmgr::instalado(pkg) {
                return EstadoReal {
                    estado: "instalado".into(),
                    formato: Some("nativo".into()),
                    versao: Some(v),
                };
            }
        }
        return EstadoReal { estado: "nao_instalado".into(), formato: None, versao: None };
    }

    // id fora do REGISTRY: consulta só por binário (o estado depende do
    // comando existir de forma confiável)
    let achou = consegui_executar("which", [id]);
    EstadoReal {
        estado: if achou { "instalado".into() } else { "nao_instalado".into() },
        formato: achou.then(|| "binário".to_string()),
        versao: None,
    }
}

// ---------- memo curto das listas de flatpak (do install.rs do repo) ----------
// cada card do catálogo consulta o estado por app (30+ apps); com memória de
// curtíssimo prazo (TTL), um render inteiro usa UMA consulta do flatpak

fn flatpak_ids_memo() -> &'static std::sync::Mutex<
    Option<(
        std::time::Instant,
        Option<(Vec<(String, String)>, Vec<(String, String)>)>,
    )>,
> {
    static M: std::sync::OnceLock<
        std::sync::Mutex<
            Option<(
                std::time::Instant,
                Option<(Vec<(String, String)>, Vec<(String, String)>)>,
            )>,
        >,
    > = std::sync::OnceLock::new();
    M.get_or_init(Default::default)
}

const FLATPAK_IDS_TTL: std::time::Duration = std::time::Duration::from_secs(10);

/// listas de flatpaks instalados (sistema, usuário), do memo se fresco;
/// `Some(listas)` = flatpak presente; `None` = sem flatpak/comando falhou
pub fn flatpak_ids_estado() -> Option<(Vec<(String, String)>, Vec<(String, String)>)> {
    let mut l = flatpak_ids_memo().lock().expect("mutex flatpak ids");
    if let Some((t, Some(cachada))) = &*l {
        if t.elapsed() < FLATPAK_IDS_TTL {
            return Some(cachada.clone());
        }
    }
    if let Some((_, None)) = &*l {
        if l.as_ref().is_some_and(|(t, _)| t.elapsed() < FLATPAK_IDS_TTL) {
            return None;
        }
    }
    let presente = rodar("flatpak", &["--version"]).is_some();
    let listas = if presente {
        Some((flatpaks_escopo("--system"), flatpaks_escopo("--user")))
    } else {
        None
    };
    *l = Some((std::time::Instant::now(), listas.clone()));
    listas
}

/// ações de instalar/remover mudam as listas — memo inválido na hora
pub fn flatpak_ids_invalidate() {
    *flatpak_ids_memo().lock().expect("mutex flatpak ids") = None;
}

fn rodar(prog: &str, args: &[&str]) -> Option<String> {
    super::spawn::comando(prog)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn flatpaks_escopo(escopo: &str) -> Vec<(String, String)> {
    rodar(
        "flatpak",
        &["list", "--app", escopo, "--columns=application,version"],
    )
    .map(|s| listar_com_version(&s))
    .unwrap_or_default()
}

/// parser pura: linhas "id\tversão" das colunas do flatpak list
fn listar_com_version(saida: &str) -> Vec<(String, String)> {
    saida
        .lines()
        .filter_map(|l| {
            let mut p = l.split('\t');
            let id = p.next()?.trim().to_string();
            let versao = p.next().unwrap_or("").trim().to_string();
            Some((id, versao))
        })
        .collect()
}

// ---------- tipos de disponibilidade (mesma semântica, sem shell) ----------

#[derive(Clone)]
enum Availability {
    Binario(&'static str),
    PadraoFlatpak(&'static str),
    Qualquer(Box<Availability>, Box<Availability>),
}

impl Availability {
    fn check(self) -> bool {
        match self {
            Availability::Binario(b) => consegui_executar("which", [b]),
            Availability::PadraoFlatpak(id) => consegui_executar("flatpak", ["info", "--app", id]),
            Availability::Qualquer(a, b) => a.check() || b.check(),
        }
    }
}

fn either(a: Availability, b: Availability) -> Availability {
    Availability::Qualquer(Box::new(a), Box::new(b))
}

fn consegui_executar<A: AsRef<str>>(programa: &str, args: impl IntoIterator<Item = A>) -> bool {
    let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
    super::spawn::comando(programa)
        .args(&args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn versao(programa: &str, args: &[&str]) -> Option<String> {
    let output = super::spawn::comando(programa).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let primeira = text.trim().lines().next().unwrap_or("").trim();
    if primeira.is_empty() {
        return None;
    }
    let util = primeira
        .split_whitespace()
        .find(|token| !token.eq_ignore_ascii_case(programa) && *token != "-")?;
    let mut curta: String = util.chars().take(40).collect();
    if curta.ends_with('.') {
        curta.pop();
    }
    Some(curta)
}

#[cfg(test)]
mod tests {
    use super::listar_com_version;

    #[test]
    fn parser_flatpak_list() {
        let l = listar_com_version("io.itch.itch\t\norg.libretro.RetroArch\t1.19.0");
        assert_eq!(l[0], ("io.itch.itch".to_string(), String::new()));
        assert_eq!(l[1].0, "org.libretro.RetroArch");
        assert_eq!(l[1].1, "1.19.0");
        assert!(listar_com_version("").is_empty());
    }
}
