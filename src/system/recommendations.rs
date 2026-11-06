//! Análise de oportunidades de otimização — porto de `recommendations.rs`
//! do repositório Gamefic (Rust puro, sem Tauri/serde).
//!
//! A análise lê apenas dados reais: governadores de `/sys/devices/system/
//! cpu/*/cpufreq/scaling_governor`, ferramentas pelo mesmo `tools::detect()`
//! do catálogo (casamento por id, estável entre catálogos), suporte Vulkan
//! por `hardware::detect_vulkan()` e swap por `/proc/meminfo`.
//!
//! Nenhuma recomendação é aplicada nesta versão: `action_available` é
//! sempre falso e os perfis são estrutura para ativação em etapa futura,
//! como no app original.

use super::hardware;
use super::tools;
use super::ToolInfo;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Baixo,
    Medio,
    Alto,
}

pub struct Recommendation {
    pub id: String,
    pub title: String,
    pub rationale: String,
    pub expected_benefit: String,
    pub limit_note: String,
    pub risk: RiskLevel,
    pub action_available: bool,
}

pub struct SystemResource {
    pub id: String,
    pub name: String,
    pub state: String,
    pub detail: Option<String>,
}

pub struct ProfileScope {
    pub id: String,
    pub name: String,
    pub description: String,
    pub planned_adjustments: Vec<String>,
    pub implemented_adjustments: u32,
}

pub struct Analysis {
    pub resources: Vec<SystemResource>,
    pub recommendations: Vec<Recommendation>,
    pub profiles: Vec<ProfileScope>,
}

