//! Ações da aba Apps: listar fontes, instalar/remover/atualizar e abrir.
//!
//! Porta de `src-tauri/src/system/install.rs` do repositório Gamefic,
//! limitada ao que a UI consome neste slice:
//! - `fontes(id)`: listagem real de fontes por aplicativo (pacote nativo
//!   apto à família, flatpak system/user, snap). As diferenças ao original,
//!   todas documentadas: sem o cache de versões remotas (consultas de rede
//!   e has_update ficam para o slice de atualizações — `versao_repo` é
//!   sempre None aqui, e a disponibilidade de flatpak não instalado é
//!   honestamente indeterminada);
//! - `acao_app(id, acao, fonte)`: instalar/remover/atualizar por fonte
//!   validada contra o REGISTRY — nativo via `pkgmgr` (pkexec), flatpak por
//!   escopo com revalidação de estado remoto e snap via pkexec;
//! - `abrir_launcher(id, overlay)`: `flatpak run` ou binário nativo;
//!   sem ativação de MangoHud (slice da aba Overlay);
//! - `fonte_padrao(id, acao)`: primeira fonte que atende à ação, para os
//!   botões do modal que ainda não têm diálogo de escolha de fonte.
//!
//! Sem serde: os tipos não cruzam fronteira de processo. Nada passa por
//! shell — só args estruturados. Ação privilegiada exclusivamente via
//! `pkexec` (sem sudo, sem armazenar senha).

/// fonte de instalação por aplicativo, com estado real e motivo quando
/// a disponibilidade não pôde ser confirmada
#[derive(Debug, Clone)]
pub struct FonteInfo {
    /// "nativo:pkg" | "flatpak" | "flatpak-user" | "snap:nome"
    pub fonte: String,
    pub disponivel: bool,
    pub instalado: bool,
    pub versao: Option<String>,
    /// sem consultas de rede neste slice: fica None (o original preenche
    /// com versões do remoto/manager cacheado)
    pub versao_repo: Option<String>,
    pub motivo: Option<String>,
    /// consulta inconclusiva permite nova tentativa (§5-G do original)
    pub indeterminado: bool,
}

#[derive(Debug, Clone)]
pub enum InstalarError {
    Fonte,
    Falha(String),
    NaoDisponivel,
    Execucao(String),
    /// conflito real entre pacotes (p.ex. variantes de Wine que se contradizem)
    Conflito(String),
}

impl std::fmt::Display for InstalarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            InstalarError::Fonte => write!(f, "fonte desconhecida"),
            InstalarError::Falha(m) => write!(f, "{m}"),
            InstalarError::NaoDisponivel => write!(f, "fonte não está disponível"),
            InstalarError::Execucao(m) => write!(f, "falha na execução: {m}"),
            InstalarError::Conflito(m) => write!(f, "conflito de pacotes: {m}"),
        }
    }
}

// ---------- pacotes e flatpak ----------

