//! Mexe nas janelas de um programa que já está aberto.
//!
//! Existe para o pad de abrir programa poder fazer mais do que abrir: trazer
//! para a frente, minimizar e fechar. Tudo pelo nome do executável, que é a
//! única coisa que o pad conhece.

use std::time::Duration;

/// A partir daqui, apertar virou segurar.
pub const LIMIAR_SEGURAR: Duration = Duration::from_millis(700);

/// A partir daqui, dois toques deixam de ser dois toques separados.
pub const JANELA_DUPLO_TOQUE: Duration = Duration::from_millis(350);

/// O que o pad faz quando é solto, para um programa com gerência de janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depois {
    /// O programa não estava aberto: abre.
    Abrir,
    /// Já estava aberto: traz para a frente, ou minimiza se já estava na frente.
    AlternarFrente,
    /// Dois toques rápidos: maximiza, ou volta ao tamanho se já estava maximizada.
    Maximizar,
    /// Ficou segurado: fecha.
    Fechar,
}

/// O que o pad faz quando é solto, para um link com gerência de janela.
///
/// Muda do programa em duas coisas: segurar abre outra janela em vez de fechar,
/// e um toque simples nunca minimiza, porque ir para o site é o que se espera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepoisNoLink {
    /// Não havia janela do site: abre.
    Abrir,
    /// Já havia: vai para ela em vez de abrir outra.
    IrParaJanela,
    /// Dois toques rápidos: maximiza, ou volta ao tamanho.
    Maximizar,
    /// Segurou: abre mais uma, mesmo já tendo.
    AbrirOutra,
}

/// Decide o que um pad de link faz quando é solto.
pub fn ao_soltar_link(segurado: Duration, desde_o_toque_anterior: Option<Duration>) -> DepoisNoLink {
    if segurado >= LIMIAR_SEGURAR {
        return DepoisNoLink::AbrirOutra;
    }
    if desde_o_toque_anterior.is_some_and(|d| d < JANELA_DUPLO_TOQUE) {
        return DepoisNoLink::Maximizar;
    }
    DepoisNoLink::IrParaJanela
}

/// Decide o que fazer quando o pad é solto.
///
/// Toque curto abre, ou alterna entre frente e minimizada se já estiver aberto.
/// Segurar maximiza, e segurar de novo desmaximiza. Dois toques rápidos fecham.
///
/// Maximizar ficou no segurar, e não no toque duplo, por causa de como o gesto
/// duplo acontece: o primeiro toque dele já minimizou a janela, e o segundo
/// chegava numa janela minimizada. Com a janela maximizada, dar dois toques
/// para desmaximizar acabava minimizando. Segurar não tem esse problema, porque
/// não passa pelo toque curto antes.
pub fn ao_soltar(
    segurado: Duration,
    aberto: bool,
    desde_o_toque_anterior: Option<Duration>,
) -> Depois {
    if segurado >= LIMIAR_SEGURAR {
        return Depois::Maximizar;
    }
    // Fechar, mesmo que o programa pareça fechado: pode ser uma janela que a
    // varredura de processos ainda não viu sumir, e fechar o que não existe não
    // faz mal.
    if desde_o_toque_anterior.is_some_and(|d| d < JANELA_DUPLO_TOQUE) {
        return Depois::Fechar;
    }
    if aberto {
        Depois::AlternarFrente
    } else {
        Depois::Abrir
    }
}

