//! Prova que o atalho sai do MikroDeck e chega no Windows.
//! `#[ignore]`: mexe no teclado da máquina de quem roda.

use std::time::Duration;

fn processo_em_foco() -> String {
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return "(nenhuma)".into();
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return format!("(pid {pid})");
        }
        let mut buf = [0u16; 260];
        let mut tam = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut tam);
        windows_sys::Win32::Foundation::CloseHandle(h);
        if ok == 0 {
            return format!("(pid {pid})");
        }
        String::from_utf16_lossy(&buf[..tam as usize])
            .rsplit('\\')
            .next()
            .unwrap_or("")
            .to_string()
    }
}

#[test]
#[ignore = "mexe no teclado da maquina"]
fn a_tecla_win_abre_o_iniciar() {
    let antes = processo_em_foco();
    println!("em foco antes: {antes}");
    motor::acoes::teclado::mandar_atalho("win");
    std::thread::sleep(Duration::from_millis(900));
    let depois = processo_em_foco();
    println!("em foco depois: {depois}");
    // Fecha o Iniciar para nao deixar a maquina bagunçada.
    motor::acoes::teclado::mandar_atalho("escape");
    assert_ne!(antes, depois, "a tecla win nao mudou o foco: nada abriu");
}

#[test]
#[ignore = "mexe no teclado da maquina"]
fn um_atalho_com_modificador_chega_no_windows() {
    // win+r abre o "Executar", uma janela classica que sempre pega o foco.
    // Se este passa e o "win" sozinho falha, o problema e a tecla Windows
    // sozinha, nao o envio.
    let antes = processo_em_foco();
    println!("em foco antes: {antes}");
    motor::acoes::teclado::mandar_atalho("win+r");
    std::thread::sleep(Duration::from_millis(1200));
    let depois = processo_em_foco();
    println!("em foco depois: {depois}");
    motor::acoes::teclado::mandar_atalho("escape");
    assert_ne!(antes, depois, "win+r nao abriu nada");
}

#[test]
#[ignore = "mexe no teclado da maquina"]
fn ctrl_esc_abre_o_iniciar() {
    // O equivalente classico da tecla Windows, e este funciona injetado.
    let antes = processo_em_foco();
    println!("em foco antes: {antes}");
    motor::acoes::teclado::mandar_atalho("ctrl+esc");
    std::thread::sleep(Duration::from_millis(1200));
    let depois = processo_em_foco();
    println!("em foco depois: {depois}");
    motor::acoes::teclado::mandar_atalho("escape");
    assert_ne!(antes, depois, "ctrl+esc nao abriu o Iniciar");
}

#[test]
#[ignore = "mexe no teclado da maquina"]
fn win_s_abre_a_busca() {
    let antes = processo_em_foco();
    println!("em foco antes: {antes}");
    motor::acoes::teclado::mandar_atalho("win+s");
    std::thread::sleep(Duration::from_millis(1400));
    let depois = processo_em_foco();
    println!("em foco depois: {depois}");
    motor::acoes::teclado::mandar_atalho("escape");
    assert_ne!(antes, depois, "win+s nao abriu a busca");
}

#[test]
#[ignore = "abre um programa de verdade"]
fn abrir_um_app_do_menu_iniciar_traz_ele_para_a_frente() {
    // O caso que nao funcionava: o Raycast e app da Microsoft Store, o
    // executavel dele mora numa pasta protegida, e atalho de teclado injetado
    // ele descarta. Pelo identificador do menu Iniciar, abre.
    let apps = motor::apps::listar().expect("listar");
    let Some(alvo) = apps.iter().find(|a| a.nome.to_lowercase().contains("raycast")) else {
        eprintln!("Raycast nao instalado: teste pulado");
        return;
    };
    println!("abrindo {} por {}", alvo.nome, alvo.caminho);
    let antes = processo_em_foco();
    motor::acoes::Executor::novo().disparar(motor::acoes::Acao::AbrirPrograma {
        caminho: alvo.caminho.clone(),
        argumentos: vec![],
    });
    std::thread::sleep(Duration::from_millis(2500));
    let depois = processo_em_foco();
    println!("em foco antes: {antes} | depois: {depois}");
    assert!(
        depois.to_lowercase().contains("raycast"),
        "o Raycast nao veio para a frente; ficou {depois}"
    );
}