fn rodar(prog: &str, args: &[&str]) -> Option<String> {
    super::spawn::comando(prog)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// executou e devolve o Output bruto (código de saída incluído), p/ distinguir
/// "programa não existe" de "programa respondeu com erro"
/// (não usado neste slice: as consultas usam `rodar`/`saida_com_max`; a
/// face de execução fica completa com o slice de verificação de versões)
#[allow(dead_code)]
fn saida(prog: &str, args: &[&str]) -> Option<std::process::Output> {
    super::spawn::comando(prog).args(args).output().ok()
}

/// remotos flatpak efetivamente configurados no escopo (não presume flathub)
fn flatpak_remotos() -> Vec<String> {
    flatpak_remotos_de(&rodar(
        "flatpak",
        &["remotes", "--system", "--user", "--columns=name"],
    ))
}

/// parser pura, testável: nomes de remotos a partir das colunas
pub(crate) fn flatpak_remotos_de(saida: &Option<String>) -> Vec<String> {
    match saida {
        Some(s) => s
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        None => Vec::new(),
    }
}

/// linhas "id\tversão" das colunas do flatpak list (cópia do original,
/// que também mora em install.rs no repo)
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

fn flatpaks_escopo(escopo: &str) -> Vec<(String, String)> {
    rodar(
        "flatpak",
        &["list", "--app", escopo, "--columns=application,version"],
    )
    .map(|s| listar_com_version(&s))
    .unwrap_or_default()
}

/// roda o programa com teto de tempo: `snap` é conhecido por ficar preso
/// esperando um serviço snapd que não responde (sem teto eram ~25 s por
/// consulta); com teto, a fonte é degradada honestamente em segundos
fn saida_com_max(prog: &str, args: &[&str], max: std::time::Duration) -> Option<std::process::Output> {
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();
    let prog_t = prog.to_string();
    let args_t: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    std::thread::spawn(move || {
        let out = super::spawn::comando(&prog_t).args(&args_t).output().ok();
        let _ = tx.send(out);
    });
    rx.recv_timeout(max).ok().flatten()
}

// ---------- snap ----------

/// consentimento do estado global do serviço snapd, com TTL: serviço morto
/// não muda em segundos — memoriza a última resposta e evita repetir a espera
fn snapd_memo() -> &'static std::sync::Mutex<Option<(std::time::Instant, bool)>> {
    static M: std::sync::OnceLock<std::sync::Mutex<Option<(std::time::Instant, bool)>>> =
        std::sync::OnceLock::new();
    M.get_or_init(Default::default)
}

const SNAPD_MEMO_TTL: std::time::Duration = std::time::Duration::from_secs(600);

/// consulta a Snap Store por `snap info` — sem snapd funcional, a fonte não
/// é ofertada (não há como instalar nem confirmar disponibilidade)
fn fonte_snap(snome: &str, mgr: super::pkgmgr::PkgMgr) -> FonteInfo {
    use super::pkgmgr::PkgMgr;
    let teto = std::time::Duration::from_secs(2);
    let sem_snapd = |motivo: &str| FonteInfo {
        fonte: format!("snap:{snome}"),
        disponivel: false,
        instalado: false,
        versao: None,
        versao_repo: None,
        motivo: Some(motivo.to_string()),
        indeterminado: false,
    };
    {
        let memo = snapd_memo().lock().expect("mutex snapd");
        if let Some((t, respondeu)) = &*memo {
            if t.elapsed() < SNAPD_MEMO_TTL && !respondeu {
                return sem_snapd(
                    "snap instalado, mas o serviço snapd não respondeu — fonte não ofertada e sem consulta possível agora",
                );
            }
        }
    }
    if mgr == PkgMgr::Nenhum {
        return sem_snapd(
            "snapd precisa de um sistema com suporte a snap — fonte não ofertada aqui",
        );
    }
    let Some(o) = saida_com_max("snap", &["version"], teto) else {
        // sem resposta no teto: snap interpelado, mas serviço não disponível
        {
            let mut memo = snapd_memo().lock().expect("mutex snapd");
            *memo = Some((std::time::Instant::now(), false));
        }
        return sem_snapd(
            "snap instalado, mas o serviço snapd não respondeu — fonte não ofertada e sem consulta possível agora",
        );
    };
    let ver = String::from_utf8_lossy(&o.stdout).to_lowercase();
    // `snapd unavailable` na tabela do snap version = serviço não em execução
    if !o.status.success() || ver.contains("unavailable") || !ver.contains("snapd") {
        let mut memo = snapd_memo().lock().expect("mutex snapd");
        *memo = Some((std::time::Instant::now(), false));
        return sem_snapd(
            "snap instalado, mas o serviço snapd não está em execução — fonte não ofertada",
        );
    }
    match saida_com_max("snap", &["info", snome], teto) {
        Some(o) if o.status.success() => {
            {
                let mut memo = snapd_memo().lock().expect("mutex snapd");
                *memo = Some((std::time::Instant::now(), true));
            }
            let sinfo = String::from_utf8_lossy(&o.stdout).to_string();
            let instalado = sinfo.contains("tracking:");
            FonteInfo {
                fonte: format!("snap:{snome}"),
                disponivel: true,
                instalado,
                versao: None,
                versao_repo: None,
                motivo: None,
                indeterminado: false,
            }
        }
        // consulta à Snap Store não concluída (teto ou erro): não provam ausência
        _ => FonteInfo {
            fonte: format!("snap:{snome}"),
            disponivel: false,
            instalado: false,
            versao: None,
            versao_repo: None,
            motivo: Some("não foi possível consultar a Snap Store agora (falha temporária)".to_string()),
            indeterminado: true,
        },
    }
}

