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
    // Backend Winit explícito e identidade de app estável no desktop
    // (Wayland/X11), igual ao app nativo do repositório.
    slint::BackendSelector::new()
        .backend_name("winit".into())
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

    detectar(ui.as_weak());

    ui.run()?;
    Ok(())
}
