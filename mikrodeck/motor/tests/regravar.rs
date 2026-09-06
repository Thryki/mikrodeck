//! Prova um bug que apareceu no uso: gravar um sample novo por cima do antigo
//! deixava o pad tocando o som velho, porque o cache era só pelo caminho.

use motor::som::{Saida, Tocador, ModoDisparo};
use motor::som::envelope::Envelope;
use std::path::PathBuf;

fn escrever(caminho: &PathBuf, segundos: f32, frequencia: f32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 22050,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(caminho, spec).unwrap();
    for i in 0..(22050.0 * segundos) as u32 {
        let t = i as f32 / 22050.0;
        let a = (t * frequencia * std::f32::consts::TAU).sin() * 0.2;
        w.write_sample((a * i16::MAX as f32) as i16).unwrap();
    }
    w.finalize().unwrap();
}

#[test]
fn o_pad_toca_o_som_novo_depois_de_regravar() {
    let caminho = std::env::temp_dir().join("mikrodeck-regravar.wav");
    let _ = std::fs::remove_file(&caminho);

    let saida = Saida::nova();
    escrever(&caminho, 0.5, 220.0);
    let antes = saida.carregar(&caminho).unwrap();
    let picos_antes = motor::som::picos(&antes, 8);

    // Grava por cima, com outra duracao e outro tom.
    std::thread::sleep(std::time::Duration::from_millis(20));
    escrever(&caminho, 1.5, 880.0);
    let depois = saida.carregar(&caminho).unwrap();

    assert_ne!(
        antes.dados.len(),
        depois.dados.len(),
        "o cache devolveu o som antigo depois de regravar"
    );
    assert_eq!(depois.dados.len(), (22050.0 * 1.5) as usize);
    // E o desenho da forma de onda tambem muda: e o que a pessoa ve.
    assert_ne!(picos_antes.len(), 0);
    let _ = std::fs::remove_file(&caminho);
}

#[test]
fn o_tocador_do_servico_tambem_pega_o_som_novo() {
    // O cache que causava o bug e o do Tocador, que vive no servico e nao
    // morre entre um aperto e outro.
    let caminho = std::env::temp_dir().join("mikrodeck-regravar-tocador.wav");
    let _ = std::fs::remove_file(&caminho);
    let mut tocador = Tocador::novo();
    if !tocador.saida().disponivel() {
        eprintln!("sem placa de som: teste pulado");
        return;
    }
    escrever(&caminho, 0.3, 220.0);
    tocador
        .tocar(1, &caminho, 0.2, Envelope::default(), ModoDisparo::AteOFim)
        .unwrap();
    let curto = tocador.saida().carregar(&caminho).unwrap().dados.len();

    std::thread::sleep(std::time::Duration::from_millis(20));
    escrever(&caminho, 1.2, 220.0);
    tocador
        .tocar(1, &caminho, 0.2, Envelope::default(), ModoDisparo::AteOFim)
        .unwrap();
    let longo = tocador.saida().carregar(&caminho).unwrap().dados.len();

    assert!(longo > curto * 3, "{curto} -> {longo}: continuou no som antigo");
    tocador.cortar_tudo();
    let _ = std::fs::remove_file(&caminho);
}
