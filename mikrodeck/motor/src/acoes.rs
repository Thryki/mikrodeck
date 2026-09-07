//! O que um controle faz quando é acionado.
//!
//! As ações rodam numa thread própria. O caminho crítico do motor é
//! pad -> hid -> estado, e ele nunca pode ficar esperando um programa abrir.

use crate::rede::{self, HomeAssistant, Metodo};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, RwLock};
use std::thread;

/// Volume padrao de um sample: cheio.
fn volume_cheio() -> f32 {
    1.0
}

/// Uma ação que o motor sabe executar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Acao {
    /// Não faz nada. Serve para pad decorativo ou ainda não configurado.
    Nenhuma,
    /// Abre um programa.
    AbrirPrograma {
        caminho: String,
        #[serde(default)]
        argumentos: Vec<String>,
    },
    /// Abre uma URL no navegador padrão.
    AbrirUrl { url: String },
    /// Roda uma linha de comando no shell.
    Comando { linha: String },
    /// Dispara um atalho de teclado, por exemplo "ctrl+shift+n".
    Atalho { teclas: String },
    /// Controle de mídia do sistema.
    Midia { tecla: TeclaMidia },
    /// Vai para a próxima página.
    ProximaPagina,
    /// Volta para a página anterior.
    PaginaAnterior,
    /// Vai direto para uma página, contando de 1.
    IrParaPagina { numero: usize },
    /// Liga e desliga o MikroDeck sem fechar o programa. Pausado, os LEDs apagam
    /// e nenhum pad executa ação, para o aparelho voltar a ser um Maschine comum.
    PausarRetomar,
    /// Chama um serviço do Home Assistant, por exemplo `light.toggle` na entidade
    /// `light.sala`. Endereço e token ficam na config geral.
    HomeAssistant {
        servico: String,
        #[serde(default)]
        entidade: String,
    },
    /// Toca um sample de áudio. O disparo não passa pela thread de ações: o
    /// serviço trata direto, porque soltar o pad precisa alcançar a mesma voz
    /// que o aperto começou.
    Sample {
        caminho: String,
        #[serde(default)]
        modo: crate::som::ModoDisparo,
        #[serde(default = "volume_cheio")]
        volume: f32,
        #[serde(default)]
        envelope: crate::som::envelope::Envelope,
    },
    /// Requisição HTTP crua. Cobre webhook do Home Assistant e qualquer outro
    /// serviço da casa que aceite uma chamada.
    Http {
        url: String,
        #[serde(default)]
        metodo: Metodo,
        #[serde(default)]
        cabecalhos: BTreeMap<String, String>,
        #[serde(default)]
        corpo: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeclaMidia {
    TocarPausar,
    Proxima,
    Anterior,
    Parar,
    AumentarVolume,
    DiminuirVolume,
    Mudo,
}

/// Ações que o executor não trata sozinho, porque mexem no estado do motor.
#[derive(Debug, Clone, PartialEq)]
pub enum EfeitoNoEstado {
    ProximaPagina,
    PaginaAnterior,
    IrParaPagina(usize),
    PausarRetomar,
}

impl Acao {
    /// Se a ação muda a página, devolve qual efeito. O executor não dá conta disso
    /// porque ele roda em outra thread e não enxerga o estado.
    pub fn efeito_no_estado(&self) -> Option<EfeitoNoEstado> {
        match self {
            Acao::ProximaPagina => Some(EfeitoNoEstado::ProximaPagina),
            Acao::PaginaAnterior => Some(EfeitoNoEstado::PaginaAnterior),
            Acao::IrParaPagina { numero } => Some(EfeitoNoEstado::IrParaPagina(*numero)),
            Acao::PausarRetomar => Some(EfeitoNoEstado::PausarRetomar),
            _ => None,
        }
    }
}

/// Fila de ações rodando numa thread separada.
pub struct Executor {
    envia: Sender<Acao>,
    /// A ligação com o Home Assistant vive aqui porque a thread de ações precisa
    /// dela, e ela muda quando a pessoa salva a config.
    casa: Arc<RwLock<HomeAssistant>>,
}

impl Executor {
    pub fn novo() -> Self {
        let (envia, recebe) = mpsc::channel::<Acao>();
        let casa = Arc::new(RwLock::new(HomeAssistant::default()));
        let casa_thread = Arc::clone(&casa);
        thread::Builder::new()
            .name("mikrodeck-acoes".into())
            .spawn(move || {
                for acao in recebe {
                    let ligacao = casa_thread
                        .read()
                        .map(|c| c.clone())
                        .unwrap_or_default();
                    if let Err(e) = executar(&acao, &ligacao) {
                        eprintln!("ação falhou ({acao:?}): {e}");
                    }
                }
            })
            .expect("subir a thread de ações");
        Self { envia, casa }
    }

    /// Enfileira uma ação. Nunca bloqueia.
    pub fn disparar(&self, acao: Acao) {
        let _ = self.envia.send(acao);
    }

    /// Atualiza a ligação com o Home Assistant depois que a config muda.
    pub fn definir_home_assistant(&self, ligacao: HomeAssistant) {
        if let Ok(mut alvo) = self.casa.write() {
            *alvo = ligacao;
        }
    }
}

fn executar(acao: &Acao, casa: &HomeAssistant) -> std::io::Result<()> {
    use std::process::Command;
    match acao {
        Acao::Nenhuma => Ok(()),
        // O sample nao passa por aqui: quem toca e o servico, que e o unico que
        // sabe de qual pad veio o aperto e para onde mandar o soltar.
        Acao::Sample { .. } => Ok(()),
        Acao::AbrirPrograma {
            caminho,
            argumentos,
        } => {
            // App do menu Iniciar não tem executável para chamar: quem abre é
            // o Explorer, pelo identificador do app. É assim que app da
            // Microsoft Store abre, já que o .exe dele mora numa pasta
            // protegida que nem sempre aceita ser chamada direto.
            if crate::apps::e_da_loja(caminho) {
                return Command::new("explorer.exe")
                    .arg(caminho.trim())
                    .spawn()
                    .map(|_| ());
            }
            // Atalho do Windows não é executável: o CreateProcess recusa com
            // "não é um aplicativo Win32 válido". Quem sabe abrir atalho é o shell.
            if precisa_do_shell(caminho) {
                return abrir_pelo_shell(caminho, argumentos);
            }
            match Command::new(caminho).args(argumentos).spawn() {
                Ok(_) => Ok(()),
                // Duas coisas caem aqui e as duas o shell resolve:
                // nomes curtos como "chrome", que o Windows resolve pela chave de
                // registro "App Paths" que o CreateProcess não consulta; e qualquer
                // arquivo que dependa de associação de tipo.
                Err(_) => abrir_pelo_shell(caminho, argumentos),
            }
        }
        Acao::AbrirUrl { url } => {
            // Pelo Explorer, e não pelo `cmd /C start`.
            //
            // O `cmd` trata `&` como separador de comando, e o `Command` do
            // Rust só põe aspas em argumento que tenha espaço. Resultado: uma
            // URL comum de YouTube, `...?v=abc&list=xyz`, abria só até o `&` e
            // o resto virava comando do shell. O Explorer recebe o argumento
            // inteiro e não reparseia nada.
            Command::new("explorer.exe")
                .arg(completar_url(url))
                .spawn()?;
            Ok(())
        }
        Acao::Comando { linha } => {
            Command::new("cmd").args(["/C", linha]).spawn()?;
            Ok(())
        }
        Acao::Atalho { teclas } => {
            teclado::mandar_atalho(teclas);
            Ok(())
        }
        Acao::Midia { tecla } => {
            teclado::mandar_tecla_virtual(tecla.codigo_virtual());
            Ok(())
        }
        Acao::HomeAssistant { servico, entidade } => {
            let Some((url, corpo)) = casa.chamada(servico, entidade) else {
                eprintln!(
                    "Home Assistant não configurado, ou serviço sem domínio: {servico:?}"
                );
                return Ok(());
            };
            let mut cabecalhos = BTreeMap::new();
            cabecalhos.insert(
                "Authorization".to_string(),
                format!("Bearer {}", casa.token.trim()),
            );
            relatar(rede::chamar(Metodo::Post, &url, &cabecalhos, Some(&corpo)));
            Ok(())
        }
        Acao::Http {
            url,
            metodo,
            cabecalhos,
            corpo,
        } => {
            relatar(rede::chamar(*metodo, url, cabecalhos, corpo.as_deref()));
            Ok(())
        }
        // Tratadas pelo estado, não aqui.
        Acao::ProximaPagina
        | Acao::PaginaAnterior
        | Acao::IrParaPagina { .. }
        | Acao::PausarRetomar => Ok(()),
    }
}

/// Uma requisição que falha não derruba nada; só vira aviso no log. O pad já
/// acendeu e a pessoa já seguiu a vida.
fn relatar(resultado: Result<u16, String>) {
    match resultado {
        Ok(codigo) if (200..300).contains(&codigo) => {}
        Ok(codigo) => eprintln!("requisição respondeu {codigo}"),
        Err(e) => eprintln!("requisição falhou: {e}"),
    }
}

/// Completa um endereço digitado pela metade.
///
/// Ninguém digita `https://` numa ferramenta cujo campo já se chama "Link", e
/// sem esquema o `start` do Windows trata o texto como nome de arquivo e não
/// abre nada. O que já tem esquema passa intacto.
fn completar_url(url: &str) -> String {
    let limpo = url.trim();
    if limpo.is_empty() {
        return limpo.to_string();
    }
    // Qualquer coisa antes de "://" é esquema: http, https, ftp, steam, obsidian.
    // `mailto:` e outros de dois pontos simples também passam.
    let tem_esquema = match limpo.find(':') {
        Some(i) => limpo[..i]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.'),
        None => false,
    };
    if tem_esquema {
        limpo.to_string()
    } else {
        format!("https://{limpo}")
    }
}

/// Extensões que o `CreateProcess` não sabe abrir sozinho. Atalho e script de shell
/// dependem do interpretador do Windows.
const SO_PELO_SHELL: [&str; 4] = ["lnk", "url", "appref-ms", "msc"];

fn precisa_do_shell(caminho: &str) -> bool {
    caminho
        .rsplit('.')
        .next()
        .map(|ext| SO_PELO_SHELL.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Abre pelo shell do Windows, que resolve atalho, associação de tipo e nome curto.
/// O par de aspas vazias é o título da janela, que o `start` exige quando o
/// argumento seguinte vem entre aspas.
/// Abre pelo Explorer, que é quem sabe abrir atalho, documento e app da loja.
///
/// Não usa `cmd /C start` pelo mesmo motivo da URL: o `cmd` quebraria o
/// argumento no `&`, e um caminho como `C:\Tools\AT&Tpp.lnk` viraria dois
/// comandos. O Explorer recebe o caminho inteiro.
///
/// Argumentos para o programa não passam por aqui: o Explorer não os repassa.
/// Quem tem argumento vai pelo `Command` direto, no ramo de cima.
fn abrir_pelo_shell(caminho: &str, argumentos: &[String]) -> std::io::Result<()> {
    use std::process::Command;
    if argumentos.is_empty() {
        Command::new("explorer.exe").arg(caminho.trim()).spawn()?;
        return Ok(());
    }
    // Com argumentos, o jeito é o `start` do shell mesmo. Aspas em volta de
    // cada parte impedem o `cmd` de reparsear `&` e companhia.
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "start", "", &aspas(caminho)]);
    for a in argumentos {
        cmd.arg(aspas(a));
    }
    cmd.spawn()?;
    Ok(())
}

/// Envolve em aspas para o `cmd` não reparsear o conteúdo. Aspas de dentro são
/// removidas: elas fechariam a nossa e devolveriam o controle ao shell.
fn aspas(valor: &str) -> String {
    format!("\"{}\"", valor.trim().replace('"', ""))
}

impl TeclaMidia {
    fn codigo_virtual(self) -> u16 {
        // Códigos de tecla virtual do Windows.
        match self {
            TeclaMidia::TocarPausar => 0xB3,     // VK_MEDIA_PLAY_PAUSE
            TeclaMidia::Proxima => 0xB0,         // VK_MEDIA_NEXT_TRACK
            TeclaMidia::Anterior => 0xB1,        // VK_MEDIA_PREV_TRACK
            TeclaMidia::Parar => 0xB2,           // VK_MEDIA_STOP
            TeclaMidia::AumentarVolume => 0xAF,  // VK_VOLUME_UP
            TeclaMidia::DiminuirVolume => 0xAE,  // VK_VOLUME_DOWN
            TeclaMidia::Mudo => 0xAD,            // VK_VOLUME_MUTE
        }
    }
}

/// Simulação de teclado no Windows, via SendInput.
pub mod teclado {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
        VIRTUAL_KEY,
    };

    /// Traduz um nome de tecla no código virtual do Windows.
    /// Aceita as teclas comuns de atalho; devolve `None` para nome desconhecido.
    pub fn codigo_da_tecla(nome: &str) -> Option<u16> {
        let n = nome.trim().to_lowercase();
        Some(match n.as_str() {
            "ctrl" | "control" => 0x11,
            "shift" => 0x10,
            "alt" => 0x12,
            "win" | "super" | "meta" => 0x5B,
            "enter" | "return" => 0x0D,
            "tab" => 0x09,
            "esc" | "escape" => 0x1B,
            "espaco" | "space" => 0x20,
            "backspace" => 0x08,
            "delete" | "del" => 0x2E,
            "home" => 0x24,
            "end" => 0x23,
            "pageup" => 0x21,
            "pagedown" => 0x22,
            "cima" | "up" => 0x26,
            "baixo" | "down" => 0x28,
            "esquerda" | "left" => 0x25,
            "direita" | "right" => 0x27,
            "printscreen" | "print" => 0x2C,
            "insert" | "ins" => 0x2D,
            "capslock" => 0x14,
            // Pontuacao. Os codigos OEM valem para o teclado dos EUA, que e o
            // layout que o Windows usa para traduzir estes atalhos.
            "menos" | "minus" | "-" => 0xBD,
            "mais" | "plus" | "igual" | "equal" | "=" | "+" => 0xBB,
            "ponto" | "period" | "." => 0xBE,
            "virgula" | "comma" | "," => 0xBC,
            "pontoevirgula" | "semicolon" | ";" => 0xBA,
            "barra" | "slash" | "/" => 0xBF,
            "crase" | "backtick" | "`" => 0xC0,
            "colchete_esquerdo" | "bracketleft" | "[" => 0xDB,
            "contrabarra" | "backslash" => 0xDC,
            "colchete_direito" | "bracketright" | "]" => 0xDD,
            "apostrofo" | "quote" | "'" => 0xDE,
            _ => {
                // F1 a F24
                if let Some(resto) = n.strip_prefix('f') {
                    if let Ok(numero) = resto.parse::<u16>() {
                        if (1..=24).contains(&numero) {
                            return Some(0x70 + numero - 1);
                        }
                    }
                }
                // Letras e números soltos usam o próprio código ASCII maiúsculo.
                let mut chars = n.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if c.is_ascii_alphanumeric() => {
                        c.to_ascii_uppercase() as u16
                    }
                    _ => return None,
                }
            }
        })
    }

    /// Manda um atalho como "ctrl+shift+n": aperta os modificadores na ordem,
    /// aperta e solta a última tecla, e solta os modificadores na ordem inversa.
    pub fn mandar_atalho(atalho: &str) {
        // Uma parte desconhecida cancela o atalho inteiro. Descartar so ela
        // mandaria outro atalho: "ctrl+xis" viraria um "ctrl" solto.
        let mut codigos: Vec<u16> = Vec::new();
        for parte in atalho.split('+') {
            match codigo_da_tecla(parte) {
                Some(c) => codigos.push(c),
                None => {
                    // Parte vazia e so separador solto ("ctrl+" ou "ctrl++").
                    // Para a tecla "+" em si, escreva "ctrl+mais".
                    if parte.trim().is_empty() {
                        continue;
                    }
                    eprintln!("atalho ignorado, tecla desconhecida: {parte:?} em {atalho:?}");
                    return;
                }
            }
        }
        if codigos.is_empty() {
            eprintln!("atalho não reconhecido: {atalho}");
            return;
        }
        for &c in &codigos {
            if !enviar(c, false) {
                // Solta o que já foi apertado: deixar um modificador preso
                // trava o teclado inteiro da pessoa.
                for &solta in codigos.iter().rev() {
                    enviar(solta, true);
                }
                return;
            }
        }
        for &c in codigos.iter().rev() {
            enviar(c, true);
        }
    }

    /// Aperta e solta uma tecla virtual só.
    pub fn mandar_tecla_virtual(codigo: u16) {
        if enviar(codigo, false) {
            enviar(codigo, true);
        }
    }

    /// Gira a roda do mouse, como o scroll. `passos` positivo rola para cima.
    ///
    /// Um passo é a unidade que o Windows chama de "linha de rolagem", o mesmo
    /// que um clique da roda de um mouse comum.
    pub fn rolar(passos: i32) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            MOUSEEVENTF_WHEEL, MOUSEINPUT,
        };
        const RODA_POR_PASSO: i32 = 120; // WHEEL_DELTA
        let mut entrada = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: (passos * RODA_POR_PASSO) as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            SendInput(1, &mut entrada, std::mem::size_of::<INPUT>() as i32);
        }
    }

    /// Teclas que o Windows chama de estendidas. Sem a marca, o `Ctrl` direito
    /// vira esquerdo e as setas viram as do teclado numérico.
    fn e_estendida(codigo: u16) -> bool {
        matches!(
            codigo,
            0x21..=0x28 // PageUp, PageDown, End, Home, setas
                | 0x2D | 0x2E // Insert, Delete
                | 0x5B | 0x5C // Win esquerda e direita
                | 0x5D // Menu de contexto
                | 0x90 // NumLock
                | 0xA3 // Ctrl direito
                | 0xA5 // Alt direito
        )
    }

    /// Aperta ou solta uma tecla. Devolve `false` quando o Windows recusou.
    ///
    /// Dois detalhes que parecem enfeite e não são:
    ///
    /// - **O scancode.** Programas que escutam o teclado por hook de baixo
    ///   nível, como lançadores e sobreposições de jogo, descartam tecla que
    ///   chega sem scancode. Era por isso que a tecla Windows não abria nada.
    /// - **O retorno.** `SendInput` devolve zero quando o Windows bloqueia a
    ///   injeção, o que acontece quando a janela em foco roda com privilégio
    ///   maior que o nosso. Ignorar isso é ficar sem saber por que nada
    ///   aconteceu.
    fn enviar(codigo: u16, soltar: bool) -> bool {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            MapVirtualKeyW, KEYEVENTF_EXTENDEDKEY, MAPVK_VK_TO_VSC,
        };
        let scan = unsafe { MapVirtualKeyW(codigo as u32, MAPVK_VK_TO_VSC) } as u16;
        let mut flags = if soltar { KEYEVENTF_KEYUP } else { 0 };
        if e_estendida(codigo) {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        let mut entrada = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: codigo as VIRTUAL_KEY,
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let enviados =
            unsafe { SendInput(1, &mut entrada, std::mem::size_of::<INPUT>() as i32) };
        if enviados == 0 {
            eprintln!(
                "o Windows recusou a tecla {codigo:#04x}. A janela em foco costuma                  rodar como administrador; rode o MikroDeck como administrador também."
            );
            return false;
        }
        true
    }
}

