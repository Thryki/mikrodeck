//! Diagnóstico do canal de saída (LEDs) do Maschine Mikro MK3 no Windows.
//!
//! A leitura já funciona. O que este binário responde:
//! - Quais report IDs de saída o aparelho declara, e com quantos bytes cada um.
//! - Qual caminho de escrita realmente acende o LED no Windows:
//!   HidDevice::write (WriteFile, buffer preenchido até o tamanho máximo do report)
//!   ou HidDevice::send_output_report (HidD_SetOutputReport, tamanho exato).

use hidapi::HidApi;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

const VID: u16 = 0x17cc;
const PID: u16 = 0x1700;
const PAD_ORDER: [u8; 16] = [12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3];

/// Percorre o report descriptor HID e imprime os itens que interessam:
/// Report ID, Report Size, Report Count e as marcas de Input/Output/Feature.
fn analisar_descriptor(d: &[u8]) {
    println!("Report descriptor: {} bytes", d.len());
    let mut i = 0usize;
    let mut report_id: u32 = 0;
    let mut size: u32 = 0;
    let mut count: u32 = 0;
    while i < d.len() {
        let prefixo = d[i];
        let tam = match prefixo & 0x03 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        };
        let tipo = (prefixo >> 2) & 0x03;
        let tag = prefixo >> 4;
        let mut valor: u32 = 0;
        for b in 0..tam {
            if i + 1 + b < d.len() {
                valor |= (d[i + 1 + b] as u32) << (8 * b);
            }
        }
        match (tipo, tag) {
            (1, 0x8) => {
                report_id = valor;
                println!("  Report ID 0x{valor:02x}");
            }
            (1, 0x7) => size = valor,
            (1, 0x9) => count = valor,
            (0, 0x8) => println!(
                "    INPUT   id=0x{report_id:02x} {count} x {size} bits = {} bytes de dados",
                (count * size) / 8
            ),
            (0, 0x9) => println!(
                "    OUTPUT  id=0x{report_id:02x} {count} x {size} bits = {} bytes de dados",
                (count * size) / 8
            ),
            (0, 0xB) => println!(
                "    FEATURE id=0x{report_id:02x} {count} x {size} bits = {} bytes de dados",
                (count * size) / 8
            ),
            _ => {}
        }
        i += 1 + tam;
    }
}

/// Frame de LEDs com o tamanho que o report descriptor declara: 81 bytes
/// (1 byte de report ID + 80 de dados). Todos os 16 pads acesos na cor pedida.
fn frame_todos_pads(cor: u8, intensidade: u8) -> Vec<u8> {
    let mut buf = vec![0u8; 81];
    buf[0] = 0x80;
    for idx in 0..16 {
        buf[40 + idx] = 0x04 * cor + intensidade;
    }
    let _ = PAD_ORDER;
    buf
}

fn pausa(msg: &str) {
    print!("{msg}");
    let _ = std::io::stdout().flush();
    sleep(Duration::from_secs(4));
    println!(" pronto.");
}

fn main() {
    let api = HidApi::new().expect("hidapi");
    let dev = api.open(VID, PID).expect("abrir Mikro MK3");
    println!("Device aberto: {:?}\n", dev.get_product_string().ok().flatten());

    let mut desc = [0u8; 4096];
    match dev.get_report_descriptor(&mut desc) {
        Ok(n) => {
            analisar_descriptor(&desc[..n]);
            println!();
        }
        Err(e) => println!("Não deu para ler o report descriptor: {e}\n"),
    }

    let frame = frame_todos_pads(1, 3); // vermelho, intensidade máxima
    println!("Frame de LEDs: {} bytes, começa com 0x{:02x}\n", frame.len(), frame[0]);

    println!("=== Teste 1: send_output_report() com 81 bytes, vermelho ===");
    match dev.send_output_report(&frame) {
        Ok(()) => println!("send_output_report() 81 ok"),
        Err(e) => println!("send_output_report() 81 falhou: {e}"),
    }
    pausa("Os 16 pads devem estar VERMELHOS (4s)...");

    println!("\n=== Teste 2: mesma coisa em verde ===");
    match dev.send_output_report(&frame_todos_pads(7, 3)) {
        Ok(()) => println!("verde ok"),
        Err(e) => println!("verde falhou: {e}"),
    }
    pausa("Os 16 pads devem estar VERDES (4s)...");

    println!("\n=== Teste 3: mesma coisa em azul, e strip acesa ===");
    let mut azul = frame_todos_pads(11, 3);
    for led in 0..25 {
        azul[56 + led] = 0x04 * 11 + 3;
    }
    match dev.send_output_report(&azul) {
        Ok(()) => println!("azul + strip ok"),
        Err(e) => println!("azul falhou: {e}"),
    }
    pausa("Pads AZUIS e a touch strip acesa (4s)...");

    println!("\n=== Teste 4: write() com 81 bytes (deve falhar/não acender no Windows) ===");
    let laranja = frame_todos_pads(2, 3);
    match dev.write(&laranja) {
        Ok(n) => println!("write() retornou {n} (o SO preencheu até o tamanho máximo)"),
        Err(e) => println!("write() falhou: {e}"),
    }
    pausa("Ficaram LARANJAS? Se continuaram azuis, write() não serve (4s)...");

    println!("\nFim. Apagando tudo.");
    let _ = dev.send_output_report(&frame_todos_pads(0, 0));
}
