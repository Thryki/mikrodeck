//! Acesso USB cru ao Maschine Mikro MK3, via libusb (crate `rusb`).
//!
//! Por que existe: a API HID do Windows obriga toda escrita neste aparelho a ter 265 bytes
//! (`OutputReportByteLength`, o tamanho do report da tela). O report de LEDs tem 81 bytes e
//! chega inflado, e o firmware o descarta. Falando USB cru mandamos o tamanho exato.
//!
//! Precisa do driver WinUSB na interface MI_00. Sem ele, o `open()` falha com acesso negado
//! e o programa explica o que fazer.
//!
//! Uso:
//!   usb                 mostra descritores e endpoints
//!   usb leds <cor>      acende todos os pads na cor (0 a 17)
//!   usb tela <on|off>   liga ou desliga a tela inteira
//!   usb ler             imprime os eventos de entrada até Ctrl+C

use rusb::{Direction, TransferType, UsbContext};
use std::time::Duration;

const VID: u16 = 0x17cc;
const PID: u16 = 0x1700;
const INTERFACE: u8 = 0;

/// Report 0x80: 81 bytes no total. 1 de ID, 39 de botões, 16 de pads, 25 de strip.
fn frame_leds(cor: u8, intensidade: u8, brilho_botoes: u8) -> [u8; 81] {
    let mut b = [0u8; 81];
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
    b
}

/// Report 0xE0: 265 bytes. Header de 9 e 256 de bitmap. Metade 0 são as linhas 0 a 15.
fn pacote_tela(metade: u8, preenchimento: u8) -> Vec<u8> {
    let mut p = Vec::with_capacity(265);
    p.push(0xE0);
    p.extend_from_slice(&[0x00, 0x00]);
    p.extend_from_slice(&[metade * 2, 0x00]);
    p.extend_from_slice(&[0x80, 0x00]);
    p.extend_from_slice(&[0x02, 0x00]);
    p.extend_from_slice(&[preenchimento; 256]);
    p
}

