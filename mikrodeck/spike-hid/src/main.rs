//! Spike HID do Maschine Mikro MK3.
//!
//! Objetivo (CLAUDE.md, "Riscos a validar no spike"):
//! 1. Convivência com o serviço da NI (abre o device sem exclusividade?).
//! 2. No Windows, escrita HID exige um byte 0 de report ID no início do buffer?
//! 3. Resolução e formato de pixel da tela.
//!
//! Mapa de pacotes (protocolo levantado a partir de pymikro, https://github.com/flokapi/pymikro):
//! - VID 0x17cc, PID 0x1700.
//! - Entrada report 0x01 (botões/knob/strip): bitmask de botões nos bytes 1..6,
//!   knob tocado no byte 6, posição do knob (0..15 cíclico) no byte 7,
//!   posição 1 da strip no byte 10, posição 2 no byte 12.
//! - Entrada report 0x02 (pad): byte 1 = índice bruto do pad (0..15, ordem de hardware),
//!   byte 2 nibble alto = estado (0x40 toque, 0x10 pressão, 0x20/0x30 solta),
//!   valor de pressão = (byte 2 & 0x0F) * 256 + byte 3 (0..4095).
//! - Saída report 0x80 (LEDs): buffer de 91 bytes, byte 0 = 0x80.
//!   Pads a partir do offset 40 (16 bytes, um por pad, offset = índice bruto de hardware).
//!   Byte do pad = 0x04 * índice_da_cor + intensidade (intensidade 0..3, 0 = apagado).
//!   Índice da cor 1 = vermelho (tabela: off, red, orange, orange_light, yellow_warm,
//!   yellow, lime, green, mint, cyan, turquoise, blue, plum, violet, purple, magenta,
//!   fuchsia, white).
//! - Saída report 0xE0 (tela): dois pacotes de 9 + 256 bytes (header + metade do bitmap),
//!   128x32 pixels, 1 bit por pixel, 4 bytes por coluna (32 linhas / 8 bits).

use hidapi::HidApi;
use std::time::Duration;

const VID: u16 = 0x17cc;
const PID: u16 = 0x1700;

/// settings["pad"]["order"] do pymikro: índice bruto de hardware -> número lógico do pad.
const PAD_ORDER: [u8; 16] = [12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3];

const COR_VERMELHO: u8 = 1;

fn decode_botoes(buf: &[u8]) {
    if buf.len() <= 10 {
        return;
    }
    let mut btn_val: u64 = 0;
    for i in 0..5 {
        btn_val |= (buf[1 + i] as u64) << (8 * i);
    }
    let pressionados: Vec<usize> = (0..40).filter(|b| (btn_val >> b) & 1 == 1).collect();
    let knob_tocado = buf[6] == 1;
    let knob_pos = buf[7];
    let strip_pos_1 = buf[10];
    let strip_pos_2 = if buf.len() > 12 { buf[12] } else { 0 };
    println!(
        "  [botão] bits_pressionados={pressionados:?} knob_tocado={knob_tocado} knob_pos={knob_pos} strip1={strip_pos_1} strip2={strip_pos_2}"
    );
}

fn decode_pad(buf: &[u8]) {
    if buf.len() <= 4 {
        return;
    }
    let idx_bruto = buf[1];
    let pad_logico = PAD_ORDER.get(idx_bruto as usize).copied();
    let ctrl = buf[2] & 0xF0;
    let valor = ((buf[2] & 0x0F) as u16) * 256 + buf[3] as u16;
    let estado = match ctrl {
        0x40 => "toque",
        0x10 => "pressão",
        0x20 | 0x30 => "solta",
        _ => "?",
    };
    println!(
        "  [pad]   idx_bruto={idx_bruto} pad_logico={pad_logico:?} estado={estado} valor={valor}"
    );
}

/// Monta o frame de LEDs e acende um pad de vermelho.
/// `pad_logico` é o número lógico (0..15) igual ao devolvido por decode_pad / pymikro.
fn frame_pad_vermelho(pad_logico: u8, intensidade: u8) -> [u8; 91] {
    let mut buf = [0u8; 91];
    buf[0] = 0x80;
    let idx_bruto = PAD_ORDER
        .iter()
        .position(|&v| v == pad_logico)
        .expect("pad lógico 0..15") as usize;
    buf[40 + idx_bruto] = 0x04 * COR_VERMELHO + intensidade;
    buf
}