#[cfg(test)]
mod testes_url {
    use super::completar_url;

    #[test]
    fn endereco_sem_esquema_ganha_https() {
        assert_eq!(completar_url("youtube.com"), "https://youtube.com");
        assert_eq!(completar_url("google.com/maps"), "https://google.com/maps");
        assert_eq!(completar_url("  claude.ai  "), "https://claude.ai");
    }

    #[test]
    fn endereco_com_esquema_passa_intacto() {
        assert_eq!(completar_url("https://claude.ai"), "https://claude.ai");
        assert_eq!(completar_url("http://casa.local:8123"), "http://casa.local:8123");
        assert_eq!(
            completar_url("mailto:alguem@exemplo.com"),
            "mailto:alguem@exemplo.com"
        );
        assert_eq!(completar_url("obsidian://open"), "obsidian://open");
    }

    #[test]
    fn vazio_continua_vazio() {
        assert_eq!(completar_url("   "), "");
    }
}

#[cfg(test)]
mod testes_shell {
    use super::aspas;

    #[test]
    fn as_aspas_impedem_o_shell_de_reparsear() {
        // O `cmd` trata `&` como separador de comando. Sem as aspas, um caminho
        // com `&` viraria dois comandos, e o segundo rodaria de verdade.
        let caminho = r"C:\Tools\AT&T\app.lnk";
        let entre_aspas = aspas(caminho);
        assert!(entre_aspas.starts_with('"') && entre_aspas.ends_with('"'));
        assert!(entre_aspas.contains("AT&T"), "perdeu o & : {entre_aspas}");
        assert_eq!(entre_aspas.len(), caminho.len() + 2);
    }