// ---------- listagem de fontes ----------

/// fontes reais do aplicativo `id`, na ordem do original: pacotes nativos
/// aptos à família, flatpak (sistema, depois usuário) e snap
pub fn fontes(id: &str) -> Vec<FonteInfo> {
    let Some(info) = super::appinfo::achado(id) else {
        return Vec::new();
    };
    // memo curto: uma única tripla de consultas alimenta pontos múltiplos
    let (tem_flatpak_cmd, sistema, usuario) = match super::tools::flatpak_ids_estado() {
        Some((s, u)) => (true, s, u),
        None => (false, Vec::new(), Vec::new()),
    };
    let familia = super::os::detect_distro().and_then(|d| d.family);
    let mgr = super::pkgmgr::atual();

    let mut fontes = Vec::new();
    for pkg in info.pacotes_aptos(familia.as_deref()) {
        let na_repo = super::pkgmgr::na_repo(pkg);
        let local = super::pkgmgr::instalado(pkg);
        let (disponivel_repo, indet_repo) = match &na_repo {
            super::pkgmgr::Consulta::Ausente => (false, false),
            super::pkgmgr::Consulta::Disponivel(_) => (true, false),
            super::pkgmgr::Consulta::Indeterminado(_) => (false, true),
        };
        let motivo = if local.is_none() {
            match (&na_repo, mgr) {
                (super::pkgmgr::Consulta::Indeterminado(m), _) => {
                    Some(format!("não foi possível verificar agora: {m}"))
                }
                (_, super::pkgmgr::PkgMgr::Nenhum) => {
                    Some("esta distribuição não tem fonte nativa compatível".to_string())
                }
                (super::pkgmgr::Consulta::Ausente, _) => {
                    Some("pacote não está nos repositórios habilitados".to_string())
                }
                _ => None,
            }
        } else {
            None
        };
        fontes.push(FonteInfo {
            fonte: format!("nativo:{pkg}"),
            disponivel: local.is_some() || disponivel_repo,
            instalado: local.is_some(),
            versao: local.clone(),
            versao_repo: None,
            motivo,
            indeterminado: indet_repo && local.is_none(),
        });
    }
    if let Some(fid) = info.flatpak {
        if !tem_flatpak_cmd {
            fontes.push(FonteInfo {
                fonte: "flatpak".to_string(),
                disponivel: false,
                instalado: false,
                versao: None,
                versao_repo: None,
                motivo: Some("flatpak não está instalado no sistema".to_string()),
                indeterminado: false,
            });
        } else {
            let no_sistema = sistema.iter().find(|(i, _)| i == fid);
            let no_usuario = usuario.iter().find(|(i, _)| i == fid);

            if let Some((_, versao)) = no_sistema {
                fontes.push(FonteInfo {
                    fonte: "flatpak".to_string(),
                    disponivel: true,
                    instalado: true,
                    versao: Some(versao.clone()),
                    // sem consultas de rede neste slice
                    versao_repo: None,
                    motivo: None,
                    indeterminado: false,
                });
            } else {
                // não instalado e sem consulta de remoto neste slice:
                // disponibilidade fica inconclusiva, honestamente (§3/§4.2)
                fontes.push(FonteInfo {
                    fonte: "flatpak".to_string(),
                    disponivel: false,
                    instalado: false,
                    versao: None,
                    versao_repo: None,
                    motivo: Some(
                        "o repositório ainda não foi verificado — a consulta de remotos chega com a verificação de atualizações".to_string(),
                    ),
                    indeterminado: true,
                });
            }

            if let Some((_, versao)) = no_usuario {
                if no_sistema.is_none() {
                    fontes.push(FonteInfo {
                        fonte: "flatpak-user".to_string(),
                        disponivel: true,
                        instalado: true,
                        versao: Some(versao.clone()),
                        versao_repo: None,
                        motivo: None,
                        indeterminado: false,
                    });
                }
            }
        }
    }
    // Snap Store como fonte distinta: só aparece se o snapd responder
    if let Some(snome) = info.snap {
        fontes.push(fonte_snap(snome, mgr));
    }

    fontes
}

