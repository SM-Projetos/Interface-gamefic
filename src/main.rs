//! Interface Gamefic (Slint) — ponto de entrada.
//!
//! O backend da aba Apps roda em thread de fundo: `system::snapshot()`
//! consulta o sistema real (pacotes nativos, flatpaks e binários) e os
//! modelos da UI são atualizados somente pela event loop do Slint
//! (`upgrade_in_event_loop`), como no app nativo do repositório Gamefic.
//! As ações de abrir/instalar/remover/atualizar e a listagem de fontes
//! também rodam fora da thread da interface, com guarda de concorrência.

mod system;

slint::include_modules!();

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;

/// linha da UI para um AppEntry detectado: desenvolvedor vem do REGISTRY
/// pelo id; versão/origem apenas com confirmação, senão honestos vazios.
fn linha_de_app(e: &system::AppEntry) -> AppInfo {
    let developer = system::appinfo::achado(&e.id)
        .map(|i| SharedString::from(i.fornecedor))
        .unwrap_or_else(|| SharedString::from("—"));
    AppInfo {
        id: SharedString::from(e.id.as_str()),
        name: SharedString::from(e.name.as_str()),
        description: SharedString::from(e.description.as_str()),
        estado: SharedString::from(e.estado.as_str()),
        version: SharedString::from(e.version.as_deref().unwrap_or("Não detectada")),
        developer,
        source: SharedString::from(e.formato.as_deref().unwrap_or("Não consultada")),
        has_update: false,
    }
}

/// linha de ferramenta: disponibilidade é capacidade detectada (binário/
/// flatpak respondendo), não prova de pacote — sem prova, o estado mostrado
/// é "desconhecido", como na tool_cards do app nativo do repositório.
fn linha_de_tool(t: &system::ToolInfo) -> AppInfo {
    let developer = system::appinfo::achado(&t.id)
        .map(|i| SharedString::from(i.fornecedor))
        .unwrap_or_else(|| SharedString::from("—"));
    AppInfo {
        id: SharedString::from(t.id.as_str()),
        name: SharedString::from(t.name.as_str()),
        description: SharedString::from(t.description.as_str()),
        estado: SharedString::from(if t.available { "instalado" } else { "desconhecido" }),
        version: SharedString::from("Não detectada"),
        developer,
        source: SharedString::from("Não consultada"),
        has_update: false,
    }
}

fn modelo(lines: Vec<AppInfo>) -> ModelRc<AppInfo> {
    Rc::new(VecModel::from(lines)).into()
}

/// linha de GPU para a aba Kernels e Drivers: os campos voltem sempre com
/// valor apresentável — "Não disponível" quando a detecção não respondeu.
fn linha_de_gpu(g: &system::hardware::GpuInfo) -> GpuLine {
    GpuLine {
        vendor: SharedString::from(
            g.vendor_name.as_deref().unwrap_or("Não disponível"),
        ),
        device: SharedString::from(
            g.device_name.as_deref().unwrap_or("Dispositivo gráfico"),
        ),
        pci: SharedString::from(g.pci_address.as_deref().unwrap_or("Não disponível")),
        ids: SharedString::from(format!(
            "{}:{}",
            g.vendor_id.as_deref().unwrap_or("?"),
            g.device_id.as_deref().unwrap_or("?")
        )),
        driver: SharedString::from(g.driver.as_deref().unwrap_or("Não disponível")),
    }
}

fn linha_de_vulkan(v: &system::hardware::VulkanGpu) -> VulkanLine {
    VulkanLine {
        device: SharedString::from(
            v.device_name.as_deref().unwrap_or("Dispositivo Vulkan"),
        ),
        driver: SharedString::from(v.driver_name.as_deref().unwrap_or("Não disponível")),
    }
}

/// linha de ferramenta de jogos: `available` é capacidade detectada
/// (binário/flatpak respondendo); sem prova, "Não instalada" é honesto
/// porque a ferramenta não respondeu no sistema.
fn linha_de_ferramenta(t: &system::ToolInfo) -> ToolLine {
    ToolLine {
        name: SharedString::from(t.name.as_str()),
        description: SharedString::from(t.description.as_str()),
        installed: t.available,
    }
}

/// frequência base formatada: MHz abaixo de 1000, GHz com uma casa acima
fn frequencia(mhz: Option<u32>) -> SharedString {
    SharedString::from(match mhz {
        None => "Não disponível".to_string(),
        Some(mhz) if mhz >= 1000 => format!("{:.1} GHz", mhz as f32 / 1000.0),
        Some(mhz) => format!("{mhz} MHz"),
    })
}

/// contagem de CPU: "—" quando não detectada (degradado honesto)
fn contagem(valor: Option<u32>) -> SharedString {
    SharedString::from(
        valor
            .map(|v| v.to_string())
            .unwrap_or_else(|| "—".to_string()),
    )
}