fn decode_pad(buf: &[u8]) {
    const ORDEM: [u8; 16] = [12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3];
    if buf.len() <= 4 {
        return;
    }
    let idx = buf[1] as usize;
    let ctrl = buf[2] & 0xF0;
    let valor = ((buf[2] & 0x0F) as u16) * 256 + buf[3] as u16;
    let estado = match ctrl {
        0x40 => "toque",
        0x10 => "pressão",
        0x20 | 0x30 => "solta",
        _ => "?",
    };
    println!(
        "  [pad] bruto={idx} lógico={:?} {estado} valor={valor}",
        ORDEM.get(idx)
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ctx = rusb::Context::new().expect("contexto libusb");

    let dispositivo = ctx
        .devices()
        .expect("listar dispositivos")
        .iter()
        .find(|d| {
            d.device_descriptor()
                .map(|dd| dd.vendor_id() == VID && dd.product_id() == PID)
                .unwrap_or(false)
        });

    let dispositivo = match dispositivo {
        Some(d) => d,
        None => {
            eprintln!("Mikro MK3 não encontrado pelo libusb (VID 0x{VID:04x} PID 0x{PID:04x}).");
            eprintln!("Se ele está conectado, provavelmente a interface MI_00 ainda está no driver HID.");
            std::process::exit(1);
        }
    };

    // Descobre os endpoints de interrupção da interface 0.
    let config = dispositivo.active_config_descriptor().expect("config descriptor");
    let mut ep_entrada = None;
    let mut ep_saida = None;
    let mut tam_max_saida = 0u16;
    for interface in config.interfaces() {
        for desc in interface.descriptors() {
            if desc.interface_number() != INTERFACE {
                continue;
            }
            for ep in desc.endpoint_descriptors() {
                if ep.transfer_type() != TransferType::Interrupt {
                    continue;
                }
                match ep.direction() {
                    Direction::In => ep_entrada = Some(ep.address()),
                    Direction::Out => {
                        ep_saida = Some(ep.address());
                        tam_max_saida = ep.max_packet_size();
                    }
                }
            }
        }
    }
    println!(
        "Endpoints da interface {INTERFACE}: entrada={:?} saída={:?} (pacote máx. de saída {tam_max_saida})",
        ep_entrada.map(|e| format!("0x{e:02x}")),
        ep_saida.map(|e| format!("0x{e:02x}"))
    );

    let handle = match dispositivo.open() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("\nNão consegui abrir o aparelho: {e}");
            eprintln!("A interface MI_00 precisa estar com o driver WinUSB.");
            eprintln!("Troque com o Zadig: selecione \"Maschine Mikro MK3 HID\" (interface 0) e instale WinUSB.");
            std::process::exit(1);
        }
    };
    if let Err(e) = handle.claim_interface(INTERFACE) {
        eprintln!("Não consegui reivindicar a interface {INTERFACE}: {e}");
        std::process::exit(1);
    }
    println!("Aparelho aberto e interface {INTERFACE} reivindicada.\n");

    let tempo = Duration::from_millis(1000);
    let comando = args.first().map(|s| s.as_str()).unwrap_or("info");

    match comando {
        // leds <cor> [intensidade] [brilho_botoes]
        "leds" => {
            let cor: u8 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let inten: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
            let botoes: u8 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(13);
            let saida = ep_saida.expect("endpoint de saída");
            let frame = frame_leds(cor, inten, botoes);
            match handle.write_interrupt(saida, &frame, tempo) {
                Ok(n) => println!(
                    "LEDs: {n} bytes (esperado 81). cor={cor} intensidade={inten} botões={botoes}. Byte do pad = 0x{:02x}",
                    frame[40]
                ),
                Err(e) => println!("LEDs falhou: {e}"),
            }
        }
        // bruto <hex...>: manda exatamente esses bytes, sem completar nada.
        // Com WinUSB podemos usar qualquer report ID, inclusive os que a pilha HID recusava.
        "bruto" => {
            let saida = ep_saida.expect("endpoint de saída");
            let buf: Vec<u8> = args[1..]
                .iter()
                .filter_map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .collect();
            match handle.write_interrupt(saida, &buf, tempo) {
                Ok(n) => println!("bruto: {n} de {} bytes enviados, id=0x{:02x}", buf.len(), buf[0]),
                Err(e) => println!("bruto falhou: {e}"),
            }
        }
        // r81 <cor> <intensidade>: report 0x81 no layout do MK3 grande.
        // 1 byte de ID, 25 LEDs de strip, 16 pads. Total 42 bytes.
        "r81" => {
            let cor: u8 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let inten: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
            let saida = ep_saida.expect("endpoint de saída");
            let v = 0x04 * cor + inten;
            let mut buf = vec![0u8; 42];
            buf[0] = 0x81;
            for i in 1..42 {
                buf[i] = v;
            }
            match handle.write_interrupt(saida, &buf, tempo) {
                Ok(n) => println!("report 0x81: {n} bytes, valor 0x{v:02x}"),
                Err(e) => println!("report 0x81 falhou: {e}"),
            }
        }
        // lerfeature <id_hex> <tamanho>: GET_REPORT de um feature report.
        // Mostra o estado atual guardado no aparelho (brilho, modo, etc).
        "lerfeature" => {
            let id = args
                .get(1)
                .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .unwrap_or(0xd0);
            let tam: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(33);
            let mut buf = vec![0u8; tam];
            // bmRequestType 0xA1 = entrada, classe, interface. bRequest 0x01 = GET_REPORT.
            // wValue alto 0x03 = tipo Feature.
            match handle.read_control(0xA1, 0x01, 0x0300 | id as u16, INTERFACE as u16, &mut buf, tempo) {
                Ok(n) => {
                    let hex: Vec<String> = buf[..n].iter().map(|b| format!("{b:02x}")).collect();
                    println!("feature 0x{id:02x} ({n} bytes): {}", hex.join(" "));
                }
                Err(e) => println!("feature 0x{id:02x}: falhou: {e}"),
            }
        }
        // escrevefeature <hex...>: SET_REPORT de um feature report, bytes exatos com o ID na frente.
        "escrevefeature" => {
            let buf: Vec<u8> = args[1..]
                .iter()
                .filter_map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .collect();
            // bmRequestType 0x21 = saída, classe, interface. bRequest 0x09 = SET_REPORT.
            match handle.write_control(0x21, 0x09, 0x0300 | buf[0] as u16, INTERFACE as u16, &buf, tempo) {
                Ok(n) => println!("SET_REPORT feature 0x{:02x}: {n} bytes", buf[0]),
                Err(e) => println!("SET_REPORT falhou: {e}"),
            }
        }
        // telaarquivo <caminho>: manda um bitmap de 512 bytes (as duas metades) para a tela.
        "telaarquivo" => {
            let dados = std::fs::read(&args[1]).expect("ler bitmap");
            assert_eq!(dados.len(), 512);
            let saida = ep_saida.expect("endpoint de saída");
            for metade in 0..2usize {
                let mut p = Vec::with_capacity(265);
                p.push(0xE0);
                p.extend_from_slice(&[0x00, 0x00]);
                p.extend_from_slice(&[(metade as u8) * 2, 0x00]);
                p.extend_from_slice(&[0x80, 0x00]);
                p.extend_from_slice(&[0x02, 0x00]);
                p.extend_from_slice(&dados[metade * 256..(metade + 1) * 256]);
                match handle.write_interrupt(saida, &p, tempo) {
                    Ok(n) => println!("tela metade {metade}: {n} bytes"),
                    Err(e) => println!("tela metade {metade}: falhou: {e}"),
                }
            }
        }
        // ctrlout <hex...>: manda um OUTPUT report pelo pipe de controle (SET_REPORT tipo Output),
        // com o tamanho exato. É o caminho que a API HID do Windows não conseguia fazer.
        "ctrlout" => {
            let buf: Vec<u8> = args[1..]
                .iter()
                .filter_map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .collect();
            // wValue alto 0x02 = tipo Output
            match handle.write_control(0x21, 0x09, 0x0200 | buf[0] as u16, INTERFACE as u16, &buf, tempo) {
                Ok(n) => println!("SET_REPORT output 0x{:02x}: {n} bytes", buf[0]),
                Err(e) => println!("SET_REPORT output falhou: {e}"),
            }
        }
        // bulk [cor]: manda o frame de LEDs como transferência BULK em vez de interrupção.
        // O openAV-Ctlra manda a tela por bulk neste mesmo endpoint, então vale testar.
        "bulk" => {
            let cor: u8 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let saida = ep_saida.expect("endpoint de saída");
            let mut frame = [0u8; 81];
            frame[0] = 0x80;
            for i in 1..40 {
                frame[i] = 0x7F;
            }
            let v = (cor << 2) | 3;
            for i in 40..81 {
                frame[i] = v;
            }
            match handle.write_bulk(saida, &frame, tempo) {
                Ok(n) => println!("bulk: {n} bytes enviados"),
                Err(e) => println!("bulk falhou: {e}"),
            }
        }
        // duplo [cor]: manda o mesmo frame duas vezes seguidas, caso o firmware use buffer duplo.
        "duplo" => {
            let cor: u8 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let saida = ep_saida.expect("endpoint de saída");
            let mut frame = [0u8; 81];
            frame[0] = 0x80;
            for i in 1..40 {
                frame[i] = 0x7F;
            }
            let v = (cor << 2) | 3;
            for i in 40..81 {
                frame[i] = v;
            }
            for k in 0..4 {
                match handle.write_interrupt(saida, &frame, tempo) {
                    Ok(n) => println!("envio {k}: {n} bytes"),
                    Err(e) => println!("envio {k} falhou: {e}"),
                }
                std::thread::sleep(Duration::from_millis(30));
            }
        }
        // eco [cor]: escreve o frame de LEDs e escuta o eco que o aparelho devolve.
        // O openAV-Ctlra mostra que o aparelho reenvia o frame de LED (81 bytes) no endpoint
        // de entrada. Comparar o eco com o que mandamos diz se o frame foi aceito.
        // Atenção: o buffer de leitura precisa ser maior que 81, senão a transferência estoura.
        "eco" => {
            let cor: u8 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let saida = ep_saida.expect("endpoint de saída");
            let entrada = ep_entrada.expect("endpoint de entrada");
            // Brilho de botão conforme as implementações que funcionam: 0x7f = forte.
            let mut frame = [0u8; 81];
            frame[0] = 0x80;
            for i in 1..40 {
                frame[i] = 0x7F;
            }
            let v = (cor << 2) | 3;
            for i in 40..81 {
                frame[i] = v;
            }
            println!("mandando frame: botões 0x7f, pads e strip 0x{v:02x}");
            match handle.write_interrupt(saida, &frame, tempo) {
                Ok(n) => println!("escrita: {n} bytes"),
                Err(e) => println!("escrita falhou: {e}"),
            }
            println!("\nescutando o eco por 3 segundos (buffer de 512 bytes):");
            let inicio = std::time::Instant::now();
            let mut buf = [0u8; 512];
            let mut vistos: std::collections::BTreeMap<usize, usize> = Default::default();
            while inicio.elapsed().as_secs() < 3 {
                match handle.read_interrupt(entrada, &mut buf, Duration::from_millis(300)) {
                    Ok(n) => {
                        *vistos.entry(n).or_insert(0) += 1;
                        if n == 81 {
                            let igual = buf[..81] == frame[..];
                            let hex: Vec<String> =
                                buf[..12].iter().map(|b| format!("{b:02x}")).collect();
                            println!(
                                "  ECO de 81 bytes: {} | igual ao enviado: {igual}",
                                hex.join(" ")
                            );
                            if !igual {
                                let dif = (0..81).filter(|&i| buf[i] != frame[i]).count();
                                println!("    {dif} bytes diferentes de 81");
                            }
                        }
                    }
                    Err(rusb::Error::Timeout) => {}
                    Err(e) => println!("  erro de leitura: {e}"),
                }
            }
            println!("\ntamanhos de pacote recebidos: {vistos:?}");
        }
        // reset [cor] [segundos]: reinicia o aparelho pelo USB e manda LEDs logo em seguida.
        // O aparelho toca a animação de boot quando reinicia, então pode existir uma janela
        // logo depois do boot em que ele aceita o controle dos LEDs.
        "reset" => {
            let cor: u8 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let segs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(6);
            println!("reiniciando o aparelho...");
            match handle.reset() {
                Ok(()) => println!("reset ok"),
                Err(e) => println!("reset falhou: {e}"),
            }
            let _ = handle.claim_interface(INTERFACE);
            let saida = ep_saida.expect("endpoint de saída");
            let frame = frame_leds(cor, 3, 13);
            let inicio = std::time::Instant::now();
            let (mut ok, mut err) = (0u64, 0u64);
            while inicio.elapsed().as_secs() < segs {
                match handle.write_interrupt(saida, &frame, Duration::from_millis(100)) {
                    Ok(_) => ok += 1,
                    Err(_) => err += 1,
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            println!("depois do reset: {ok} frames aceitos, {err} erros");
        }
        // descriptor: baixa o report descriptor HID por transferência de controle e
        // imprime os bytes crus, para conferir o parser na mão.
        "descriptor" => {
            let mut buf = [0u8; 4096];
            // bmRequestType 0x81 = entrada, padrão, destinatário interface
            // bRequest 0x06 = GET_DESCRIPTOR, wValue 0x2200 = report descriptor
            match handle.read_control(0x81, 0x06, 0x2200, INTERFACE as u16, &mut buf, tempo) {
                Ok(n) => {
                    println!("Report descriptor: {n} bytes\n");
                    for (i, chunk) in buf[..n].chunks(16).enumerate() {
                        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
                        println!("{:04x}: {}", i * 16, hex.join(" "));
                    }
                }
                Err(e) => println!("falhou ao ler o descriptor: {e}"),
            }
        }
        // fluxo <cor> <segundos> [tamanho]: manda o frame de LEDs continuamente a 60 Hz.
        // Testa a hipótese de o firmware apagar os LEDs quando o fluxo de frames para.
        "fluxo" => {
            let cor: u8 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let segs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            let tam: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(81);
            let saida = ep_saida.expect("endpoint de saída");
            let v = 0x04 * cor + 3;
            let mut buf = vec![0u8; tam];
            buf[0] = 0x80;
            for i in 1..40.min(tam) {
                buf[i] = 13;
            }
            for i in 40..tam {
                buf[i] = v;
            }
            // 4º argumento: intervalo em ms entre frames. 0 = o mais rápido possível.
            let intervalo: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(16);
            let inicio = std::time::Instant::now();
            let mut enviados = 0u64;
            let mut erros = 0u64;
            while inicio.elapsed().as_secs() < segs {
                match handle.write_interrupt(saida, &buf, Duration::from_millis(200)) {
                    Ok(_) => enviados += 1,
                    Err(_) => erros += 1,
                }
                if intervalo > 0 {
                    std::thread::sleep(Duration::from_millis(intervalo));
                }
            }
            println!(
                "fluxo de {segs}s: {enviados} frames enviados, {erros} erros, tamanho {tam}, byte do pad 0x{v:02x}"
            );
        }
        // faixa <inicio> <fim> <valor_hex>: preenche uma faixa de offsets, resto zero
        "faixa" => {
            let ini: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(40);
            let fim: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(55);
            let val = args
                .get(3)
                .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .unwrap_or(0x07);
            let saida = ep_saida.expect("endpoint de saída");
            let mut buf = [0u8; 81];
            buf[0] = 0x80;
            for i in ini..=fim.min(80) {
                buf[i] = val;
            }
            match handle.write_interrupt(saida, &buf, tempo) {
                Ok(n) => println!("faixa {ini}..{fim} = 0x{val:02x}, {n} bytes"),
                Err(e) => println!("falhou: {e}"),
            }
        }
        // padoffset <offset> <valor_hex>: só um byte setado, resto zero
        "padoffset" => {
            let off: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(40);
            let val = args
                .get(2)
                .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                .unwrap_or(0x07);
            let saida = ep_saida.expect("endpoint de saída");
            let mut buf = [0u8; 81];
            buf[0] = 0x80;
            buf[off] = val;
            match handle.write_interrupt(saida, &buf, tempo) {
                Ok(n) => println!("offset {off} = 0x{val:02x}, {n} bytes enviados"),
                Err(e) => println!("falhou: {e}"),
            }
        }
        "tela" => {
            let ligada = args.get(1).map(|s| s == "on").unwrap_or(true);
            let saida = ep_saida.expect("endpoint de saída");
            let preenchimento = if ligada { 0x00u8 } else { 0xFF };
            for metade in 0..2u8 {
                let p = pacote_tela(metade, preenchimento);
                match handle.write_interrupt(saida, &p, tempo) {
                    Ok(n) => println!("tela metade {metade}: {n} bytes"),
                    Err(e) => println!("tela metade {metade}: falhou: {e}"),
                }
            }
        }
        "ler" => {
            let entrada = ep_entrada.expect("endpoint de entrada");
            println!("Lendo eventos. Ctrl+C para sair.\n");
            let mut buf = [0u8; 64];
            loop {
                match handle.read_interrupt(entrada, &mut buf, Duration::from_millis(2000)) {
                    Ok(n) => {
                        let d = &buf[..n];
                        let hex: Vec<String> =
                            d.iter().take(12).map(|b| format!("{b:02x}")).collect();
                        println!("[0x{:02x}] {n} bytes: {}", d[0], hex.join(" "));
                        if d[0] == 0x02 {
                            decode_pad(d);
                        }
                    }
                    Err(rusb::Error::Timeout) => {}
                    Err(e) => {
                        eprintln!("erro de leitura: {e}");
                        break;
                    }
                }
            }
        }
        _ => {
            let dd = dispositivo.device_descriptor().expect("descriptor");
            println!("Fabricante id 0x{:04x}, produto id 0x{:04x}", dd.vendor_id(), dd.product_id());
            println!("Interfaces na configuração ativa: {}", config.num_interfaces());
            println!(
                "Corrente pedida pelo aparelho: {} mA, autoalimentado: {}",
                config.max_power(),
                config.self_powered()
            );
            for interface in config.interfaces() {
                for desc in interface.descriptors() {
                    println!(
                        "  interface {} classe 0x{:02x} subclasse 0x{:02x} protocolo 0x{:02x}, {} endpoints",
                        desc.interface_number(),
                        desc.class_code(),
                        desc.sub_class_code(),
                        desc.protocol_code(),
                        desc.num_endpoints()
                    );
                    for ep in desc.endpoint_descriptors() {
                        println!(
                            "    endpoint 0x{:02x} {:?} {:?} pacote máx. {}",
                            ep.address(),
                            ep.direction(),
                            ep.transfer_type(),
                            ep.max_packet_size()
                        );
                    }
                }
            }
        }
    }

    let _ = handle.release_interface(INTERFACE);
}
