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
        let Some(nome) = nome_do_executavel(caminho) else {
            return false;
        };
        self.0.read().map(|s| s.contains(&nome)).unwrap_or(false)
    }
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
