//! Ponte entre o motor e a interface.
//!
//! A UI nunca fala com o HID. Ela chama os comandos daqui e escuta os eventos
//! que o motor emite. Toda a lógica de aparelho fica no crate `motor`.

use motor::config::Config;
use motor::servico::{Aviso, Servico, Situacao};
use serde::Serialize;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

/// O motor vivo, guardado no estado do Tauri.
struct MotorVivo(Mutex<Option<Servico>>);

/// A gravação em curso, se houver. Uma por vez: duas disputariam o microfone.
struct GravacaoVivo(Mutex<Option<motor::som::gravador::Gravacao>>);

/// Situação do aparelho, no formato que a UI entende.
#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
enum SituacaoUi {
    Procurando,
    Conectado,
}

impl From<Situacao> for SituacaoUi {
    fn from(s: Situacao) -> Self {
        match s {
            Situacao::Procurando => SituacaoUi::Procurando,
            Situacao::Conectado => SituacaoUi::Conectado,
        }
    }
}

#[derive(Serialize, Clone)]
struct PadAoVivo {
    pad: u8,
    apertado: bool,
}

#[derive(Serialize, Clone)]
struct PaginaAtual {
    numero: usize,
    nome: String,
}

/// Devolve a config em uso, para a UI desenhar as páginas e os pads.
#[tauri::command]
fn ler_config(motor: State<'_, MotorVivo>) -> Result<Config, String> {
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    guarda
        .as_ref()
        .map(|s| s.config())
        .ok_or_else(|| "motor não iniciado".to_string())
}

/// Salva a config no disco e aplica no motor na hora, sem reiniciar nada.
#[tauri::command]
fn salvar_config(config: Config, motor: State<'_, MotorVivo>) -> Result<(), String> {
    let caminho = Config::caminho_padrao();
    config.salvar(&caminho).map_err(|e| e.to_string())?;
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    if let Some(s) = guarda.as_ref() {
        s.aplicar_config(config);
    }
    Ok(())
}

/// Situação do aparelho agora, para a barra de status.
#[tauri::command]
fn ler_situacao(motor: State<'_, MotorVivo>) -> Result<SituacaoUi, String> {
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    Ok(guarda
        .as_ref()
        .map(|s| s.situacao().into())
        .unwrap_or(SituacaoUi::Procurando))
}

/// Página aberta agora no aparelho.
#[tauri::command]
fn ler_pagina(motor: State<'_, MotorVivo>) -> Result<PaginaAtual, String> {
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    let (numero, nome) = guarda
        .as_ref()
        .map(|s| s.pagina_atual())
        .unwrap_or((1, "—".into()));
    Ok(PaginaAtual { numero, nome })
}

/// Troca a página a partir da interface. O aparelho acompanha na hora.
#[tauri::command]
fn ir_para_pagina(numero: usize, motor: State<'_, MotorVivo>) -> Result<(), String> {
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    if let Some(s) = guarda.as_ref() {
        s.ir_para_pagina(numero);
    }
    Ok(())
}

#[derive(Serialize, Clone)]
struct BotaoAoVivo {
    nome: String,
    apertado: bool,
}

#[derive(Serialize)]
struct Diagnostico {
    aparelho_conectado: bool,
    caminho_config: String,
    paginas: usize,
    pads_configurados: usize,
    botoes_configurados: usize,
    versao_motor: String,
}

