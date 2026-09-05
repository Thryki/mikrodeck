//! Sessão única de observação do aparelho.
//!
//! Roda um padrão bem visível por cerca de 30 segundos, alternando tela e LEDs,
//! para responder de uma vez: alguma saída HID chega ao Mikro MK3 no Windows?
//!
//! Cada etapa é anunciada no terminal antes de acontecer e dura 4 segundos.

use hidapi::{HidApi, HidDevice};
use std::thread::sleep;
use std::time::Duration;

const VID: u16 = 0x17cc;
const PID: u16 = 0x1700;

fn tela(dev: &HidDevice, ligada: bool) {
    let preenchimento = if ligada { 0xFFu8 } else { 0x00 };
    for metade in 0..2u8 {
        let mut p = Vec::with_capacity(265);
        p.push(0xE0);
        p.extend_from_slice(&[0x00, 0x00]);
        p.extend_from_slice(&[metade * 2, 0x00]);
        p.extend_from_slice(&[0x80, 0x00]);
        p.extend_from_slice(&[0x02, 0x00]);
        p.extend_from_slice(&[preenchimento; 256]);
        let _ = dev.write(&p);
    }
}

fn leds(dev: &HidDevice, cor: u8, intensidade: u8, brilho_botoes: u8) {
    let mut b = vec![0u8; 265];
    b[0] = 0x80;
    for i in 1..40 {
        b[i] = brilho_botoes;
    }
    let v = 0x04 * cor + intensidade;
    for i in 0..16 {
        b[40 + i] = v;
    }
    for i in 0..25 {
        b[56 + i] = v;
    }
    let _ = dev.write(&b);
}

fn etapa(msg: &str) {
    println!(">>> {msg}");
    sleep(Duration::from_secs(4));
}

fn main() {
    let api = HidApi::new().expect("hidapi");
    let dev = api.open(VID, PID).expect("abrir Mikro MK3");
    println!("Device aberto. Olhe o aparelho pelos próximos 30 segundos.\n");
    sleep(Duration::from_secs(2));

    tela(&dev, true);
    etapa("TELA deveria estar TODA ACESA");

    tela(&dev, false);
    etapa("TELA deveria estar APAGADA");

    tela(&dev, true);
    etapa("TELA ACESA de novo");

    tela(&dev, false);
    leds(&dev, 1, 3, 13);
    etapa("PADS deveriam estar VERMELHOS e os botões acesos");

    leds(&dev, 7, 3, 13);
    etapa("PADS deveriam estar VERDES");

    leds(&dev, 11, 3, 13);
    etapa("PADS deveriam estar AZUIS");

    leds(&dev, 0, 0, 0);
    etapa("Tudo APAGADO");

    println!("\nFim da sessão.");
}