/// linha de estado do sistema para a aba Otimização; sem fonte de leitura,
/// o detalhe fica vazio e a UI omite a linha
fn linha_de_recurso(r: &system::recommendations::SystemResource) -> OptResource {
    OptResource {
        name: SharedString::from(r.name.as_str()),
        state: SharedString::from(r.state.as_str()),
        detail: SharedString::from(r.detail.as_deref().unwrap_or("")),
    }
}

fn linha_de_recomendacao(r: &system::recommendations::Recommendation) -> OptRecommendation {
    OptRecommendation {
        title: SharedString::from(r.title.as_str()),
        rationale: SharedString::from(r.rationale.as_str()),
        beneficio: SharedString::from(r.expected_benefit.as_str()),
        limitacoes: SharedString::from(r.limit_note.as_str()),
        risk: SharedString::from(match r.risk {
            system::recommendations::RiskLevel::Baixo => "baixo",
            system::recommendations::RiskLevel::Medio => "medio",
            system::recommendations::RiskLevel::Alto => "alto",
        }),
        actionable: r.action_available,
    }
}

fn linha_de_perfil(p: &system::recommendations::ProfileScope) -> (String, String, Vec<SharedString>, i32, String) {
    (
        p.name.clone(),
        p.description.clone(),
        p.planned_adjustments
            .iter()
            .map(|a| SharedString::from(format!("• {a}")))
            .collect(),
        p.implemented_adjustments as i32,
        format!(
            "{} de {} ajustes implementáveis nesta versão",
            p.implemented_adjustments,
            p.planned_adjustments.len()
        ),
    )
}

/// análise de otimização em thread de fundo (`system::recommendations`),
/// aplicada à UI pela event loop; disparada na inicialização e pelo
/// refresh do global
fn analisar(weak: slint::Weak<MainWindow>) {
    let _ = std::thread::Builder::new()
        .name("gamefic-analise".to_string())
        .spawn(move || {
            let analise = system::recommendations::analyze();

            let recursos = analise.resources.iter().map(linha_de_recurso).collect::<Vec<_>>();
            let recomendacoes = analise
                .recommendations
                .iter()
                .map(linha_de_recomendacao)
                .collect::<Vec<_>>();
            let perfis = analise.profiles.iter().map(linha_de_perfil).collect::<Vec<_>>();

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let dados = ui.global::<OtimizacaoData>();
                dados.set_recursos(Rc::new(VecModel::from(recursos)).into());
                dados.set_recomendacoes(Rc::new(VecModel::from(recomendacoes)).into());
                // os modelos de lista são montados aqui na event loop: Rc
                // não atravessa threads (mesma regra do catálogo)
                dados.set_perfis(
                    Rc::new(VecModel::from(
                        perfis
                            .into_iter()
                            .map(|(name, description, ajustes, implemented, resumo)| OptProfile {
                                name: name.into(),
                                description: description.into(),
                                adjustments: Rc::new(VecModel::from(ajustes)).into(),
                                implemented,
                                resumo: SharedString::from(resumo),
                            })
                            .collect::<Vec<_>>(),
                    ))
                    .into(),
                );
                dados.set_loaded(true);
            });
        });
}

/// detecção de kernel/GPUs/drivers em thread de fundo (`system::hardware`),
/// aplicada à UI pela event loop; disparada na inicialização e pelo
/// botão de atualizar da aba
fn detectar_kernels(weak: slint::Weak<MainWindow>) {
    let _ = std::thread::Builder::new()
        .name("gamefic-kernels".to_string())
        .spawn(move || {
            let snap = system::hardware::snapshot();

            let gpus = snap.gpus.iter().map(linha_de_gpu).collect::<Vec<_>>();
            let vulkan_gpus = snap.vulkan.iter().flat_map(|v| v.gpus.iter().map(linha_de_vulkan)).collect::<Vec<_>>();
            let ferramentas = snap.ferramentas.iter().map(linha_de_ferramenta).collect::<Vec<_>>();

            let distro_nome = snap
                .distro
                .as_ref()
                .map(|d| SharedString::from(d.name.as_str()))
                .unwrap_or_else(|| "Não disponível".into());
            let distro_familia = snap
                .distro
                .as_ref()
                .and_then(|d| d.family.as_deref())
                .map(SharedString::from)
                .unwrap_or_else(|| "Não disponível".into());
            let distro_versao = snap
                .distro
                .as_ref()
                .and_then(|d| d.version.as_deref())
                .map(SharedString::from)
                .unwrap_or_else(|| "Não disponível".into());
            let kernel = SharedString::from(
                snap.kernel.as_deref().unwrap_or("Não disponível"),
            );
            let arch = SharedString::from(snap.arch.as_deref().unwrap_or("Não disponível"));
            let (cpu_modelo, cpu_fisicos, cpu_threads, cpu_freq) = match &snap.cpu {
                Some(cpu) => (
                    SharedString::from(
                        cpu.model.as_deref().unwrap_or("Não disponível"),
                    ),
                    contagem(cpu.cores_physical),
                    contagem(cpu.threads_logical),
                    frequencia(cpu.base_freq_mhz),
                ),
                None => (
                    "Não disponível".into(),
                    contagem(None),
                    contagem(None),
                    frequencia(None),
                ),
            };

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let kernel_data = ui.global::<KernelData>();
                kernel_data.set_distro_nome(distro_nome);
                kernel_data.set_distro_familia(distro_familia);
                kernel_data.set_distro_versao(distro_versao);
                kernel_data.set_kernel_execucao(kernel);
                kernel_data.set_kernel_arch(arch);
                kernel_data.set_cpu_modelo(cpu_modelo);
                kernel_data.set_cpu_fisicos(cpu_fisicos);
                kernel_data.set_cpu_threads(cpu_threads);
                kernel_data.set_cpu_freq(cpu_freq);
                kernel_data.set_gpus(Rc::new(VecModel::from(gpus)).into());
                kernel_data.set_vulkan_gpus(Rc::new(VecModel::from(vulkan_gpus)).into());
                kernel_data.set_ferramentas(Rc::new(VecModel::from(ferramentas)).into());
                kernel_data.set_loaded(true);
            });
        });
}