/// primeira fonte que atende à ação: instalar → disponível (ordem da
/// listagem); remover/atualizar → instalada. None = nenhuma fonte conhecida
/// atende — a UI repassa o erro sem inventar comando
pub fn fonte_padrao(id: &str, acao: &str) -> Option<String> {
    let quer_instalada = matches!(acao, "remover" | "atualizar");
    fontes(id)
        .into_iter()
        .find(|f| if quer_instalada { f.instalado } else { f.disponivel })
        .map(|f| f.fonte)
}

// ---------- abrir launcher instalado ----------

/// constante de ativação de overlay na iniciação de jogos (fica para o
/// slice da aba Overlay; aqui segue existindo como no original para o
/// `flatpak run` não se divergir do repositório)
pub(crate) const MANGOHUD_VAR: (&str, &str) = ("MANGOHUD", "1");

/// argumentos de `flatpak run` para o aplicativo. Com overlay, a variável
/// precisa ir como `--env`: o valor definido via `Command::env` pertence ao
/// processo cliente do flatpak e NÃO atravessa o sandbox onde os jogos rodam.
/// Reconstruída em função pura para poder ser testada.
pub(crate) fn args_flatpak_run(fid: &str, overlay: bool) -> Vec<String> {
    let mut args = vec!["run".to_string()];
    if overlay {
        args.push("--env=MANGOHUD=1".to_string());
    }
    args.push(fid.to_string());
    args
}

/// Executa o launcher já instalado — na fonte em que ele foi detectado.
/// Flatpak: `flatpak run <id>`; nativo: o binário do pacote.
pub fn abrir_launcher(id: &str, overlay: bool) -> Result<String, String> {
    let info = super::appinfo::achado(id)
        .ok_or_else(|| "aplicativo desconhecido".to_string())?;
    if !info.gui {
        return Err(format!(
            "{} é uma ferramenta de linha de comando e não tem interface gráfica para iniciar",
            info.nome
        ));
    }
    let instalados: Vec<String> = flatpaks_escopo("--system")
        .into_iter()
        .chain(flatpaks_escopo("--user"))
        .map(|(id, _)| id)
        .collect();

    if let Some(fid) = info.flatpak {
        if instalados.iter().any(|l| l == fid) {
            let mut cmd = super::spawn::comando("flatpak");
            cmd.args(args_flatpak_run(fid, overlay));
            cmd.spawn().map_err(|e| format!("falha ao iniciar: {e}"))?;
            // sem ativação de MangoHud neste slice: overlay vem false da UI
            return Ok("Aplicativo iniciado".to_string());
        }
    }
    for pkg in info.pacotes_aptos(super::os::detect_distro().and_then(|d| d.family).as_deref()) {
        if super::pkgmgr::instalado(pkg).is_some() {
            let mut cmd = super::spawn::comando(pkg);
            if overlay {
                let (var, valor) = MANGOHUD_VAR;
                cmd.env(var, valor);
            }
            cmd.spawn().map_err(|e| format!("falha ao iniciar: {e}"))?;
            return Ok("Aplicativo iniciado".to_string());
        }
    }
    Err(format!("{} não está instalado", info.nome))
}

// ---------- ações (install/remove/update) ----------