/// Números para a tela de diagnóstico, quando algo não está funcionando.
#[tauri::command]
fn diagnostico(motor: State<'_, MotorVivo>) -> Result<Diagnostico, String> {
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    let servico = guarda.as_ref().ok_or("motor não iniciado")?;
    let config = servico.config();
    Ok(Diagnostico {
        aparelho_conectado: servico.situacao() == Situacao::Conectado,
        caminho_config: Config::caminho_padrao().display().to_string(),
        paginas: config.paginas.len(),
        pads_configurados: config.paginas.iter().map(|p| p.pads.len()).sum(),
        botoes_configurados: config.paginas.iter().map(|p| p.botoes.len()).sum(),
        versao_motor: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Volta para a configuração de exemplo. A antiga vira `config.json.bak`.
#[tauri::command]
fn restaurar_padrao(motor: State<'_, MotorVivo>) -> Result<Config, String> {
    let caminho = Config::caminho_padrao();
    if caminho.exists() {
        let _ = std::fs::rename(&caminho, caminho.with_extension("json.bak"));
    }
    let nova = Config::exemplo();
    nova.salvar(&caminho).map_err(|e| e.to_string())?;
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    if let Some(s) = guarda.as_ref() {
        s.aplicar_config(nova.clone());
    }
    Ok(nova)
}

/// Abre a pasta da config no explorador de arquivos.
#[tauri::command]
fn abrir_pasta_config() -> Result<(), String> {
    let caminho = Config::caminho_padrao();
    let pasta = caminho.parent().ok_or("sem pasta")?;
    std::process::Command::new("explorer")
        .arg(pasta)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Acende tudo por um instante, para conferir que o aparelho responde.
#[tauri::command]
fn testar_leds(motor: State<'_, MotorVivo>) -> Result<(), String> {
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    let servico = guarda.as_ref().ok_or("motor não iniciado")?;
    servico.testar_leds();
    Ok(())
}

/// Força o descanso no aparelho agora, para ver a luz dos pads sem esperar.
#[tauri::command]
fn previsualizar_descanso(motor: State<'_, MotorVivo>) -> Result<(), String> {
    let guarda = motor.0.lock().map_err(|e| e.to_string())?;
    let servico = guarda.as_ref().ok_or("motor não iniciado")?;
    servico.previsualizar_descanso();
    Ok(())
}

/// Microfones disponíveis, para o seletor da gravação.
#[tauri::command]
fn microfones() -> Vec<motor::som::gravador::Microfone> {
    motor::som::gravador::microfones()
}

/// Começa a gravar um sample. O arquivo nasce em `~/.mikrodeck/samples`.
///
/// Só uma gravação por vez: gravar em dois pads ao mesmo tempo não faz sentido
/// e disputaria o microfone.
#[tauri::command]
fn gravar_sample(
    nome: String,
    microfone: Option<String>,
    segundos: u32,
    gravacao: State<'_, GravacaoVivo>,
) -> Result<String, String> {
    let mut guarda = gravacao.0.lock().map_err(|e| e.to_string())?;
    if guarda.is_some() {
        return Err("já tem uma gravação em curso".into());
    }
    // O nome vem da interface: fica só o que dá nome de arquivo, para uma barra
    // ou dois pontos não escreverem fora da pasta.
    let limpo: String = nome
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let limpo = limpo.trim_matches('-').to_string();
    let limpo = if limpo.is_empty() { "sample".to_string() } else { limpo };
    let caminho = motor::som::pasta_dos_samples().join(format!("{limpo}.wav"));
    let g = motor::som::gravador::Gravacao::comecar(microfone.as_deref(), &caminho, segundos)?;
    *guarda = Some(g);
    Ok(caminho.to_string_lossy().to_string())
}

/// Para a gravação e devolve o caminho do arquivo.
#[tauri::command]
fn parar_gravacao(gravacao: State<'_, GravacaoVivo>) -> Result<String, String> {
    let mut guarda = gravacao.0.lock().map_err(|e| e.to_string())?;
    let g = guarda.take().ok_or("não tem gravação em curso")?;
    let caminho = g.parar()?;
    Ok(caminho.to_string_lossy().to_string())
}

/// Quanto já foi gravado, em segundos. Zero quando não há gravação.
#[tauri::command]
fn tempo_de_gravacao(gravacao: State<'_, GravacaoVivo>) -> f32 {
    gravacao
        .0
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|g| g.duracao().as_secs_f32()))
        .unwrap_or(0.0)
}

/// Toca um sample aqui no computador, para a pessoa conferir antes de salvar.
#[tauri::command]
fn testar_sample(caminho: String, volume: f32) -> Result<f32, String> {
    let saida = motor::som::Saida::nova();
    let duracao = saida
        .carregar(std::path::Path::new(&caminho))?
        .duracao()
        .as_secs_f32();
    let voz = saida.tocar(
        std::path::Path::new(&caminho),
        volume,
        motor::som::envelope::Envelope::default(),
    )?;
    // A saída morre no fim desta função e levaria o som junto, então espera o
    // sample acabar. O teto de 30 s impede de travar a interface num arquivo longo.
    let limite = std::time::Instant::now();
    while !voz.acabou() && limite.elapsed().as_secs() < 30 {
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    Ok(duracao)
}

/// Páginas prontas que a pessoa pode adicionar com um clique.
#[tauri::command]
fn paginas_prontas() -> Vec<motor::prontas::Pronta> {
    motor::prontas::catalogo()
}

/// Adiciona uma página pronta no fim da lista e devolve a config nova.
/// Nunca sobrescreve o que já existe.
#[tauri::command]
fn adicionar_pagina_pronta(id: String, motor_vivo: State<'_, MotorVivo>) -> Result<Config, String> {
    let pagina = motor::prontas::montar(&id).ok_or_else(|| format!("página \"{id}\" não existe"))?;
    let caminho = Config::caminho_padrao();
    let mut config = Config::carregar_ou_criar(&caminho).map_err(|e| e.to_string())?;
    config.paginas.push(pagina);
    config.salvar(&caminho).map_err(|e| e.to_string())?;
    let guarda = motor_vivo.0.lock().map_err(|e| e.to_string())?;
    if let Some(s) = guarda.as_ref() {
        s.aplicar_config(config.clone());
    }
    Ok(config)
}

/// Diz se o MikroDeck está registrado para subir junto com o Windows.
#[tauri::command]
fn ler_inicia_com_o_sistema(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Liga ou desliga a subida automática com o Windows.
#[tauri::command]
fn definir_inicia_com_o_sistema(app: AppHandle, ligado: bool) -> Result<(), String> {
    let auto = app.autolaunch();
    if ligado {
        auto.enable().map_err(|e| e.to_string())
    } else {
        auto.disable().map_err(|e| e.to_string())
    }
}

/// Fica de olho no arquivo de config e aplica sozinho quando ele muda por fora.
///
/// É o que permite editar a configuração sem passar pela interface: pelo servidor
/// MCP, por outro programa, ou na mão. Compara o conteúdo, não a data, para uma
/// gravação que não mudou nada não virar recarga.
fn vigiar_arquivo_de_config(app: AppHandle) {
    std::thread::Builder::new()
        .name("mikrodeck-vigia-config".into())
        .spawn(move || {
            let caminho = Config::caminho_padrao();
            let mut ultimo = std::fs::read_to_string(&caminho).unwrap_or_default();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                let Ok(agora) = std::fs::read_to_string(&caminho) else {
                    continue;
                };
                if agora == ultimo {
                    continue;
                }
                ultimo = agora.clone();
                let Ok(config) = serde_json::from_str::<Config>(&agora) else {
                    eprintln!("config mudou mas não deu para ler; ignorando");
                    continue;
                };
                let estado: State<'_, MotorVivo> = app.state();
                if let Ok(guarda) = estado.0.lock() {
                    if let Some(s) = guarda.as_ref() {
                        s.aplicar_config(config);
                    }
                }
                let _ = app.emit("config-mudou", ());
            }
        })
        .expect("subir a vigia da config");
}

/// Onde está o servidor MCP, para a pessoa copiar o comando de ligação.
///
/// Instalado, ele vem junto nos recursos do app. Rodando do código, fica no
/// `target` do próprio workspace.
#[tauri::command]
fn caminho_do_mcp(app: AppHandle) -> Option<String> {
    let mut candidatos = Vec::new();
    if let Ok(recursos) = app.path().resource_dir() {
        candidatos.push(recursos.join("mikrodeck-mcp.exe"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(pasta) = exe.parent() {
            candidatos.push(pasta.join("mikrodeck-mcp.exe"));
            // Rodando do código: mcp/target/debug fica ao lado de app/src-tauri/target.
            candidatos.push(
                pasta.join("../../../../mcp/target/debug/mikrodeck-mcp.exe"),
            );
            candidatos.push(
                pasta.join("../../../../mcp/target/release/mikrodeck-mcp.exe"),
            );
        }
    }
    candidatos
        .into_iter()
        .find(|c| c.exists())
        .and_then(|c| c.canonicalize().ok())
        .map(|c| {
            // O canonicalize do Windows devolve o caminho com um prefixo de
            // namespace que confunde quem lê e quebra o comando colado. Tira ele.
            let texto = c.display().to_string();
            texto
                .strip_prefix(r#"\\?\"#)
                .unwrap_or(&texto)
                .to_string()
        })
}

/// Ícone na bandeja do sistema. É ele que mantém o MikroDeck vivo com a janela
/// fechada, e é por ele que se encerra o programa de verdade.
fn montar_bandeja(app: &AppHandle) -> tauri::Result<()> {
    let abrir = MenuItem::with_id(app, "abrir", "Abrir o MikroDeck", true, None::<&str>)?;
    let sair = MenuItem::with_id(app, "sair", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&abrir, &sair])?;

    TrayIconBuilder::with_id("bandeja")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("MikroDeck")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, evento| match evento.id().as_ref() {
            "abrir" => mostrar_janela(app),
            "sair" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|bandeja, evento| {
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = evento
            {
                mostrar_janela(bandeja.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn mostrar_janela(app: &AppHandle) {
    if let Some(janela) = app.get_webview_window("main") {
        let _ = janela.show();
        let _ = janela.unminimize();
        let _ = janela.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(MotorVivo(Mutex::new(None)))
        .manage(GravacaoVivo(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            ler_config,
            salvar_config,
            ler_situacao,
            ler_pagina,
            ir_para_pagina,
            diagnostico,
            restaurar_padrao,
            abrir_pasta_config,
            testar_leds,
            previsualizar_descanso,
            ler_inicia_com_o_sistema,
            definir_inicia_com_o_sistema,
            caminho_do_mcp,
            paginas_prontas,
            adicionar_pagina_pronta,
            microfones,
            gravar_sample,
            parar_gravacao,
            tempo_de_gravacao,
            testar_sample
        ])
        .setup(|app| {
            let caminho = Config::caminho_padrao();
            let config = Config::carregar_ou_criar(&caminho)?;

            // O motor avisa a UI de tudo que acontece no aparelho.
            let handle: AppHandle = app.handle().clone();
            let servico = Servico::iniciar(config, move |aviso| {
                let _ = match aviso {
                    Aviso::Situacao(s) => {
                        handle.emit("situacao", SituacaoUi::from(s))
                    }
                    Aviso::Pad { pad, apertado } => {
                        handle.emit("pad", PadAoVivo { pad, apertado })
                    }
                    Aviso::Botao { nome, apertado } => {
                        handle.emit("botao", BotaoAoVivo { nome, apertado })
                    }
                    Aviso::Pagina { numero, nome } => {
                        handle.emit("pagina", PaginaAtual { numero, nome })
                    }
                    Aviso::Pausado(pausado) => handle.emit("pausado", pausado),
                    Aviso::Strip { posicao } => handle.emit("strip", posicao),
                };
            });

            let estado: State<'_, MotorVivo> = app.state();
            *estado.0.lock().unwrap() = Some(servico);

            montar_bandeja(app.handle())?;
            vigiar_arquivo_de_config(app.handle().clone());
            Ok(())
        })
        .on_window_event(|janela, evento| {
            // Fechar a janela esconde, não encerra: o motor precisa continuar
            // vivo para os pads funcionarem. Quem encerra é o menu da bandeja.
            if let WindowEvent::CloseRequested { api, .. } = evento {
                api.prevent_close();
                let _ = janela.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("erro ao rodar o MikroDeck");
}