/// mensagem de ação na caixa de informação do modal de detalhes
fn mensagem(weak: &slint::Weak<MainWindow>, texto: String) {
    let _ = weak.upgrade_in_event_loop(move |ui| {
        ui.global::<CatalogData>().set_acao_mensagem(texto.into());
    });
}

/// detecção completa em thread de fundo; o resultado (modelos, status e o
/// modal de detalhes aberto, se houver) é aplicado pela event loop
fn detectar(weak: slint::Weak<MainWindow>) {
    // a thread falha só quando o processo está terminando; join_id não é usado
    let _ = std::thread::Builder::new()
        .name("gamefic-system-detection".to_string())
        .spawn(move || {
            let snap = system::snapshot();
            let status = match &snap.distro {
                Some(d) => SharedString::from(format!("Detecção concluída no sistema: {}", d.name)),
                None => SharedString::from("Detecção concluída (distribuição não identificada)"),
            };

            let launchers = snap.launchers.iter().map(linha_de_app).collect::<Vec<_>>();
            let emuladores = snap.emuladores.iter().map(linha_de_app).collect::<Vec<_>>();
            let tools = snap.tools.iter().map(linha_de_tool).collect::<Vec<_>>();
            let compatibilidade = snap.compatibilidade.iter().map(linha_de_app).collect::<Vec<_>>();
            let gravacao = snap.criacao_gravacao.iter().map(linha_de_app).collect::<Vec<_>>();
            let video = snap.criacao_video.iter().map(linha_de_app).collect::<Vec<_>>();
            let audio = snap.criacao_audio.iter().map(linha_de_app).collect::<Vec<_>>();
            let social = snap.criacao_social.iter().map(linha_de_app).collect::<Vec<_>>();

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let catalog = ui.global::<CatalogData>();
                catalog.set_launchers(modelo(launchers.clone()));
                catalog.set_emuladores(modelo(emuladores.clone()));
                catalog.set_tools(modelo(tools.clone()));
                catalog.set_compatibilidade(modelo(compatibilidade.clone()));
                catalog.set_criacao_gravacao(modelo(gravacao.clone()));
                catalog.set_criacao_video(modelo(video.clone()));
                catalog.set_criacao_audio(modelo(audio.clone()));
                catalog.set_criacao_social(modelo(social.clone()));
                catalog.set_status(status);
                catalog.set_loaded(true);

                // modal aberto na hora da re-detecção: campos sincronizam
                // com o estado real recém-consultado (nada fica congelado)
                let modal = ui.global::<CatalogModal>();
                if modal.get_open() {
                    let alvo = modal.get_app_id().to_string();
                    if let Some(a) = launchers
                        .iter()
                        .chain(emuladores.iter())
                        .chain(tools.iter())
                        .chain(compatibilidade.iter())
                        .chain(gravacao.iter())
                        .chain(video.iter())
                        .chain(audio.iter())
                        .chain(social.iter())
                        .find(|a| a.id.as_str() == alvo)
                    {
                        modal.set_estado(a.estado.clone());
                        modal.set_version(a.version.clone());
                        modal.set_source(a.source.clone());
                    }
                }
            });
        });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Backend Winit como padrão (identidade de app estável no desktop
    // Wayland/X11, igual ao app nativo do repositório). `SLINT_BACKEND`
    // no ambiente sobrepõe — usado para diagnóstico em backend headless.
    let backend = std::env::var("SLINT_BACKEND").unwrap_or_else(|_| "winit".into());
    slint::BackendSelector::new()
        .backend_name(backend.into())
        .select()?;
    slint::set_xdg_app_id("com.sergio.gamefic")?;

    let ui = MainWindow::new()?;

    // guarda de concorrência: uma ação de sistema por vez (instalar/remover/
    // atualizar consultam e mudam o estado real; duas em paralelo se
    // contradiriam nas validações)
    let ocupado = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let catalog = ui.global::<CatalogData>();
    catalog.on_abrir_app({
        let weak = ui.as_weak();
        move |id| {
            let weak = weak.clone();
            std::thread::Builder::new()
                .name("gamefic-abrir".to_string())
                .spawn(move || {
                    let resultado = system::install::abrir_launcher(id.as_str(), false);
                    mensagem(&weak, match resultado {
                        Ok(msg) => msg,
                        Err(e) => format!("Não foi possível abrir: {e}"),
                    });
                })
                .expect("thread abrir");
        }
    });

    catalog.on_acao_app({
        let weak = ui.as_weak();
        let ocupado = ocupado.clone();
        move |id, acao| {
            let weak = weak.clone();
            let ocupado = ocupado.clone();
            std::thread::Builder::new()
                .name("gamefic-acao".to_string())
                .spawn(move || {
                    let id = id.to_string();
                    let acao = acao.to_string();
                    if ocupado
                        .compare_exchange(
                            false,
                            true,
                            std::sync::atomic::Ordering::SeqCst,
                            std::sync::atomic::Ordering::SeqCst,
                        )
                        .is_err()
                    {
                        return mensagem(&weak, "Já existe uma operação em andamento — aguarde a conclusão.".into());
                    }
                    let livre = || {
                        ocupado.store(false, std::sync::atomic::Ordering::SeqCst);
                    };
                    match system::install::fonte_padrao(&id, &acao) {
                        None => {
                            livre();
                            mensagem(&weak, format!("Nenhuma fonte atende a '{acao}' para este aplicativo — confira o estado e a fonte com 'Escolher fonte'."));
                        }
                        Some(fonte) => {
                            mensagem(&weak, format!("Executando {acao} via {fonte}… a elevação usa pkexec e pode pedir sua senha."));
                            match system::install::acao_app(&id, &acao, &fonte) {
                                Ok(msg) => {
                                    livre();
                                    // re-detecção completa, com estados reais pós-operação
                                    detectar(weak.clone());
                                    mensagem(&weak, format!("Última ação: {msg} via {fonte}."));
                                }
                                Err(e) => {
                                    livre();
                                    mensagem(&weak, format!("Falha em '{acao}' via {fonte}: {e}"));
                                }
                            }
                        }
                    }
                })
                .expect("thread ação");
        }
    });

    catalog.on_fontes_app({
        let weak = ui.as_weak();
        move |id| {
            let weak = weak.clone();
            std::thread::Builder::new()
                .name("gamefic-fontes".to_string())
                .spawn(move || {
                    let id = id.to_string();
                    let fontes = system::install::fontes(&id);
                    let texto = if fontes.is_empty() {
                        "Nenhuma fonte conhecida para este aplicativo no catálogo.".to_string()
                    } else {
                        let mut linhas = Vec::new();
                        for f in &fontes {
                            let estado_fonte = if f.instalado {
                                "instalado".to_string()
                            } else if f.disponivel {
                                "disponível".to_string()
                            } else if f.indeterminado {
                                "indeterminado".to_string()
                            } else {
                                "indisponível".to_string()
                            };
                            let detalhe = match (&f.versao, &f.motivo) {
                                (Some(v), Some(m)) => format!("v{v}; {m}"),
                                (Some(v), None) => format!("v{v}"),
                                (None, Some(m)) => m.clone(),
                                (None, None) => String::new(),
                            };
                            linhas.push(if detalhe.is_empty() {
                                format!("• {} — {estado_fonte}", f.fonte)
                            } else {
                                format!("• {} — {estado_fonte} ({detalhe})", f.fonte)
                            });
                        }
                        format!("Fontes reais detectadas: {}", linhas.join("  |  "))
                    };
                    mensagem(&weak, texto);
                })
                .expect("thread fontes");
        }
    });

    let kernel_data = ui.global::<KernelData>();
    kernel_data.on_refresh({
        let weak = ui.as_weak();
        move || {
            detectar_kernels(weak.clone());
        }
    });

    let otimizacao_data = ui.global::<OtimizacaoData>();
    otimizacao_data.on_refresh({
        let weak = ui.as_weak();
        move || {
            analisar(weak.clone());
        }
    });

    detectar(ui.as_weak());
    detectar_kernels(ui.as_weak());
    analisar(ui.as_weak());

    ui.run()?;
    Ok(())
}
