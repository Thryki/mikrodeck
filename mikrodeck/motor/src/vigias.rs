//! Descobre o que está aberto no Windows, para o pad mudar de cor sozinho.
//!
//! Roda numa thread própria, com uma varredura por vez. Nada aqui pode bloquear
//! o caminho pad -> LED, então o resto do motor só lê o resultado da última
//! varredura.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

/// Intervalo entre duas varreduras. Programa abrindo é coisa de segundo, não de
/// milissegundo, e a varredura custa uma passada por toda a tabela de processos.
const INTERVALO: Duration = Duration::from_millis(1500);

/// Nomes de executável abertos agora, em minúsculas e com extensão.
#[derive(Clone, Default)]
pub struct Abertos(Arc<RwLock<HashSet<String>>>);

impl Abertos {
    /// Sobe a thread de varredura e devolve a vista compartilhada.
    pub fn vigiar() -> Self {
        let vista = Abertos(Arc::new(RwLock::new(HashSet::new())));
        let alvo = Arc::clone(&vista.0);
        thread::Builder::new()
            .name("mikrodeck-vigias".into())
            .spawn(move || loop {
                let agora = processos_abertos();
                if let Ok(mut w) = alvo.write() {
                    *w = agora;
                }
                thread::sleep(INTERVALO);
            })
            .expect("subir a thread de vigias");
        vista
    }

    /// Diz se o programa daquele caminho está aberto. Compara só o nome do
    /// arquivo: o caminho configurado pode ser um atalho, e o processo que sobe
    /// tem outro caminho.
    pub fn tem(&self, caminho: &str) -> bool {
        let pistas = pistas_de_processo(caminho);
        if pistas.is_empty() {
            return false;
        }
        self.0
            .read()
            .map(|abertos| pistas.iter().any(|n| abertos.contains(n)))
            .unwrap_or(false)
    }
}

/// Nomes de executável que este caminho pode ter virado, em minúsculas e com
/// `.exe`.
///
/// Caminho comum dá uma pista só, que é o próprio nome do arquivo. App do menu
/// Iniciar é outra história: o identificador
/// `shell:appsFolder\Raycast.Raycast_qypenmj9wpt2a!Raycast` não contém o nome
/// do processo em lugar nenhum óbvio, e sem isso o "cuidar da janela" não acha
/// a janela para minimizar nem para fechar. Daí duas pistas: o que vem depois
/// do `!`, que costuma ser o nome do programa, e o último pedaço do nome do
/// pacote, que costuma ser parecido com ele.
pub fn pistas_de_processo(caminho: &str) -> Vec<String> {
    let limpo = caminho.trim().trim_matches('"');
    if !crate::apps::e_da_loja(limpo) {
        return nome_do_executavel(limpo).into_iter().collect();
    }
    let id = limpo
        .split_once(crate::apps::PREFIXO_LOJA)
        .map(|(_, resto)| resto)
        .unwrap_or(limpo)
        .trim();
    // Muito item do menu Iniciar e atalho para um arquivo, e o identificador
    // dele carrega o caminho: `{GUID}\7-Zip\7zFM.exe`. Ai o nome do processo
    // esta ali na cara, e nao ha o que adivinhar.
    if id.contains('\\') || id.to_lowercase().ends_with(".exe") {
        return nome_do_executavel(id).into_iter().collect();
    }
    let (pacote, apelido) = match id.split_once('!') {
        Some((p, a)) => (p, Some(a)),
        None => (id, None),
    };
    // `Raycast.Raycast_qypenmj9wpt2a` vira `raycast`; `Brave` continua `brave`.
    let do_pacote = pacote
        .split_once('_')
        .map(|(antes, _)| antes)
        .unwrap_or(pacote)
        .rsplit('.')
        .next()
        .unwrap_or(pacote);
    let mut pistas = Vec::new();
    for bruta in [apelido.unwrap_or(""), do_pacote] {
        let nome = bruta.trim().to_lowercase();
        // "app" e apelido generico de meio mundo: como pista, casaria com
        // qualquer coisa e traria a janela errada.
        if nome.is_empty() || nome == "app" {
            continue;
        }
        let com_exe = if nome.ends_with(".exe") {
            nome
        } else {
            format!("{nome}.exe")
        };
        if !pistas.contains(&com_exe) {
            pistas.push(com_exe);
        }
    }
    pistas
}

/// Nome do executável a partir de um caminho de configuração, em minúsculas e
/// sempre com `.exe`. Aceita caminho cheio, nome curto e atalho.
pub(crate) fn nome_do_executavel(caminho: &str) -> Option<String> {
    let base = caminho
        .trim()
        .trim_matches('"')
        .rsplit(['\\', '/'])
        .next()?
        .trim();
    if base.is_empty() {
        return None;
    }
    let base = base.to_lowercase();
    // Atalho aponta para um programa, mas o processo tem o nome do programa.
    // Sem saber para onde ele aponta, o melhor palpite é o nome do próprio atalho.
    let sem_extensao = match base.rsplit_once('.') {
        Some((antes, "lnk" | "url" | "exe")) => antes.to_string(),
        _ => base.clone(),
    };
    if sem_extensao.is_empty() {
        return None;
    }
    Some(format!("{sem_extensao}.exe"))
}

