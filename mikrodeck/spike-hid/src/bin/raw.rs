//! Escrita HID sem o hidapi no meio, chamando CreateFileW e WriteFile direto.
//!
//! Por que: o hidapi, no Windows, sempre preenche o buffer até OutputReportByteLength
//! (265 bytes neste aparelho, o tamanho do report da tela 0xE0) antes de chamar
//! WriteFile. A tela funciona justamente porque já tem 265 bytes. O report de LEDs
//! (0x80) tem 81 bytes e chega inflado ao aparelho, que parece descartá-lo.
//!
//! Aqui testamos, na ordem:
//! 1. WriteFile com os 81 bytes exatos do report 0x80.
//! 2. WriteFile com 265 bytes e todos os dados em 0xFF, para descartar erro de
//!    codificação de cor (se nada acender nem com tudo em 0xFF, o problema é o tamanho).
//! 3. HidD_SetOutputReport com 81 bytes, para registrar o erro exato do Windows.

use std::ffi::c_void;
use std::ptr::null_mut;
use std::thread::sleep;
use std::time::Duration;
use windows_sys::Win32::Devices::HumanInterfaceDevice::HidD_SetOutputReport;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING, WriteFile,
};

const VID: u16 = 0x17cc;
const PID: u16 = 0x1700;

/// Monta o report 0x80 no tamanho declarado pelo descriptor: 81 bytes.
/// Layout: byte 0 = ID, 1..39 botões, 40..55 pads, 56..80 strip.
fn frame_leds(tamanho: usize, cor: u8, intensidade: u8, brilho_botoes: u8) -> Vec<u8> {
    let mut b = vec![0u8; tamanho];
    b[0] = 0x80;
    let v = 0x04 * cor + intensidade;
    for i in 1..40.min(tamanho) {
        b[i] = brilho_botoes;
    }
    for i in 0..16 {
        if 40 + i < tamanho {
            b[40 + i] = v;
        }
    }
    for i in 0..25 {
        if 56 + i < tamanho {
            b[56 + i] = v;
        }
    }
    b
}

fn caminho_do_device() -> Option<Vec<u16>> {
    let api = hidapi::HidApi::new().ok()?;
    let info = api
        .device_list()
        .find(|d| d.vendor_id() == VID && d.product_id() == PID)?;
    let bytes = info.path().to_bytes();
    let texto = std::str::from_utf8(bytes).ok()?;
    println!("Caminho do device: {texto}");
    let mut wide: Vec<u16> = texto.encode_utf16().collect();
    wide.push(0);
    Some(wide)
}

fn escrever(handle: *mut c_void, buf: &[u8], rotulo: &str) {
    let mut escritos: u32 = 0;
    let ok = unsafe {
        WriteFile(
            handle,
            buf.as_ptr(),
            buf.len() as u32,
            &mut escritos,
            null_mut(),
        )
    };
    if ok != 0 {
        println!("  {rotulo}: WriteFile OK, {escritos} bytes de {} enviados", buf.len());
    } else {
        let erro = unsafe { GetLastError() };
        let explicacao = match erro {
            87 => " (ERROR_INVALID_PARAMETER: o Windows exige o tamanho do maior report)",
            5 => " (ERROR_ACCESS_DENIED: handle sem permissão de escrita)",
            _ => "",
        };
        println!("  {rotulo}: WriteFile falhou, erro {erro}{explicacao}");
    }
}

fn main() {
    let caminho = match caminho_do_device() {
        Some(c) => c,
        None => {
            eprintln!("Mikro MK3 não encontrado.");
            return;
        }
    };

    let handle = unsafe {
        CreateFileW(
            caminho.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let erro = unsafe { GetLastError() };
        eprintln!("CreateFileW falhou com erro {erro}. Sem handle de escrita, nada feito.");
        return;
    }
    println!("Handle aberto com leitura e escrita.\n");

    println!("=== Teste 1: WriteFile com os 81 bytes exatos, vermelho ===");
    escrever(handle, &frame_leds(81, 1, 3, 13), "81 bytes");
    println!("  Olhe os pads (5s)...");
    sleep(Duration::from_secs(5));

    println!("\n=== Teste 2: WriteFile com 265 bytes, TODOS os dados em 0xFF ===");
    let mut tudo = vec![0xFFu8; 265];
    tudo[0] = 0x80;
    escrever(handle, &tudo, "265 bytes 0xFF");
    println!("  Se nada acender nem assim, o problema é o tamanho, não a cor (5s)...");
    sleep(Duration::from_secs(5));

    println!("\n=== Teste 3: HidD_SetOutputReport com 81 bytes ===");
    let mut f = frame_leds(81, 7, 3, 13);
    let ok = unsafe {
        HidD_SetOutputReport(handle, f.as_mut_ptr() as *const c_void, f.len() as u32)
    };
    if ok {
        println!("  HidD_SetOutputReport OK");
    } else {
        println!("  HidD_SetOutputReport falhou, erro {}", unsafe { GetLastError() });
    }
    println!("  Olhe os pads (5s)...");
    sleep(Duration::from_secs(5));

    println!("\n=== Limpando (265 bytes zerados) ===");
    let mut limpar = vec![0u8; 265];
    limpar[0] = 0x80;
    escrever(handle, &limpar, "limpar");

    unsafe {
        CloseHandle(handle);
    }
    println!("\nFim.");
}