#[test]
#[ignore = "mexe nas janelas de verdade"]
fn cuidar_da_janela_acha_o_app_do_menu_iniciar() {
    // O bug: com o caminho do menu Iniciar, o MikroDeck procurava uma janela
    // do processo "raycast.raycast_qypenmj9wpt2a!raycast.exe", que nao existe,
    // e por isso apertar de novo nao minimizava e segurar nao fechava.
    use motor::janelas::{agir, Alvo};
    let apps = motor::apps::listar().expect("listar");
    let Some(alvo) = apps.iter().find(|a| a.nome.to_lowercase().contains("raycast")) else {
        eprintln!("Raycast nao instalado: teste pulado");
        return;
    };
    // Garante que ele esta aberto.
    motor::acoes::Executor::novo().disparar(motor::acoes::Acao::AbrirPrograma {
        caminho: alvo.caminho.clone(),
        argumentos: vec![],
    });
    std::thread::sleep(Duration::from_millis(2500));

    let achou = agir(&alvo.caminho, Alvo::TrazerParaFrente);
    println!("achou a janela pelo caminho do menu Iniciar: {achou}");
    assert!(achou, "nao achou a janela: minimizar e fechar nunca iam funcionar");
    std::thread::sleep(Duration::from_millis(600));
    println!("em foco: {}", processo_em_foco());

    // E o gesto de alternar realmente minimiza quando ela esta na frente.
    assert!(agir(&alvo.caminho, Alvo::AlternarFrente));
    std::thread::sleep(Duration::from_millis(900));
    let depois = processo_em_foco();
    println!("depois de alternar: {depois}");
    assert!(
        !depois.to_lowercase().contains("raycast"),
        "continuou na frente: nao minimizou"
    );
}

/// Quantas janelas visiveis este processo tem, e se a primeira esta minimizada.
fn janelas_de(nome_exe: &str) -> (usize, bool) {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    };
    struct Conta {
        alvo: String,
        quantas: usize,
        minimizada: bool,
    }
    unsafe extern "system" fn visitar(janela: HWND, dados: LPARAM) -> i32 {
        unsafe {
            let c = &mut *(dados as *mut Conta);
            if IsWindowVisible(janela) == 0 || GetWindowTextLengthW(janela) == 0 {
                return 1;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(janela, &mut pid);
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return 1;
            }
            let mut buf = [0u16; 512];
            let mut tam = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut tam);
            windows_sys::Win32::Foundation::CloseHandle(h);
            if ok != 0 {
                let caminho = String::from_utf16_lossy(&buf[..tam as usize]).to_lowercase();
                if caminho.ends_with(&c.alvo) {
                    if c.quantas == 0 {
                        c.minimizada = IsIconic(janela) != 0;
                    }
                    c.quantas += 1;
                }
            }
            1
        }
    }
    let mut c = Conta {
        alvo: nome_exe.to_lowercase(),
        quantas: 0,
        minimizada: false,
    };
    unsafe {
        EnumWindows(Some(visitar), &mut c as *mut Conta as LPARAM);
    }
    (c.quantas, c.minimizada)
}

#[test]
#[ignore = "mexe nas janelas de verdade"]
fn minimizar_funciona_pelo_caminho_do_menu_iniciar() {
    // O bug: com o caminho do menu Iniciar, o MikroDeck procurava uma janela
    // de um processo que nao existe, e por isso apertar de novo nao minimizava.
    //
    // Nao fecha nada: fechar mataria as janelas de verdade de quem roda o teste.
    use motor::janelas::{agir, Alvo};
    let apps = motor::apps::listar().expect("listar");
    let Some(alvo) = apps.iter().find(|a| a.nome == "Terminal") else {
        eprintln!("Terminal nao esta no menu Iniciar: teste pulado");
        return;
    };
    println!("usando {} em {}", alvo.nome, alvo.caminho);
    println!("pistas: {:?}", motor::apps::pistas_publicas(&alvo.caminho));

    motor::acoes::Executor::novo().disparar(motor::acoes::Acao::AbrirPrograma {
        caminho: alvo.caminho.clone(),
        argumentos: vec![],
    });
    std::thread::sleep(Duration::from_millis(3000));
    let (quantas, _) = janelas_de("windowsterminal.exe");
    println!("janelas do Terminal: {quantas}");
    if quantas == 0 {
        eprintln!("o Terminal nao abriu: teste pulado");
        return;
    }

    // Traz para a frente primeiro: o gesto de minimizar so faz sentido quando
    // a janela esta na frente, que e a situacao de quem acabou de abrir o app.
    assert!(
        agir(&alvo.caminho, Alvo::TrazerParaFrente),
        "nao achou a janela pelo caminho do menu Iniciar"
    );
    std::thread::sleep(Duration::from_millis(900));
    println!("em foco antes de alternar: {}", processo_em_foco());

    assert!(agir(&alvo.caminho, Alvo::AlternarFrente));
    std::thread::sleep(Duration::from_millis(900));
    let (_, minimizada) = janelas_de("windowsterminal.exe");
    println!("minimizada: {minimizada}");

    // Devolve como estava, para nao deixar a maquina bagunçada.
    agir(&alvo.caminho, Alvo::TrazerParaFrente);
    assert!(minimizada, "apertar de novo devia ter minimizado");
}