#[cfg(windows)]
fn processos_abertos() -> HashSet<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut nomes = HashSet::new();
    unsafe {
        let foto = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if foto == INVALID_HANDLE_VALUE {
            return nomes;
        }
        let mut entrada: PROCESSENTRY32W = std::mem::zeroed();
        entrada.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(foto, &mut entrada) != 0 {
            loop {
                let fim = entrada
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entrada.szExeFile.len());
                let nome = String::from_utf16_lossy(&entrada.szExeFile[..fim]);
                if !nome.is_empty() {
                    nomes.insert(nome.to_lowercase());
                }
                if Process32NextW(foto, &mut entrada) == 0 {
                    break;
                }
            }
        }
        CloseHandle(foto);
    }
    nomes
}

#[cfg(not(windows))]
fn processos_abertos() -> HashSet<String> {
    HashSet::new()
}

#[cfg(test)]
mod testes {
    #[test]
    fn app_do_menu_iniciar_da_pistas_de_processo() {
        // O identificador nao tem o nome do processo em lugar nenhum obvio, e
        // sem estas pistas o "cuidar da janela" nao acha a janela do Raycast.
        let p = pistas_de_processo(
            r"shell:appsFolder\Raycast.Raycast_qypenmj9wpt2a!Raycast",
        );
        assert!(p.contains(&"raycast.exe".to_string()), "{p:?}");
    }

    #[test]
    fn app_sem_apelido_usa_o_nome_do_pacote() {
        let p = pistas_de_processo(r"shell:appsFolder\Chrome");
        assert_eq!(p, vec!["chrome.exe".to_string()]);
    }

    #[test]
    fn pacote_com_dominio_fica_com_o_ultimo_pedaco() {
        let p = pistas_de_processo(
            r"shell:appsFolder\SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify",
        );
        assert!(p.contains(&"spotify.exe".to_string()), "{p:?}");
        assert!(p.contains(&"spotifymusic.exe".to_string()), "{p:?}");
    }

    #[test]
    fn atalho_com_caminho_dentro_do_identificador_usa_o_arquivo() {
        // Muito item do menu Iniciar e atalho, e o identificador carrega o
        // caminho. Sem tratar isso, a pista virava "exe.exe".
        let p = pistas_de_processo(
            r"shell:appsFolder\{6D809377-6AF0-444B-8957-A3773F02200E}\7-Zip\7zFM.exe",
        );
        assert_eq!(p, vec!["7zfm.exe".to_string()]);
    }

    #[test]
    fn apelido_generico_nao_vira_pista() {
        // "App" e apelido de meio mundo: como pista, traria a janela errada.
        let p = pistas_de_processo(r"shell:appsFolder\OpenAI.Codex_2p2nqsd0c76g0!App");
        assert!(!p.contains(&"app.exe".to_string()), "{p:?}");
        assert_eq!(p, vec!["codex.exe".to_string()]);
    }

    #[test]
    fn caminho_comum_continua_dando_uma_pista_so() {
        assert_eq!(
            pistas_de_processo(r"C:\Program Files\Notepad++\notepad++.exe"),
            vec!["notepad++.exe".to_string()]
        );
        assert_eq!(pistas_de_processo("spotify.exe"), vec!["spotify.exe".to_string()]);
        assert!(pistas_de_processo("").is_empty());
    }

    use super::*;

    #[test]
    fn tira_o_nome_do_executavel_de_qualquer_forma_de_caminho() {
        assert_eq!(
            nome_do_executavel("C:\\Program Files\\Google\\Chrome\\chrome.exe"),
            Some("chrome.exe".into())
        );
        assert_eq!(nome_do_executavel("chrome"), Some("chrome.exe".into()));
        assert_eq!(nome_do_executavel("Notepad.EXE"), Some("notepad.exe".into()));
        assert_eq!(
            nome_do_executavel("C:\\Menu\\Spotify.lnk"),
            Some("spotify.exe".into())
        );
        assert_eq!(nome_do_executavel("  "), None);
    }

    #[test]
    fn sem_varredura_nada_esta_aberto() {
        let a = Abertos::default();
        assert!(!a.tem("chrome.exe"));
    }

    #[test]
    fn a_varredura_de_verdade_acha_o_proprio_processo() {
        // Se a chamada do Windows quebrar, este teste avisa.
        let abertos = processos_abertos();
        if cfg!(windows) {
            assert!(!abertos.is_empty(), "varredura voltou vazia");
        }
    }
}