/// análise completa; consultas vivas (thread de fundo na main.rs)
pub fn analyze() -> Analysis {
    let mut resources = Vec::new();
    let mut recommendations = Vec::new();

    let governors = read_governors();
    if let Some(g) = governors.as_ref().and_then(|v| v.first().cloned()) {
        resources.push(SystemResource {
            id: "governador-cpu".to_string(),
            name: "Governadores de CPU".to_string(),
            state: g.clone(),
            detail: Some(
                "Leitura de /sys/devices/system/cpu/*/cpufreq/scaling_governor".to_string(),
            ),
        });

        let resume = match g.as_str() {
            "powersave" => {
                "Com governador powersave a CPU tolera maior latência sob carga — considerar configuração de desempenho para jogos."
            }
            "performance" => {
                "Governador performance já ativo; sem recomendação necessária neste item."
            }
            _ => "Governador alternativo ativo; avaliar conforme o uso.",
        };

        if g.as_str() == "powersave" {
            recommendations.push(Recommendation {
                id: "governador-desempenho".to_string(),
                title: "Governador de CPU em modo desempenho ao jogar".to_string(),
                rationale: format!(
                    "Detecção encontrou governor '{g}' em pelo menos um núcleo."
                ),
                expected_benefit:
                    "Menos variação de frequência em picos de carga dos jogos".to_string(),
                limit_note:
                    "Ajuste real de governador exige implementação com confirmação e reversão — não disponível nesta versão"
                        .to_string(),
                risk: RiskLevel::Baixo,
                action_available: false,
            });
        } else {
            recommendations.push(Recommendation {
                id: "governador-informativo".to_string(),
                title: "Estado do governador de CPU".to_string(),
                rationale: resume.to_string(),
                expected_benefit: "Informativo".to_string(),
                limit_note: "Nenhuma alteração recomendada".to_string(),
                risk: RiskLevel::Baixo,
                action_available: false,
            });
        }
    } else {
        resources.push(SystemResource {
            id: "governador-cpu".to_string(),
            name: "Governadores de CPU".to_string(),
            state: "Não disponível".to_string(),
            detail: Some("Interface cpufreq não exposta pelo sistema".to_string()),
        });
    }

    let tools_detectados = tools::detect();

    let gamemode = disponivel(&tools_detectados, "gamemode");
    resources.push(SystemResource {
        id: "gamemode".to_string(),
        name: "Gamemode (daemon)".to_string(),
        state: if gamemode {
            "Presente".to_string()
        } else {
            "Não instalado".to_string()
        },
        detail: Some("Detecção pelo binário gamemoded".to_string()),
    });

    if !gamemode {
        recommendations.push(Recommendation {
            id: "instalar-gamemode".to_string(),
            title: "Instalar o daemon Gamemode".to_string(),
            rationale:
                "O daemon otimiza governadores e outros recursos automaticamente durante o jogo; não foi encontrado no sistema."
                    .to_string(),
            expected_benefit:
                "Otimizações automáticas enquanto joga, sem intervenção manual".to_string(),
            limit_note:
                "Instalação por gerenciador de pacotes ainda não implementada (Etapa futura acordada por API da distribuição)"
                    .to_string(),
            risk: RiskLevel::Medio,
            action_available: false,
        });
    }

    let mangohud = disponivel(&tools_detectados, "mangohud");
    resources.push(SystemResource {
        id: "mangohud".to_string(),
        name: "MangoHud (overlay)".to_string(),
        state: if mangohud {
            "Presente".to_string()
        } else {
            "Não instalado".to_string()
        },
        detail: Some("Detecção pelo binário mangohud".to_string()),
    });

    if !mangohud {
        recommendations.push(Recommendation {
            id: "instalar-mangohud".to_string(),
            title: "Instalar o MangoHud".to_string(),
            rationale:
                "Overlay de desempenho útil para medir FPS e uso de recursos dentro dos jogos; não foi encontrado."
                    .to_string(),
            expected_benefit:
                "Visibilidade do desempenho dentro dos jogos".to_string(),
            limit_note: "Instalação por pacotes não implementada nesta versão".to_string(),
            risk: RiskLevel::Baixo,
            action_available: false,
        });
    }

    if let Some(vulkan) = hardware::detect_vulkan() {
        let estado = if vulkan.gpus.is_empty() {
            "Nenhum dispositivo reportado".to_string()
        } else {
            "Disponível".to_string()
        };

        resources.push(SystemResource {
            id: "vulkan".to_string(),
            name: "Suporte Vulkan".to_string(),
            state: estado,
            detail: Some(
                vulkan
                    .gpus
                    .iter()
                    .filter_map(|g| g.driver_name.clone())
                    .collect::<Vec<_>>()
                    .join(" · "),
            ),
        });
    } else {
        resources.push(SystemResource {
            id: "vulkan".to_string(),
            name: "Suporte Vulkan".to_string(),
            state: "Não disponível — vulkaninfo ausente".to_string(),
            detail: None,
        });
    }

    if let Some(swap) = swap_total_kb().filter(|s| *s > 0) {
        resources.push(SystemResource {
            id: "swap".to_string(),
            name: "Swap configurado".to_string(),
            state: format!("{} MiB", swap / 1024),
            detail: Some(
                "Leitura de /proc/meminfo; verificação de zram em etapa futura".to_string(),
            ),
        });
    }

    let profiles = vec![
        ProfileScope {
            id: "equilibrado".to_string(),
            name: "Equilibrado".to_string(),
            description: "Sem alterações agressivas; equilíbrio entre desempenho e consumo"
                .to_string(),
            planned_adjustments: vec!["Governador da CPU em schedutil/ondemand".to_string()],
            implemented_adjustments: 0,
        },
        ProfileScope {
            id: "gamer".to_string(),
            name: "Gamer".to_string(),
            description: "Prioriza desempenho em jogos".to_string(),
            planned_adjustments: vec![
                "Governador da CPU em performance".to_string(),
                "Ativação automática via Gamemode".to_string(),
            ],
            implemented_adjustments: 0,
        },
        ProfileScope {
            id: "economia".to_string(),
            name: "Economia de energia".to_string(),
            description: "Prioriza economia de energia e menor geração de calor".to_string(),
            planned_adjustments: vec![
                "Governador da CPU em powersave".to_string(),
                "Limite prático de temperatura de destino (se exposto pelo sistema)"
                    .to_string(),
            ],
            implemented_adjustments: 0,
        },
    ];

    Analysis {
        resources,
        recommendations,
        profiles,
    }
}

/// disponibilidade por id do catálogo (mais estável que o nome exibido)
fn disponivel(ferramentas: &[ToolInfo], id: &str) -> bool {
    ferramentas
        .iter()
        .any(|t| t.id == id && t.available)
}

/// SwapTotal de /proc/meminfo, em KiB (None quando não legível)
fn swap_total_kb() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("SwapTotal:") {
            let kb: u64 = value.trim().trim_end_matches(" kB").trim().parse().ok()?;
            return Some(kb);
        }
    }

    None
}

/// governors únicos encontrados nos núcleos expostos pelo cpufreq
fn read_governors() -> Option<Vec<String>> {
    let mut governors = Vec::new();

    if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            if !(name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit())) {
                continue;
            }

            if let Ok(gov) = fs::read_to_string(entry.path().join("cpufreq/scaling_governor"))
            {
                let trimmed = gov.trim();
                if !trimmed.is_empty() && !governors.iter().any(|g| g == trimmed) {
                    governors.push(trimmed.to_string());
                }
            }
        }
    }

    if governors.is_empty() {
        None
    } else {
        Some(governors)
    }
}