/// Escreve o frame de LEDs. No Windows, dispositivos HID sem report ID (report ID 0)
/// exigem um byte 0 extra no início do buffer passado a HidDevice::write — é o risco 2
/// do CLAUDE.md. Aqui os reports têm ID explícito (0x80), então o buffer já começa com
/// ele, sem prefixo extra: se a escrita falhar ou não acender nada, tentamos com prefixo.
fn escrever_leds(dev: &hidapi::HidDevice, frame: &[u8; 91]) -> Result<(), String> {
    match dev.write(frame) {
        // hidapi no Windows escreve o tamanho do output report declarado pelo device
        // (maior que os 91 bytes que montamos), não o tamanho do buffer de entrada.
        // Qualquer Ok(_) aqui conta como sucesso da chamada de escrita.
        Ok(n) => {
            println!("[hid] escrita ok, SO reportou {n} bytes (sem prefixo, report ID = primeiro byte)");
            Ok(())
        }
        Err(e) => {
            println!("[hid] escrita sem prefixo falhou ({e}), tentando com prefixo 0x00...");
            let mut com_prefixo = Vec::with_capacity(frame.len() + 1);
            com_prefixo.push(0x00);
            com_prefixo.extend_from_slice(frame);
            match dev.write(&com_prefixo) {
                Ok(n) => {
                    println!("[hid] escrita com prefixo ok, {n} bytes");
                    Ok(())
                }
                Err(e2) => Err(format!("falhou nas duas formas: sem prefixo={e}, com prefixo={e2}")),
            }
        }
    }
}

fn main() {
    println!("=== Spike HID — Maschine Mikro MK3 ===");
    println!("VID=0x{VID:04x} PID=0x{PID:04x}\n");

    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Falha ao iniciar hidapi: {e}");
            std::process::exit(1);
        }
    };

    println!("Interfaces HID encontradas para este VID/PID:");
    let mut achou = false;
    for info in api.device_list().filter(|d| d.vendor_id() == VID && d.product_id() == PID) {
        achou = true;
        println!(
            "  path={:?} interface={} usage_page=0x{:04x} usage=0x{:04x}",
            info.path(),
            info.interface_number(),
            info.usage_page(),
            info.usage()
        );
    }
    if !achou {
        eprintln!("\nNenhuma interface encontrada com VID=0x{VID:04x} PID=0x{PID:04x}.");
        eprintln!("Confira se o Mikro está conectado (risco 1: pode ser preciso fechar");
        eprintln!("o NIHardwareService / NIHostIntegrationAgent se o device não aparecer aqui).");
        std::process::exit(1);
    }
    println!();

    // Risco 1: abrir enquanto o serviço da NI está rodando.
    let dev = match api.open(VID, PID) {
        Ok(d) => {
            println!("[hid] device aberto (convivendo com o driver da NI, se estiver rodando).");
            d
        }
        Err(e) => {
            eprintln!("\nFalha ao abrir o device: {e}");
            eprintln!("Risco 1 provavelmente confirmado: o serviço da NI está segurando o device");
            eprintln!("com acesso exclusivo. Tente parar NIHardwareService / NIHostIntegrationAgent");
            eprintln!("(Gerenciador de Tarefas ou services.msc) e rode de novo.");
            std::process::exit(1);
        }
    };

    if let Ok(m) = dev.get_manufacturer_string() {
        println!("Fabricante: {m:?}");
    }
    if let Ok(p) = dev.get_product_string() {
        println!("Produto: {p:?}");
    }
    if let Ok(s) = dev.get_serial_number_string() {
        println!("Serial: {s:?}");
    }

    // Acende o pad lógico 0 (mesma numeração que aparece em pad_logico nos eventos)
    // de vermelho, intensidade 3 (máxima na tabela de 0..3).
    println!("\nAcendendo pad lógico 0 de vermelho...");
    let frame = frame_pad_vermelho(0, 3);
    match escrever_leds(&dev, &frame) {
        Ok(()) => println!("Confira no aparelho qual pad físico acendeu (risco 2 e mapeamento)."),
        Err(e) => eprintln!("Falha ao escrever LEDs: {e}"),
    }

    println!("\nLendo eventos brutos (aperte pads/botões/knob/strip no aparelho, Ctrl+C para sair)...\n");
    let _ = dev.set_blocking_mode(true);
    let mut buf = [0u8; 1024];
    loop {
        match dev.read_timeout(&mut buf, 2000) {
            Ok(0) => continue,
            Ok(n) => {
                let dados = &buf[..n];
                let hex: Vec<String> = dados.iter().take(16).map(|b| format!("{b:02x}")).collect();
                println!("[report 0x{:02x}] {n} bytes: {} ...", dados[0], hex.join(" "));
                match dados[0] {
                    0x01 => decode_botoes(dados),
                    0x02 => decode_pad(dados),
                    _ => println!("  (report não decodificado neste spike)"),
                }
            }
            Err(e) => {
                eprintln!("[hid] erro de leitura: {e}");
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}
