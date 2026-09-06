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