    #[test]
    fn aspas_de_dentro_somem_para_nao_fechar_as_nossas() {
        // Uma aspa no meio fecharia a nossa e devolveria o resto ao shell.
        let com_aspas = aspas(r#"a" & whoami & "b"#);
        assert_eq!(com_aspas.matches('"').count(), 2, "{com_aspas}");
        assert!(com_aspas.starts_with('"') && com_aspas.ends_with('"'));
    }

    #[test]
    fn a_url_do_youtube_com_e_comercial_fica_inteira() {
        // Era o bug: `...?v=abc&list=xyz` abria so ate o `&`, e o resto virava
        // comando. Agora a URL vai pelo Explorer, num argumento so.
        let url = super::completar_url("youtube.com/watch?v=abc&list=xyz");
        assert_eq!(url, "https://youtube.com/watch?v=abc&list=xyz");
        assert!(url.contains("&list=xyz"), "a URL perdeu o rabo: {url}");
    }
}

#[cfg(test)]
mod testes_teclas {
    use crate::acoes::teclado::codigo_da_tecla;

    #[test]
    fn pontuacao_tem_codigo() {
        // Sem isso, "ctrl+minus" perdia a tecla e mandava um "ctrl" solto.
        for nome in ["minus", "-", "equal", "=", "period", ".", "comma", "/"] {
            assert!(codigo_da_tecla(nome).is_some(), "{nome} sem codigo");
        }
        assert_eq!(codigo_da_tecla("menos"), codigo_da_tecla("-"));
        assert_eq!(codigo_da_tecla("ponto"), codigo_da_tecla("."));
    }

