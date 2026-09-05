//! Teste decisivo do canal de saída do Maschine Mikro MK3 no Windows.
//!
//! O report da tela (0xe0) tem 265 bytes, exatamente o tamanho que o Windows
//! força em toda escrita HID neste aparelho (OutputReportByteLength). Se a tela
//! reagir, o transporte de saída funciona e o problema está só no report 0x80.
//! Se a tela também não reagir, nenhuma escrita HID chega ao aparelho no Windows.
//!
//! Formato da tela (pymikro): 128x32, 1 bit por pixel, mandada em duas metades
//! de 128x16. Cada metade: header de 9 bytes + 256 bytes de bitmap.
//! Header: [0xE0, x_lo, x_hi, y_lo, y_hi, w_lo, w_hi, h_lo, h_hi]
//! com w = 128 pixels e h = 2 (em unidades de 8 pixels, ou seja 16 linhas).
//! No bitmap, cada byte cobre 8 linhas de uma mesma coluna.

use hidapi::HidApi;
use std::thread::sleep;
use std::time::Duration;

const VID: u16 = 0x17cc;
const PID: u16 = 0x1700;

/// Monta um dos dois pacotes da tela. `metade` 0 = linhas 0..15, 1 = linhas 16..31.
/// `bitmap` tem 256 bytes: 128 colunas x 2 bytes (16 linhas).
fn pacote_tela(metade: u8, bitmap: &[u8; 256]) -> Vec<u8> {
    let mut p = Vec::with_capacity(265);
    p.push(0xE0);
    p.extend_from_slice(&[0x00, 0x00]); // x = 0
    p.extend_from_slice(&[metade * 2, 0x00]); // y, em unidades de 8 linhas
    p.extend_from_slice(&[0x80, 0x00]); // largura = 128
    p.extend_from_slice(&[0x02, 0x00]); // altura = 2 unidades = 16 linhas
    p.extend_from_slice(bitmap);
    p
}

/// Bitmap de 256 bytes onde todos os pixels estão acesos.
fn tudo_aceso() -> [u8; 256] {
    [0xFF; 256]
}

/// Bitmap com listras verticais grossas, fácil de reconhecer na tela.
fn listras() -> [u8; 256] {
    let mut b = [0u8; 256];
    for coluna in 0..128 {
        let aceso = (coluna / 8) % 2 == 0;
        b[coluna * 2] = if aceso { 0xFF } else { 0x00 };
        b[coluna * 2 + 1] = if aceso { 0xFF } else { 0x00 };
    }
    b
}

fn enviar(dev: &hidapi::HidDevice, rotulo: &str, bitmap: &[u8; 256]) {
    for metade in 0..2u8 {
        let p = pacote_tela(metade, bitmap);
        match dev.write(&p) {
            Ok(n) => println!("  {rotulo} metade {metade}: write() ok, {n} bytes"),
            Err(e) => println!("  {rotulo} metade {metade}: falhou: {e}"),
        }
    }
}

fn main() {
    let api = HidApi::new().expect("hidapi");
    let dev = api.open(VID, PID).expect("abrir Mikro MK3");
    println!("Device aberto: {:?}\n", dev.get_product_string().ok().flatten());

    println!("=== Tela toda acesa ===");
    enviar(&dev, "acesa", &tudo_aceso());
    println!("Olhe a TELA do aparelho (5s)...");
    sleep(Duration::from_secs(5));

    println!("\n=== Tela apagada ===");
    enviar(&dev, "apagada", &[0x00; 256]);
    println!("A tela deve ter apagado (5s)...");
    sleep(Duration::from_secs(5));

    println!("\n=== Listras verticais ===");
    enviar(&dev, "listras", &listras());
    println!("A tela deve mostrar listras (5s)...");
    sleep(Duration::from_secs(5));

    println!("\n=== Outros reports de saída (0xf3 e 0xf4), pode ser init/brilho ===");
    for (id, tam) in [(0xf3u8, 2usize), (0xf4u8, 33usize)] {
        let mut buf = vec![0u8; tam];
        buf[0] = id;
        for b in buf.iter_mut().skip(1) {
            *b = 0xFF;
        }
        match dev.write(&buf) {
            Ok(n) => println!("  report 0x{id:02x}: write() ok, {n} bytes"),
            Err(e) => println!("  report 0x{id:02x}: falhou: {e}"),
        }
    }
    println!("Algo mudou no aparelho (5s)?...");
    sleep(Duration::from_secs(5));

    println!("\n=== Depois disso, tentando os LEDs de novo (report 0x80, 81 bytes) ===");
    let mut leds = vec![0u8; 81];
    leds[0] = 0x80;
    for i in 0..16 {
        leds[40 + i] = 0x04 * 1 + 3; // vermelho
    }
    for i in 0..25 {
        leds[56 + i] = 0x04 * 1 + 3;
    }
    for i in 1..40 {
        leds[i] = 0x0D; // brilho de botão
    }
    match dev.write(&leds) {
        Ok(n) => println!("  LEDs: write() ok, {n} bytes"),
        Err(e) => println!("  LEDs: falhou: {e}"),
    }
    println!("Acendeu alguma coisa agora (5s)?...");
    sleep(Duration::from_secs(5));

    println!("\nFim.");
}
