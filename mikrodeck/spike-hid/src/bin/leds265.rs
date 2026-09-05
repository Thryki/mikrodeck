//! Testa o caminho de saída pelo pipe de controle com o tamanho que o Windows exige.
//!
//! HidD_SetOutputReport exige que o buffer tenha exatamente OutputReportByteLength
//! bytes (aqui 265), não o tamanho do report em si. As tentativas anteriores com
//! 81 e 91 bytes falharam por causa disso.

use hidapi::HidApi;
use std::thread::sleep;
use std::time::Duration;

const VID: u16 = 0x17cc;
const PID: u16 = 0x1700;
const TAM_SO: usize = 265;

/// Frame de LEDs no buffer de 265 bytes que o Windows exige.
/// Layout real do report 0x80: byte 0 = ID, 1..39 botões, 40..55 pads, 56..80 strip.
fn frame(cor: u8, intensidade: u8, brilho_botoes: u8) -> Vec<u8> {
    let mut b = vec![0u8; TAM_SO];
    b[0] = 0x80;
    for i in 1..40 {
        b[i] = brilho_botoes;
    }
    let valor = 0x04 * cor + intensidade;
    for i in 0..16 {
        b[40 + i] = valor;
    }
    for i in 0..25 {
        b[56 + i] = valor;
    }
    b
}

fn main() {
    let api = HidApi::new().expect("hidapi");
    let dev = api.open(VID, PID).expect("abrir Mikro MK3");
    println!("Device aberto: {:?}\n", dev.get_product_string().ok().flatten());

    let cores = [("VERMELHO", 1u8), ("VERDE", 7), ("AZUL", 11), ("BRANCO", 17)];

    for (nome, cor) in cores {
        let f = frame(cor, 3, 13);
        match dev.send_output_report(&f) {
            Ok(()) => println!("send_output_report 265 bytes, {nome}: ok"),
            Err(e) => println!("send_output_report 265 bytes, {nome}: falhou: {e}"),
        }
        println!("  Olhe os pads (4s)...");
        sleep(Duration::from_secs(4));
    }

    println!("\nApagando.");
    let _ = dev.send_output_report(&frame(0, 0, 0));
    println!("Fim.");
}