    #[test]
    fn tecla_desconhecida_continua_sem_codigo() {
        assert_eq!(codigo_da_tecla("xis grande"), None);
        assert_eq!(codigo_da_tecla("f99"), None);
    }

    #[test]
    fn as_teclas_das_paginas_prontas_existem() {
        // Guarda de regressao: toda tecla que as paginas prontas usam precisa
        // ter codigo, senao o atalho e cancelado inteiro em silencio.
        for atalho in [
            "ctrl+t", "ctrl+w", "ctrl+shift+t", "ctrl+shift+n", "alt+left",
            "alt+right", "f5", "ctrl+f", "ctrl+shift+equal", "ctrl+minus",
            "ctrl+0", "f11", "ctrl+h", "ctrl+j", "ctrl+d", "ctrl+shift+o",
            "win+left", "win+right", "win+up", "win+down", "alt+tab",
            "win+tab", "win+d", "alt+f4", "win+shift+s", "win+alt+r",
            "win+v", "win+period", "win+i", "win+p", "win+l",
        ] {
            for parte in atalho.split('+') {
                assert!(codigo_da_tecla(parte).is_some(), "{parte:?} em {atalho:?}");
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_as_acoes_de_pagina_mexem_no_estado() {
        assert_eq!(
            Acao::ProximaPagina.efeito_no_estado(),
            Some(EfeitoNoEstado::ProximaPagina)
        );
        assert_eq!(
            Acao::IrParaPagina { numero: 3 }.efeito_no_estado(),
            Some(EfeitoNoEstado::IrParaPagina(3))
        );
        assert_eq!(
            Acao::AbrirUrl {
                url: "https://exemplo.com".into()
            }
            .efeito_no_estado(),
            None
        );
    }

    #[test]
    fn atalho_do_windows_vai_pelo_shell() {
        // Foi um bug real: escolher um .lnk no seletor de arquivos falhava com
        // "não é um aplicativo Win32 válido", porque o CreateProcess não abre atalho.
        assert!(precisa_do_shell(r"C:\Users\x\Menu\Alethe.lnk"));
        assert!(precisa_do_shell("atalho.LNK"), "extensão sem diferenciar maiúscula");
        assert!(precisa_do_shell("favorito.url"));
        assert!(!precisa_do_shell(r"C:\Windows\notepad.exe"));
        assert!(!precisa_do_shell("chrome"));
    }

    #[test]
    fn traduz_teclas_de_atalho() {
        use teclado::codigo_da_tecla;
        assert_eq!(codigo_da_tecla("ctrl"), Some(0x11));
        assert_eq!(codigo_da_tecla("Shift"), Some(0x10));
        assert_eq!(codigo_da_tecla("f5"), Some(0x74));
        assert_eq!(codigo_da_tecla("F12"), Some(0x7B));
        assert_eq!(codigo_da_tecla("n"), Some(b'N' as u16));
        assert_eq!(codigo_da_tecla("7"), Some(b'7' as u16));
        assert_eq!(codigo_da_tecla("tecla_que_nao_existe"), None);
        assert_eq!(codigo_da_tecla("f99"), None);
    }

    #[test]
    fn acao_vai_e_volta_do_json() {
        let a = Acao::Atalho {
            teclas: "ctrl+shift+n".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"tipo\":\"atalho\""));
        assert_eq!(serde_json::from_str::<Acao>(&json).unwrap(), a);
    }

    #[test]
    fn abrir_programa_aceita_json_sem_argumentos() {
        let a: Acao =
            serde_json::from_str(r#"{"tipo":"abrir_programa","caminho":"notepad.exe"}"#).unwrap();
        assert_eq!(
            a,
            Acao::AbrirPrograma {
                caminho: "notepad.exe".into(),
                argumentos: vec![]
            }
        );
    }
}
