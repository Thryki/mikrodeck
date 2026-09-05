//! Envia um pacote HID arbitrário para o Mikro MK3, para experimentação.
//!
//! Uso:
//!   envia <bytes em hex>              manda exatamente esses bytes
//!   envia --leds <cor> <intensidade>  monta o report 0x80 com todos os pads na cor
//!   envia --preenche <id> <valor>     report <id> com todos os dados no mesmo valor
//!   envia --byte <id> <offset> <valor>  report <id> com um único byte setado
//!
//! Opção --tam <n> define o tamanho do buffer entregue ao hidapi (padrão 81 para
//! o report 0x80). O hidapi completa com zeros até 265, que é o que o Windows exige.

use hidapi::HidApi;

const VID: u16 = 0x17cc;
const PID: u16 = 0x1700;

fn hex(s: &str) -> Option<u8> {
    u8::from_str_radix(s.trim_start_matches("0x"), 16).ok()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("faltam argumentos");
        std::process::exit(2);
    }

    // --tam <n> em qualquer posição
    let mut tam = 81usize;
    let mut limpos: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--tam" && i + 1 < args.len() {
            tam = args[i + 1].parse().unwrap_or(81);
            i += 2;
        } else {
            limpos.push(args[i].clone());
            i += 1;
        }
    }

    // --init: reproduz exatamente a sequência que o serviço da Native Instruments faz ao
    // assumir o aparelho, capturada com USBPcap. São só LEITURAS, nenhuma escrita de comando:
    //   GET_REPORT feature 0xf8, GET_REPORT feature 0xd0,
    //   GET_REPORT input 0x01 (pelo canal de controle), GET_REPORT feature 0xd0.
    // Depois disso o serviço escreve o frame de LED normalmente.
    // --esperar <cor> <segundos>: fica tentando abrir o aparelho e dispara a sequência de
    // inicialização no instante em que ele aparece. Testa a hipótese de existir uma janela
    // de tempo curta logo após a energização: o Maschine 2 escreve 226 ms depois da
    // enumeração, e nossos testes anteriores demoravam segundos por causa do PowerShell.
    if limpos[0] == "--esperar" {
        // Primeiro espera o aparelho SUMIR, senão dispara no que já está conectado.
        println!("aguardando você DESCONECTAR o cabo...");
        let t_sumir = std::time::Instant::now();
        loop {
            let presente = HidApi::new()
                .map(|api| {
                    api.device_list()
                        .any(|d| d.vendor_id() == VID && d.product_id() == PID)
                })
                .unwrap_or(false);
            if !presente {
                println!("aparelho sumiu. Agora RECONECTE o cabo.");
                break;
            }
            if t_sumir.elapsed().as_secs() > 120 {
                println!("desisti: não vi a desconexão em 2 minutos");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        // 5º argumento: segundos a esperar DEPOIS que o aparelho sumir, antes de sequer
        // tentar abrir. Serve para testar se o que importa é ganhar a corrida da abertura.
        let atrasar_abertura: u64 = limpos.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        if atrasar_abertura > 0 {
            println!("esperando {atrasar_abertura}s de propósito antes de tentar abrir...");
            std::thread::sleep(std::time::Duration::from_secs(atrasar_abertura));
        }

        let inicio_espera = std::time::Instant::now();
        loop {
            // Recria a API a cada tentativa: o hidapi cacheia a lista de dispositivos.
            if let Ok(api) = HidApi::new() {
                if let Ok(dev) = api.open(VID, PID) {
                    let ms = inicio_espera.elapsed().as_millis();
                    println!("aparelho aberto após {ms} ms de espera.");

                    // 4º argumento: atraso em ms antes de disparar. Serve para medir o
                    // tamanho da janela de tempo em que o aparelho ainda aceita o comando.
                    let atraso: u64 = limpos.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
                    if atraso > 0 {
                        println!("esperando {atraso} ms de propósito antes de escrever...");
                        std::thread::sleep(std::time::Duration::from_millis(atraso));
                    }
                    let t = std::time::Instant::now();

                    // 6º argumento: "nu" pula as 4 leituras e escreve direto. Testa se a
                    // sequência de inicialização importa ou se o aparelho já está acordado.
                    let nu = limpos.get(5).map(|s| s == "nu").unwrap_or(false);
                    if nu {
                        println!("modo nu: sem leituras, escrevendo direto");
                    } else {
                        let mut f8 = [0u8; 11];
                        f8[0] = 0xF8;
                        let _ = dev.get_feature_report(&mut f8);
                        let mut d0 = [0u8; 33];
                        d0[0] = 0xD0;
                        let _ = dev.get_feature_report(&mut d0);
                        let mut inp = [0u8; 14];
                        inp[0] = 0x01;
                        let _ = dev.get_input_report(&mut inp);
                        let mut d0b = [0u8; 33];
                        d0b[0] = 0xD0;
                        let _ = dev.get_feature_report(&mut d0b);
                    }

                    // Frame igual ao do Maschine 2: botões 0x7c, com as setas (offsets 9 e 10)
                    // apagadas, e os pads na cor pedida.
                    let cor: u8 = limpos.get(1).and_then(|s| s.parse().ok()).unwrap_or(11);
                    let mut frame = vec![0u8; 81];
                    frame[0] = 0x80;
                    for i in 1..40 {
                        frame[i] = 0x7C;
                    }
                    frame[9] = 0x00;
                    frame[10] = 0x00;
                    for i in 40..56 {
                        frame[i] = (cor << 2) | 3;
                    }
                    let r = dev.write(&frame);
                    println!(
                        "sequência completa em {} ms. escrita: {:?}",
                        t.elapsed().as_millis(),
                        r
                    );

                    let segundos: u64 = limpos.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
                    let ini = std::time::Instant::now();
                    while ini.elapsed().as_secs() < segundos {
                        let _ = dev.write(&frame);
                        std::thread::sleep(std::time::Duration::from_millis(30));
                    }
                    println!("fim");
                    return;
                }
            }
            if inicio_espera.elapsed().as_secs() > 120 {
                println!("desisti depois de 2 minutos");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    if limpos[0] == "--init" {
        let api = HidApi::new().expect("hidapi");
        let dev = api.open(VID, PID).expect("abrir Mikro MK3");

        // Igual ao que o Maschine 2 faz antes das leituras de feature: ler as strings
        // padrão do dispositivo (fabricante, produto, serial) via GET_DESCRIPTOR.
        if let Ok(s) = dev.get_manufacturer_string() {
            println!("fabricante: {s:?}");
        }
        if let Ok(s) = dev.get_product_string() {
            println!("produto: {s:?}");
        }
        if let Ok(s) = dev.get_serial_number_string() {
            println!("serial: {s:?}");
        }

        let mut f8 = [0u8; 11];
        f8[0] = 0xF8;
        match dev.get_feature_report(&mut f8) {
            Ok(n) => println!("feature 0xf8: {n} bytes {:02x?}", &f8[..n.min(11)]),
            Err(e) => println!("feature 0xf8 falhou: {e}"),
        }

        let mut d0 = [0u8; 33];
        d0[0] = 0xD0;
        match dev.get_feature_report(&mut d0) {
            Ok(n) => println!("feature 0xd0: {n} bytes {:02x?}", &d0[..n.min(12)]),
            Err(e) => println!("feature 0xd0 falhou: {e}"),
        }

        // Esta é a leitura que nunca tínhamos feito: input report 0x01 pelo canal de controle.
        let mut inp = [0u8; 14];
        inp[0] = 0x01;
        match dev.get_input_report(&mut inp) {
            Ok(n) => println!("input 0x01 (controle): {n} bytes {:02x?}", &inp[..n.min(14)]),
            Err(e) => println!("input 0x01 falhou: {e}"),
        }

        let mut d0b = [0u8; 33];
        d0b[0] = 0xD0;
        match dev.get_feature_report(&mut d0b) {
            Ok(n) => println!("feature 0xd0 de novo: {n} bytes"),
            Err(e) => println!("feature 0xd0 de novo falhou: {e}"),
        }

        // Nova hipótese: talvez o firmware só aceite a escrita se o host já estiver com
        // uma leitura pendente no endpoint de entrada. Dispara algumas leituras curtas
        // (não bloqueantes de verdade, mas iniciam o polling) antes de escrever.
        let _ = dev.set_blocking_mode(false);
        let mut lixo = [0u8; 64];
        for _ in 0..5 {
            let _ = dev.read_timeout(&mut lixo, 10);
        }
        println!("polling de entrada iniciado");

        // Frame de LED igual ao que a NI manda: botões em 0x7c, pads numa cor visível.
        let mut frame = vec![0u8; 81];
        frame[0] = 0x80;
        for i in 1..40 {
            frame[i] = 0x7C;
        }
        let cor: u8 = limpos.get(1).and_then(|s| s.parse().ok()).unwrap_or(11);
        for i in 40..56 {
            frame[i] = (cor << 2) | 3;
        }
        for i in 56..81 {
            frame[i] = (cor << 2) | 3;
        }
        match dev.write(&frame) {
            Ok(n) => println!("frame de LED enviado (SO reportou {n})"),
            Err(e) => println!("frame de LED falhou: {e}"),
        }

        // Segura o handle aberto e reenvia o frame periodicamente. Duas hipóteses de uma vez:
        // que o aparelho apague os LEDs quando o último handle fecha, e que ele precise de
        // um fluxo contínuo. O serviço da NI mantém o aparelho aberto o tempo todo.
        let segundos: u64 = limpos.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        if segundos > 0 {
            println!("segurando o aparelho aberto por {segundos}s, lendo e reenviando...");
            let inicio = std::time::Instant::now();
            let mut buf2 = [0u8; 64];
            let mut n_leituras = 0u64;
            while inicio.elapsed().as_secs() < segundos {
                if let Ok(n) = dev.read_timeout(&mut buf2, 5) {
                    if n > 0 {
                        n_leituras += 1;
                    }
                }
                let _ = dev.write(&frame);
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            println!("fim ({n_leituras} leituras não vazias no período)");
        }
        return;
    }

    // --tela-arquivo <caminho>: manda um bitmap de 512 bytes para a tela.
    // Os 512 bytes são as duas metades já empacotadas (128x16 cada, 1 bit por pixel,
    // 4 bytes por coluna, bit 1 = pixel apagado).
    if limpos[0] == "--tela-arquivo" {
        let dados = std::fs::read(&limpos[1]).expect("ler o bitmap");
        assert_eq!(dados.len(), 512, "o bitmap precisa ter 512 bytes");
        let api = HidApi::new().expect("hidapi");
        let dev = api.open(VID, PID).expect("abrir Mikro MK3");
        for metade in 0..2usize {
            let mut p = Vec::with_capacity(265);
            p.push(0xE0);
            p.extend_from_slice(&[0x00, 0x00]);
            p.extend_from_slice(&[(metade as u8) * 2, 0x00]);
            p.extend_from_slice(&[0x80, 0x00]);
            p.extend_from_slice(&[0x02, 0x00]);
            p.extend_from_slice(&dados[metade * 256..(metade + 1) * 256]);
            match dev.write(&p) {
                Ok(n) => println!("tela metade {metade}: ok ({n})"),
                Err(e) => println!("tela metade {metade}: falhou: {e}"),
            }
        }
        return;
    }

    // --tela <on|off> usa o report 0xE0, que já tem 265 bytes e comprovadamente funciona.
    // Serve de controle: se a tela responde e os LEDs não, o problema é do report 0x80.
    if limpos[0] == "--tela" {
        let ligada = limpos.get(1).map(|s| s == "on").unwrap_or(true);
        let preenchimento = if ligada { 0xFFu8 } else { 0x00 };
        let api = HidApi::new().expect("hidapi");
        let dev = api.open(VID, PID).expect("abrir Mikro MK3");
        for metade in 0..2u8 {
            let mut p = Vec::with_capacity(265);
            p.push(0xE0);
            p.extend_from_slice(&[0x00, 0x00]);
            p.extend_from_slice(&[metade * 2, 0x00]);
            p.extend_from_slice(&[0x80, 0x00]);
            p.extend_from_slice(&[0x02, 0x00]);
            p.extend_from_slice(&[preenchimento; 256]);
            match dev.write(&p) {
                Ok(n) => println!("tela metade {metade}: ok ({n})"),
                Err(e) => println!("tela metade {metade}: falhou: {e}"),
            }
        }
        return;
    }

    // --faixa <inicio> <fim> <valor>: report 0x80 com uma faixa de bytes preenchida.
    // Serve para descobrir quais offsets do report controlam o quê.
    let buf: Vec<u8> = match limpos[0].as_str() {
        "--faixa" => {
            let ini: usize = limpos[1].parse().unwrap_or(1);
            let fim: usize = limpos[2].parse().unwrap_or(80);
            let val = hex(&limpos[3]).unwrap_or(0x0D);
            let mut b = vec![0u8; tam];
            b[0] = 0x80;
            for k in ini..=fim.min(tam - 1) {
                b[k] = val;
            }
            b
        }
        "--leds" => {
            let cor = limpos[1].parse::<u8>().unwrap_or(1);
            let inten = limpos[2].parse::<u8>().unwrap_or(3);
            let mut b = vec![0u8; tam];
            b[0] = 0x80;
            let v = 0x04 * cor + inten;
            for k in 0..16 {
                if 40 + k < tam {
                    b[40 + k] = v;
                }
            }
            for k in 0..25 {
                if 56 + k < tam {
                    b[56 + k] = v;
                }
            }
            b
        }
        "--preenche" => {
            let id = hex(&limpos[1]).unwrap_or(0x80);
            let val = hex(&limpos[2]).unwrap_or(0xFF);
            let mut b = vec![val; tam];
            b[0] = id;
            b
        }
        "--byte" => {
            let id = hex(&limpos[1]).unwrap_or(0x80);
            let off: usize = limpos[2].parse().unwrap_or(1);
            let val = hex(&limpos[3]).unwrap_or(0xFF);
            let mut b = vec![0u8; tam];
            b[0] = id;
            if off < tam {
                b[off] = val;
            }
            b
        }
        _ => limpos.iter().filter_map(|s| hex(s)).collect(),
    };

    let api = HidApi::new().expect("hidapi");
    let dev = api.open(VID, PID).expect("abrir Mikro MK3");
    match dev.write(&buf) {
        Ok(n) => println!(
            "enviado: id=0x{:02x} buffer={} bytes, SO reportou {n}",
            buf[0],
            buf.len()
        ),
        Err(e) => println!("falhou: {e}"),
    }
}