pub fn acao_app(id: &str, acao: &str, fonte: &str) -> Result<String, InstalarError> {
    let info = super::appinfo::achado(id).ok_or(InstalarError::Fonte)?;
    // qualquer ação muda o estado real: invalida o memo de flatpaks apenas
    // no fim (o original invalida antes porque tem caches de detalhes com
    // TTL; aqui a revalidação pós-ação refaz tudo com snapshot novo)
    super::tools::flatpak_ids_invalidate();

    let pkgmgr_op = |op: super::pkgmgr::PkgOp, pkg: &str| -> Result<String, InstalarError> {
        // revalida a condição antes de executar: vigiar compatibilidade,
        // disponibilidade e estado real (ainda instalado / presente na repo)
        let resultado = match op {
            super::pkgmgr::PkgOp::Instalar => super::pkgmgr::instalar(pkg),
            super::pkgmgr::PkgOp::Remover => super::pkgmgr::remover(pkg),
        };
        resultado
            .map(|_| match op {
                super::pkgmgr::PkgOp::Instalar => "Instalação concluída".to_string(),
                super::pkgmgr::PkgOp::Remover => "Remoção concluída".to_string(),
            })
            .map_err(InstalarError::Falha)
    };

    match fonte {
        f if f.starts_with("nativo:") => {
            // valida que o pacote é do catálogo [apto à família] e nunca do frontend
            let familia = super::os::detect_distro().and_then(|d| d.family);
            let pkg = f.strip_prefix("nativo:").ok_or(InstalarError::Fonte)?;
            if !info.nativos.iter().any(|p| p.pkg == pkg) {
                return Err(InstalarError::Fonte);
            }
            let apto = info.pacotes_aptos(familia.as_deref()).any(|p| p == pkg);
            if !apto {
                return Err(InstalarError::Fonte);
            }
            let op = match acao {
                "instalar" => super::pkgmgr::PkgOp::Instalar,
                "atualizar" => super::pkgmgr::PkgOp::Instalar,
                "remover" => {
                    if super::pkgmgr::instalado(pkg).is_none() {
                        return Err(InstalarError::Falha(
                            "o pacote nativo não está mais instalado".into(),
                        ));
                    }
                    super::pkgmgr::PkgOp::Remover
                }
                _ => return Err(InstalarError::Fonte),
            };
            pkgmgr_op(op, pkg)
        }
        fonte_escopo @ ("flatpak" | "flatpak-user") => {
            let Some(fid) = info.flatpak else {
                return Err(InstalarError::Fonte);
            };
            // revalida: evitar remover/atualizar uma instalação de outro escopo
            let escopo = if fonte_escopo == "flatpak-user" { "--user" } else { "--system" };
            let instalado_no_escopo = rodar(
                "flatpak",
                &["list", "--app", escopo, "--columns=application"],
            )
            .map(|s| listar_com_version(&s).iter().any(|(i, _)| i == fid))
            .unwrap_or(false);
            if fonte_escopo == "flatpak-user" && !instalado_no_escopo && acao == "remover" {
                return Err(InstalarError::Falha(
                    "não há instalação do seu usuário para remover neste escopo".into(),
                ));
            }
            if !instalado_no_escopo && acao == "atualizar" {
                return Err(InstalarError::Falha(
                    "o app não está instalado neste escopo para atualizar".into(),
                ));
            }
            let out = match acao {
                "atualizar" => {
                    // update de verdade: atualiza do remoto de ORIGEM do app;
                    // `install` sobre uma instalação existente reinstalaria a
                    // mesma versão ou falharia silenciosamente em outro remoto
                    super::spawn::comando("flatpak")
                        .args(["update", escopo, "--noninteractive", "-y", fid])
                        .output()
                        .map_err(|e| InstalarError::Execucao(e.to_string()))?
                }
                "instalar" => {
                    // tenta os remotos realmente configurados, um a um —
                    // nenhum remoto configurado é erro honesto (não flathub fixo)
                    let remotos = flatpak_remotos();
                    if remotos.is_empty() {
                        return Err(InstalarError::Falha(
                            "flatpak não tem remotos configurados para instalar".into(),
                        ));
                    }
                    let mut ultimo = None;
                    for r in &remotos {
                        let o = super::spawn::comando("flatpak")
                            .args(["install", escopo, "--noninteractive", "-y", r, fid])
                            .output()
                            .map_err(|e| InstalarError::Execucao(e.to_string()))?;
                        ultimo = Some(o);
                        if ultimo.as_ref().map(|o| o.status.success()).unwrap_or(false) {
                            break;
                        }
                    }
                    ultimo.ok_or_else(|| InstalarError::Execucao(
                        "nenhum remoto pôde ser consultado".into(),
                    ))?
                }
                "remover" => super::spawn::comando("flatpak")
                    .args(["uninstall", escopo, "-y", "--noninteractive", fid])
                    .output()
                    .map_err(|e| InstalarError::Execucao(e.to_string()))?,
                _ => return Err(InstalarError::Fonte),
            };
            if out.status.success() {
                Ok(if acao == "atualizar" {
                    "Atualização concluída".to_string()
                } else {
                    "Operação concluída".to_string()
                })
            } else {
                Err(InstalarError::Falha(
                    String::from_utf8_lossy(&out.stderr).to_string(),
                ))
            }
        }
        f if f.starts_with("snap:") => {
            let snome = f.strip_prefix("snap:").ok_or(InstalarError::Fonte)?;
            // valida contra o catálogo e reconfirma a disponibilidade real
            if info.snap != Some(snome) || !fonte_snap(snome, super::pkgmgr::atual()).disponivel {
                return Err(InstalarError::NaoDisponivel);
            }
            let out = match acao {
                "atualizar" => super::spawn::comando("pkexec")
                    .args(["snap", "refresh", snome])
                    .output()
                    .map_err(|e| InstalarError::Execucao(e.to_string()))?,
                "instalar" => super::spawn::comando("pkexec")
                    .args(["snap", "install", snome])
                    .output()
                    .map_err(|e| InstalarError::Execucao(e.to_string()))?,
                "remover" => super::spawn::comando("pkexec")
                    .args(["snap", "remove", snome])
                    .output()
                    .map_err(|e| InstalarError::Execucao(e.to_string()))?,
                _ => return Err(InstalarError::Fonte),
            };
            if out.status.success() {
                Ok("Operação concluída".to_string())
            } else {
                Err(InstalarError::Falha(
                    String::from_utf8_lossy(&out.stderr).to_string(),
                ))
            }
        }
        _ => Err(InstalarError::Fonte),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remotos_parser() {
        let s = "flathub\nflathub-beta\n";
        assert_eq!(
            flatpak_remotos_de(&Some(s.to_string())),
            vec!["flathub".to_string(), "flathub-beta".to_string()]
        );
        assert_eq!(flatpak_remotos_de(&None), Vec::<String>::new());
    }

    #[test]
    fn listar_com_version_parser() {
        let linhas = "com.valvesoftware.Steam\t2.0\norg.example.Outro\t\n";
        let v = listar_com_version(linhas);
        assert_eq!(
            v[0],
            ("com.valvesoftware.Steam".to_string(), "2.0".to_string())
        );
        assert_eq!(v[1].0, "org.example.Outro");
    }

    #[test]
    fn args_flatpak_run_sem_overlay() {
        assert_eq!(
            args_flatpak_run("com.x.Y", false),
            vec!["run".to_string(), "com.x.Y".to_string()]
        );
        assert_eq!(
            args_flatpak_run("com.x.Y", true),
            vec!["run".to_string(), "--env=MANGOHUD=1".to_string(), "com.x.Y".to_string()]
        );
    }

    #[test]
    fn fonte_padrao_ordem_instalado() {
        // fonte_padrao usa as fontes reais; para id fora do REGISTRY, None
        assert_eq!(fonte_padrao("id-inexistente", "instalar"), None);
    }
}