/// Nome pelo qual um site aparece no título da janela do navegador.
///
/// `https://www.google.com/maps` vira `google`, `open.spotify.com` vira
/// `spotify`, `claude.ai` vira `claude`. O navegador escreve o título da página
/// seguido do nome dele, e o nome do site quase sempre aparece ali.
pub fn rotulo_do_site(url: &str) -> Option<String> {
    let sem_esquema = url.trim().split("://").last()?;
    let host = sem_esquema.split(['/', '?', '#']).next()?;
    let host = host.split('@').last()?.split(':').next()?;
    let partes: Vec<&str> = host.split('.').filter(|p| !p.is_empty()).collect();
    let nome = match partes.len() {
        0 => return None,
        1 => partes[0],
        // O rótulo que interessa é o antes do sufixo: em `open.spotify.com`
        // é `spotify`, não `open`.
        n => partes[n - 2],
    };
    if nome.is_empty() || nome == "www" {
        return None;
    }
    Some(nome.to_lowercase())
}

/// O que fazer com a janela de um programa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alvo {
    /// Traz para a frente. Se já estiver na frente, minimiza.
    AlternarFrente,
    /// Só traz para a frente, sem nunca minimizar.
    TrazerParaFrente,
    /// Maximiza, ou volta ao tamanho anterior se já estava maximizada.
    Maximizar,
    /// Fecha, do jeito educado: o programa ainda pode pedir para salvar.
    Fechar,
}

