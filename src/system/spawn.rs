//! Execução de comandos de sistema com consciência de sandbox.
//!
//! Porta de `src-tauri/src/system/spawn.rs` do repositório Gamefic
//! (branch gamefic-slint, GPL-3.0-only). Quando o aplicativo roda como
//! Flatpak (env `FLATPAK_ID` presente), os comandos de detecção precisam
//! acontecer no HOST — dentro do sandbox não há pacman/flatpak do sistema
//! e qualquer spawn ficaria limitado ao isolamento. Fora do Flatpak, o
//! comportamento nativo fica idêntico ao de sempre.

use std::process::Command;

/// `Command` para executar `programa` no lugar certo: direto (nativo) ou via
/// `flatpak-spawn --host` quando o app está no sandbox do Flatpak.
/// (O repo usa também `--watch`; esta cópia omite — flatpaks hosts antigos
/// rejeitam a opção, e consultas one-shot não precisam dela.)
pub fn comando(programa: &str) -> Command {
    if std::env::var_os("FLATPAK_ID").is_some() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg(programa);
        c
    } else {
        Command::new(programa)
    }
}
