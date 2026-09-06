//! Prova a cadeia inteira do som na placa de som de verdade: decodificar,
//! tocar, soltar e cortar.
//!
//! Numa máquina sem saída de áudio o teste passa sem testar nada, e diz isso.
//! Testar o `Animador` com relógio falso já cobre a lógica; o que este arquivo
//! cobre é o que só falha no hardware.

use motor::som::envelope::Envelope;
use motor::som::{ModoDisparo, Saida, Tocador};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Escreve um WAV de um segundo, 440 Hz, para não depender de arquivo no repo.
fn wav(nome: &str, segundos: f32) -> PathBuf {
    let caminho = std::env::temp_dir().join(format!("mikrodeck-som-{nome}.wav"));
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&caminho, spec).unwrap();
    let total = (44100.0 * segundos) as u32;
    for i in 0..total {
        let t = i as f32 / 44100.0;
        // Volume baixo: o teste roda na maquina do dono do projeto.
        let a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.05;
        w.write_sample((a * i16::MAX as f32) as i16).unwrap();
    }
    w.finalize().unwrap();
    caminho
}

fn espera_ate<F: Fn() -> bool>(limite: Duration, pronto: F) -> Duration {
    let inicio = Instant::now();
    while inicio.elapsed() < limite && !pronto() {
        std::thread::sleep(Duration::from_millis(10));
    }
    inicio.elapsed()
}

#[test]
fn toca_um_arquivo_ate_o_fim_na_placa_de_som() {
    let saida = Saida::nova();
    if !saida.disponivel() {
        eprintln!("sem placa de som: teste pulado");
        return;
    }
    let caminho = wav("ate-o-fim", 0.4);
    let voz = saida
        .tocar(&caminho, 0.5, Envelope::default())
        .expect("devia tocar");
    // Ela nao pode acabar antes da hora, nem ficar presa depois.
    assert!(!voz.acabou(), "acabou antes de comecar");
    let levou = espera_ate(Duration::from_secs(3), || voz.acabou());
    assert!(voz.acabou(), "nao acabou em 3 s");
    assert!(
        levou >= Duration::from_millis(250),
        "acabou rapido demais: {levou:?}"
    );
    let _ = std::fs::remove_file(&caminho);
}

#[test]
fn soltar_no_modo_segurando_encurta_o_som() {
    let mut tocador = Tocador::novo();
    if !tocador.saida().disponivel() {
        eprintln!("sem placa de som: teste pulado");
        return;
    }
    // Cinco segundos de audio, mas o pad e solto em 200 ms com liberacao de 100.
    let caminho = wav("segurando", 5.0);
    let envelope = Envelope {
        ataque_ms: 0,
        decaimento_ms: 0,
        sustentacao: 1.0,
        liberacao_ms: 100,
    };
    tocador
        .tocar(3, &caminho, 0.5, envelope, ModoDisparo::Segurando)
        .expect("devia tocar");
    assert_eq!(tocador.tocando(), 1);
    std::thread::sleep(Duration::from_millis(200));
    tocador.soltar(3);
    let levou = espera_ate(Duration::from_secs(3), || tocador.tocando() == 0);
    assert_eq!(tocador.tocando(), 0, "o som nao parou depois de soltar");
    assert!(
        levou < Duration::from_secs(2),
        "soltar devia cortar bem antes dos 5 s: {levou:?}"
    );
    let _ = std::fs::remove_file(&caminho);
}

#[test]
fn soltar_no_modo_ate_o_fim_nao_para_o_som() {
    let mut tocador = Tocador::novo();
    if !tocador.saida().disponivel() {
        eprintln!("sem placa de som: teste pulado");
        return;
    }
    let caminho = wav("ate-o-fim-solto", 1.0);
    tocador
        .tocar(7, &caminho, 0.5, Envelope::default(), ModoDisparo::AteOFim)
        .expect("devia tocar");
    std::thread::sleep(Duration::from_millis(100));
    tocador.soltar(7);
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        tocador.tocando(),
        1,
        "no modo ate o fim, soltar o pad nao pode cortar o som"
    );
    let _ = std::fs::remove_file(&caminho);
}

#[test]
fn pausar_corta_tudo_na_hora() {
    let mut tocador = Tocador::novo();
    if !tocador.saida().disponivel() {
        eprintln!("sem placa de som: teste pulado");
        return;
    }
    let caminho = wav("cortar", 5.0);
    for pad in [1u8, 2, 3] {
        tocador
            .tocar(pad, &caminho, 0.3, Envelope::default(), ModoDisparo::AteOFim)
            .expect("devia tocar");
    }
    assert_eq!(tocador.tocando(), 3);
    tocador.cortar_tudo();
    assert_eq!(tocador.tocando(), 0, "pausar tinha que calar tudo");
    let _ = std::fs::remove_file(&caminho);
}

#[test]
fn apertar_de_novo_recomeca_em_vez_de_empilhar() {
    let mut tocador = Tocador::novo();
    if !tocador.saida().disponivel() {
        eprintln!("sem placa de som: teste pulado");
        return;
    }
    let caminho = wav("recomeca", 3.0);
    for _ in 0..4 {
        tocador
            .tocar(9, &caminho, 0.3, Envelope::default(), ModoDisparo::AteOFim)
            .expect("devia tocar");
    }
    assert_eq!(tocador.tocando(), 1, "empilhou vozes no mesmo pad");
    tocador.cortar_tudo();
    let _ = std::fs::remove_file(&caminho);
}