#[cfg(windows)]
mod windows_impl {
    use super::Alvo;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetActiveWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed, PostMessageW,
        SetForegroundWindow, ShowWindow, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WM_CLOSE,
    };

    /// Traz a janela para a frente de verdade.
    ///
    /// `SetForegroundWindow` sozinho quase sempre é recusado: o Windows não
    /// deixa um programa sem foco roubar a frente, e o MikroDeck nunca tem
    /// foco, porque quem apertou o pad estava usando outra coisa. O jeito
    /// aceito é grudar a nossa fila de entrada na da janela que está na frente:
    /// enquanto estão grudadas, o Windows nos trata como parte da mesma
    /// interação e deixa passar.
    unsafe fn trazer_para_frente(janela: HWND) {
        unsafe {
            let meu = GetCurrentThreadId();
            let da_frente = {
                let f = GetForegroundWindow();
                if f.is_null() {
                    0
                } else {
                    GetWindowThreadProcessId(f, std::ptr::null_mut())
                }
            };
            let dela = GetWindowThreadProcessId(janela, std::ptr::null_mut());
            let grudar = |outro: u32, ligar: i32| {
                if outro != 0 && outro != meu {
                    AttachThreadInput(meu, outro, ligar);
                }
            };
            grudar(da_frente, 1);
            grudar(dela, 1);

            ShowWindow(janela, SW_RESTORE);
            BringWindowToTop(janela);
            SetForegroundWindow(janela);
            SetActiveWindow(janela);

            grudar(dela, 0);
            grudar(da_frente, 0);
        }
    }

    struct Busca {
        /// Nomes de executável que o programa configurado pode ter, em
        /// minúsculas. App do menu Iniciar dá mais de um palpite.
        alvos: Vec<String>,
        /// Pasta de instalação do programa, em minúsculas. Serve de segunda
        /// tentativa: um lançador abre janela com outro nome, mas mora na mesma
        /// pasta. `git-bash.exe` abre `mintty.exe`, os dois debaixo de
        /// `C:\Program Files\Git`.
        pasta: Option<String>,
        achadas: Vec<HWND>,
        por_pasta: Vec<HWND>,
        /// Terceira tentativa: nome parecido. `Microsoft.WindowsCalculator`
        /// abre `Calculator.exe`, e um contém o outro sem serem iguais.
        parecidas: Vec<HWND>,
    }

    /// Se dois nomes de executável são o mesmo programa.
    ///
    /// Igualdade primeiro. Depois, um contendo o outro, com pelo menos quatro
    /// letras: sem esse mínimo, uma pista curta casaria com meio Windows.
    fn casa(exe: &str, alvo: &str) -> bool {
        let corte = |s: &str| s.strip_suffix(".exe").unwrap_or(s).to_string();
        let (a, b) = (corte(exe), corte(alvo));
        a == b || (a.len() >= 4 && b.len() >= 4 && (a.contains(&b) || b.contains(&a)))
    }

    /// Aplica a ação nas janelas do programa. Devolve `false` se não achou nenhuma.
    pub fn agir(caminho: &str, alvo: Alvo) -> bool {
        let mut busca = Busca {
            alvos: crate::vigias::pistas_de_processo(caminho),
            pasta: super::pasta_do_programa(caminho),
            achadas: Vec::new(),
            por_pasta: Vec::new(),
            parecidas: Vec::new(),
        };
        unsafe {
            EnumWindows(Some(visitar), &mut busca as *mut Busca as LPARAM);
        }
        // Nome exato primeiro; depois a pasta de instalação; por último o
        // nome parecido, que é o palpite mais frouxo dos três.
        if busca.achadas.is_empty() {
            busca.achadas = std::mem::take(&mut busca.por_pasta);
        }
        if busca.achadas.is_empty() {
            busca.achadas = std::mem::take(&mut busca.parecidas);
        }
        let Some(&janela) = busca.achadas.first() else {
            return false;
        };
        aplicar(alvo, janela, &busca.achadas);
        true
    }

    /// Aplica a ação numa janela já escolhida.
    fn aplicar(alvo: Alvo, janela: HWND, todas: &[HWND]) {
        unsafe {
            match alvo {
                Alvo::Fechar => {
                    // WM_CLOSE em todas: um programa pode ter várias janelas.
                    for j in todas {
                        PostMessageW(*j, WM_CLOSE, 0, 0);
                    }
                }
                Alvo::AlternarFrente => {
                    // Basta **alguma** janela do programa estar na frente. Um
                    // programa costuma ter várias, e exigir que a da frente
                    // fosse justo a primeira da lista fazia o pad nunca
                    // minimizar: caía sempre no "traz para a frente" de quem
                    // já estava na frente.
                    let em_foco = GetForegroundWindow();
                    let alguma_na_frente = todas
                        .iter()
                        .any(|j| *j == em_foco && IsIconic(*j) == 0);
                    if alguma_na_frente {
                        for j in todas {
                            ShowWindow(*j, SW_MINIMIZE);
                        }
                    } else {
                        trazer_para_frente(janela);
                    }
                }
                Alvo::TrazerParaFrente => {
                    trazer_para_frente(janela);
                }
                Alvo::Maximizar => {
                    if IsZoomed(janela) != 0 {
                        ShowWindow(janela, SW_RESTORE);
                    } else {
                        ShowWindow(janela, SW_MAXIMIZE);
                    }
                    SetForegroundWindow(janela);
                }
            }
        }
    }

    /// Acha janelas cujo título contenha o fragmento e aplica a ação na primeira.
    ///
    /// É assim que um pad de link acha a janela do site: o navegador escreve o
    /// título da página no título da janela, e o nome do site quase sempre está
    /// ali. Não é exato, mas é o que dá para saber de fora do navegador.
    pub fn agir_por_titulo(fragmento: &str, alvo: Alvo) -> bool {
        let mut busca = BuscaTitulo {
            fragmento: fragmento.to_lowercase(),
            achadas: Vec::new(),
        };
        if busca.fragmento.is_empty() {
            return false;
        }
        unsafe {
            EnumWindows(Some(visitar_titulo), &mut busca as *mut BuscaTitulo as LPARAM);
        }
        let Some(&janela) = busca.achadas.first() else {
            return false;
        };
        aplicar(alvo, janela, &busca.achadas);
        true
    }

    struct BuscaTitulo {
        fragmento: String,
        achadas: Vec<HWND>,
    }

    unsafe extern "system" fn visitar_titulo(janela: HWND, dados: LPARAM) -> i32 {
        unsafe {
            let busca = &mut *(dados as *mut BuscaTitulo);
            if IsWindowVisible(janela) == 0 {
                return 1;
            }
            let tamanho = GetWindowTextLengthW(janela);
            if tamanho <= 0 {
                return 1;
            }
            let mut buffer = vec![0u16; tamanho as usize + 1];
            let lidos = GetWindowTextW(janela, buffer.as_mut_ptr(), buffer.len() as i32);
            if lidos <= 0 {
                return 1;
            }
            let titulo = String::from_utf16_lossy(&buffer[..lidos as usize]).to_lowercase();
            if titulo.contains(busca.fragmento.as_str()) {
                busca.achadas.push(janela);
            }
            1
        }
    }

    /// Chamada pelo Windows uma vez por janela de topo.
    unsafe extern "system" fn visitar(janela: HWND, dados: LPARAM) -> i32 {
        unsafe {
            let busca = &mut *(dados as *mut Busca);
            // Janela invisível ou sem título é janela de serviço, não interessa.
            if IsWindowVisible(janela) == 0 || GetWindowTextLengthW(janela) == 0 {
                return 1;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(janela, &mut pid);
            if pid == 0 {
                return 1;
            }
            if let Some((exe, caminho)) = processo_do_pid(pid) {
                if busca.alvos.iter().any(|a| *a == exe) {
                    busca.achadas.push(janela);
                } else if busca
                    .pasta
                    .as_ref()
                    .is_some_and(|p| caminho.starts_with(p.as_str()))
                {
                    busca.por_pasta.push(janela);
                } else if busca.alvos.iter().any(|a| casa(&exe, a)) {
                    busca.parecidas.push(janela);
                }
            }
            1
        }
    }

    /// Nome e caminho do executável de um processo, os dois em minúsculas.
    unsafe fn processo_do_pid(pid: u32) -> Option<(String, String)> {
        let (ok, buffer, tamanho) = unsafe {
            let processo = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if processo.is_null() {
                return None;
            }
            let mut buffer = [0u16; 512];
            let mut tamanho = buffer.len() as u32;
            let ok = QueryFullProcessImageNameW(processo, 0, buffer.as_mut_ptr(), &mut tamanho);
            windows_sys::Win32::Foundation::CloseHandle(processo);
            (ok, buffer, tamanho)
        };
        if ok == 0 {
            return None;
        }
        let caminho = String::from_utf16_lossy(&buffer[..tamanho as usize]).to_lowercase();
        let base = caminho.rsplit(['\\', '/']).next()?.to_string();
        Some((base, caminho))
    }
}

