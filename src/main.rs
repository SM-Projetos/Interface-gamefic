//! Interface Gamefic (Slint) — ponto de entrada.
//!
//! O backend da aba Apps roda em thread de fundo: `system::snapshot()`
//! consulta o sistema real (pacotes nativos, flatpaks e binários) e os
//! modelos da UI são atualizados somente pela event loop do Slint
//! (`upgrade_in_event_loop`), como no app nativo do repositório Gamefic.

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Backend Winit explícito e identidade de app estável no desktop
    // (Wayland/X11), igual ao app nativo do repositório.
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()?;
    slint::set_xdg_app_id("com.sergio.gamefic")?;

    let ui = MainWindow::new()?;

    // A detecção consulta dezenas de programas do sistema: fora da thread
    // da interface, com nomes de modelo aplicados só pela event loop.
    let weak = ui.as_weak();
    std::thread::Builder::new()
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
                catalog.set_launchers(modelo(launchers));
                catalog.set_emuladores(modelo(emuladores));
                catalog.set_tools(modelo(tools));
                catalog.set_compatibilidade(modelo(compatibilidade));
                catalog.set_criacao_gravacao(modelo(gravacao));
                catalog.set_criacao_video(modelo(video));
                catalog.set_criacao_audio(modelo(audio));
                catalog.set_criacao_social(modelo(social));
                catalog.set_status(status);
                catalog.set_loaded(true);
            });
        })?;

    ui.run()?;
    Ok(())
}