#[cfg(windows)]
pub use windows_impl::{agir, agir_por_titulo};

#[cfg(not(windows))]
pub fn agir(_caminho: &str, _alvo: Alvo) -> bool {
    false
}

#[cfg(not(windows))]
pub fn agir_por_titulo(_fragmento: &str, _alvo: Alvo) -> bool {
    false
}

/// Nome do executável a partir de um caminho de configuração, em minúsculas e
/// sempre com `.exe`.
fn nome_do_executavel(caminho: &str) -> Option<String> {
    crate::vigias::nome_do_executavel(caminho)
}

/// Pasta onde o programa está instalado, em minúsculas. `None` quando o caminho
/// é só um nome curto, sem pasta nenhuma.
fn pasta_do_programa(caminho: &str) -> Option<String> {
    let limpo = caminho.trim().trim_matches('"');
    let corte = limpo.rfind(['\\', '/'])?;
    let pasta = &limpo[..corte];
    if pasta.is_empty() {
        return None;
    }
    Some(pasta.to_lowercase())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_rotulo_do_site_sai_da_url() {
        assert_eq!(rotulo_do_site("https://www.google.com/maps"), Some("google".into()));
        assert_eq!(rotulo_do_site("youtube.com"), Some("youtube".into()));
        assert_eq!(rotulo_do_site("https://open.spotify.com"), Some("spotify".into()));
        assert_eq!(rotulo_do_site("https://claude.ai/new"), Some("claude".into()));
        assert_eq!(rotulo_do_site("http://casa.local:8123/lovelace"), Some("casa".into()));
        assert_eq!(rotulo_do_site(""), None);
    }

    #[test]
    fn dois_toques_rapidos_fecham_o_programa() {
        let rapido = Some(Duration::from_millis(200));
        assert_eq!(ao_soltar(Duration::from_millis(60), true, rapido), Depois::Fechar);
        // No link, dois toques continuam maximizando: la nao ha o que fechar
        // sem fechar o navegador inteiro.
        assert_eq!(
            ao_soltar_link(Duration::from_millis(60), rapido),
            DepoisNoLink::Maximizar
        );
    }

    #[test]
    fn toque_devagar_nao_conta_como_duplo() {
        let devagar = Some(Duration::from_millis(900));
        assert_eq!(
            ao_soltar(Duration::from_millis(60), true, devagar),
            Depois::AlternarFrente
        );
        assert_eq!(
            ao_soltar_link(Duration::from_millis(60), devagar),
            DepoisNoLink::IrParaJanela
        );
    }

    #[test]
    fn segurar_ganha_do_duplo_toque() {
        // Se a pessoa segurou, é segurar, mesmo que o toque anterior tenha sido agora.
        let rapido = Some(Duration::from_millis(50));
        assert_eq!(ao_soltar(Duration::from_secs(1), true, rapido), Depois::Maximizar);
        assert_eq!(
            ao_soltar_link(Duration::from_secs(1), rapido),
            DepoisNoLink::AbrirOutra
        );
    }

    #[test]
    fn no_link_o_toque_simples_nunca_minimiza() {
        // Ir para o site é o que se espera; minimizar seria surpresa.
        assert_eq!(
            ao_soltar_link(Duration::from_millis(60), None),
            DepoisNoLink::IrParaJanela
        );
    }

    #[test]
    fn a_pasta_do_programa_sai_do_caminho() {
        assert_eq!(
            pasta_do_programa(r"C:\Program Files\Git\git-bash.exe"),
            Some(r"c:\program files\git".into())
        );
        // Nome curto não tem pasta: só o nome exato pode achar a janela.
        assert_eq!(pasta_do_programa("chrome.exe"), None);
    }

    #[test]
    fn o_lancador_e_a_janela_dividem_a_pasta_de_instalacao() {
        // git-bash.exe abre mintty.exe, e os dois moram debaixo da mesma pasta.
        let pasta = pasta_do_programa(r"C:\Program Files\Git\git-bash.exe").unwrap();
        let janela = r"c:\program files\git\usr\bin\mintty.exe";
        assert!(janela.starts_with(pasta.as_str()));
    }

    #[test]
    fn toque_curto_em_programa_fechado_abre() {
        assert_eq!(ao_soltar(Duration::from_millis(80), false, None), Depois::Abrir);
    }

    #[test]
    fn toque_curto_em_programa_aberto_alterna_a_frente() {
        assert_eq!(
            ao_soltar(Duration::from_millis(80), true, None),
            Depois::AlternarFrente
        );
    }

    #[test]
    fn segurar_maximiza_esteja_o_programa_como_estiver() {
        assert_eq!(ao_soltar(LIMIAR_SEGURAR, true, None), Depois::Maximizar);
        assert_eq!(
            ao_soltar(Duration::from_secs(2), false, None),
            Depois::Maximizar
        );
    }

    #[test]
    fn desmaximizar_nao_passa_por_minimizar() {
        // Era a queixa: com a janela maximizada, dois toques para desmaximizar
        // acabavam minimizando, porque o primeiro toque do gesto ja minimizava.
        // Segurar nao passa pelo toque curto, entao vai direto.
        let gesto = ao_soltar(Duration::from_secs(1), true, None);
        assert_eq!(gesto, Depois::Maximizar);
        assert_ne!(gesto, Depois::AlternarFrente);
    }
}
