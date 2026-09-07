//! Os modos de luz, como funções puras: recebem o tempo e devolvem um quadro.
//!
//! Nada aqui sabe de HID nem de relógio. Isso é o que deixa cada modo testável
//! e o orçamento de escritas provável por conta, em `docs/animacoes.md`.

use super::{Quadro, APAGADO};
use crate::hid::Cor;

/// Roda de cores da luz automática. Oito matizes espaçados; os dezoito do
/// aparelho são desiguais demais para girar um a um.
pub const RODA: [Cor; 8] = [
    Cor::Vermelho,
    Cor::Laranja,
    Cor::Amarelo,
    Cor::Verde,
    Cor::Ciano,
    Cor::Azul,
    Cor::Violeta,
    Cor::Magenta,
];

/// O caminho do cometa: sobe a coluna da
/// esquerda, atravessa a fileira de cima, desce a da direita, volta pela de
/// baixo e fecha pelo centro.
pub const CONTORNO: [u8; 16] = [1, 5, 9, 13, 14, 15, 16, 12, 8, 4, 3, 2, 6, 10, 11, 7];

/// Os quatro pads do meio. A borda é o resto.
pub const CENTRO: [u8; 4] = [6, 7, 10, 11];

/// As quatro colunas, de baixo para cima, e a ordem de varredura que vai e
/// volta sem saltar da última para a primeira.
pub const COLUNAS: [[u8; 4]; 4] = [[1, 5, 9, 13], [2, 6, 10, 14], [3, 7, 11, 15], [4, 8, 12, 16]];
pub const VARREDURA: [usize; 6] = [0, 1, 2, 3, 2, 1];

/// Comprimento do rastro do cometa, em pads. Atras da cabeca a luz demora a
/// morrer; na frente ela ja comeca a nascer, mas bem mais curto. E a diferenca
/// entre os dois que da a direcao do movimento.
const RASTRO_TRAS: f32 = 3.2;
const RASTRO_FRENTE: f32 = 0.9;

/// Quanto um pad acende, de 0 a 1, pela distancia ate a cabeca. Distancia
/// positiva e "ficou para tras". E a rampa disso que faz a luz escorregar de um
/// pad para o outro em vez de pular: com a cabeca no meio do caminho, os dois
/// vizinhos dividem o brilho.
fn intensidade(distancia: f32) -> f32 {
    let comprimento = if distancia >= 0.0 { RASTRO_TRAS } else { RASTRO_FRENTE };
    (1.0 - distancia.abs() / comprimento).max(0.0)
}

/// Distancia de `k` ate a cabeca num ciclo de `n`, pelo caminho mais curto.
fn distancia_ciclica(cabeca: f32, k: f32, n: f32) -> f32 {
    let mut d = cabeca - k;
    if d > n / 2.0 {
        d -= n;
    } else if d < -n / 2.0 {
        d += n;
    }
    d
}

/// Quadro todo apagado.
pub fn vazio() -> Quadro {
    [APAGADO; 16]
}

/// Cor da roda para uma volta, ou a cor fixa se houver.
pub fn matiz(fixa: Option<Cor>, volta: u64) -> Cor {
    fixa.unwrap_or(RODA[(volta % RODA.len() as u64) as usize])
}

/// Nível da respiração num instante do ciclo.
///
/// Degraus do meio curtos e extremos longos, como uma senoide amostrada. Cada
/// tabela é para um teto de brilho, começa do alto para emendar com a página
/// sem costura, e soma 4000 ms. Toda duração é múltiplo de 50 ms, então o
/// ritmo rápido (metade) cai em múltiplos de 25 ms e bate com o laço.
pub fn nivel_respiracao(brilho: u8, ms_no_ciclo: u64, periodo_ms: u64) -> u8 {
    let tabela: &[(u8, u64)] = match brilho.min(3) {
        3 => &[(3, 900), (2, 300), (1, 400), (0, 1700), (1, 400), (2, 300)],
        2 => &[(2, 1000), (1, 500), (0, 2000), (1, 500)],
        1 => &[(1, 1500), (0, 2500)],
        _ => return 0,
    };
    // A tabela é para 4000 ms; outros períodos esticam ou encolhem na proporção.
    let escala = periodo_ms.max(1) as f64 / 4000.0;
    let mut t = (ms_no_ciclo % periodo_ms.max(1)) as f64;
    for (nivel, dur) in tabela {
        let dur = *dur as f64 * escala;
        if t < dur {
            return *nivel;
        }
        t -= dur;
    }
    tabela[0].0
}

/// Respiração: a página inteira sobe e desce de brilho. Só os pads com
/// controle participam; os vazios ficam apagados, para o desenho da página se
/// manter no escuro. Nunca chega em apagado: o vale é o fraco.
pub fn respiracao(
    repouso: &Quadro,
    brilho: u8,
    cor: Option<Cor>,
    ms_no_ciclo: u64,
    periodo_ms: u64,
) -> Quadro {
    let nivel = nivel_respiracao(brilho, ms_no_ciclo, periodo_ms);
    let mut q = vazio();
    for (i, (cor_pad, _)) in repouso.iter().enumerate() {
        if *cor_pad == Cor::Apagado {
            continue;
        }
        q[i] = (cor.unwrap_or(*cor_pad), nivel);
    }
    q
}

/// Contorno: um cometa percorre a borda e fecha no centro, com rastro.
///
/// `pos` e a posicao da cabeca em pads, e vem fracionaria: com a cabeca em 4,5
/// os pads 4 e 5 dividem o brilho, e e isso que faz o movimento parecer
/// continuo mesmo com so quatro niveis de brilho no aparelho.
/// A cor anda um passo na roda a cada volta.
pub fn contorno(pos: f32, brilho: u8, cor: Option<Cor>) -> Quadro {
    let n = CONTORNO.len() as f32;
    let volta = (pos / n).floor().max(0.0) as u64;
    let cabeca = pos.rem_euclid(n);
    let cor = matiz(cor, volta);
    let teto = brilho.min(3);
    let mut q = vazio();
    for (k, pad) in CONTORNO.iter().enumerate() {
        let d = distancia_ciclica(cabeca, k as f32, n);
        let nivel = (intensidade(d) * teto as f32).round() as u8;
        // A cabeca acende mesmo com teto 0, senao o brilho 1 do aparelho
        // deixaria a animacao invisivel.
        if nivel > 0 || d.abs() < 0.5 {
            q[*pad as usize - 1] = (cor, nivel.min(teto));
        }
    }
    q
}

/// Colunas: uma coluna varre da esquerda para a direita e volta, sem salto.
///
/// `pos` tambem e fracionaria, pelo mesmo motivo do Contorno: entre duas
/// colunas as duas ficam meio acesas, e a onda desliza. A cor anda na roda a
/// cada duas varreduras.
pub fn colunas(pos: f32, brilho: u8, cor: Option<Cor>) -> Quadro {
    let n = VARREDURA.len() as f32;
    let volta = (pos / (2.0 * n)).floor().max(0.0) as u64;
    let cor = matiz(cor, volta);
    // Triangular: 0 ate 3 e de volta a 0, sem salto e sem canto duro.
    let p = pos.rem_euclid(n);
    let x = 3.0 - (3.0 - p).abs();
    let teto = brilho.min(3);
    let mut q = vazio();
    for (coluna, pads) in COLUNAS.iter().enumerate() {
        // Simetrico: num vai e volta nao ha frente nem tras fixos.
        let nivel = ((1.0 - (coluna as f32 - x).abs() / 1.6).max(0.0) * teto as f32).round() as u8;
        let acende = nivel > 0 || (coluna as f32 - x).abs() < 0.5;
        if acende {
            for pad in pads {
                q[*pad as usize - 1] = (cor, nivel.min(teto));
            }
        }
    }
    q
}

/// Quanto dura a parte ativa de um pulso, em passos do ritmo. O resto do
/// período é espera com os pads apagados.
pub fn passos_do_pulso(brilho: u8) -> f32 {
    brilho.min(3) as f32 + 3.0
}

/// Distância de um pad ao centro do quadrado de 4 por 4, em pads.
/// Os quatro do meio ficam a 0,71; os cantos a 2,12.
fn distancia_do_centro(pad: u8) -> f32 {
    let i = (pad - 1) as f32;
    let coluna = i % 4.0;
    let linha = (i / 4.0).floor();
    ((coluna - 1.5).powi(2) + (linha - 1.5).powi(2)).sqrt()
}

/// Pulso: uma onda circular sai do centro e some na borda.
///
/// `fase` vai de 0 a 1 na parte ativa; acima de 1 é espera e devolve `None`,
/// que não vale escrita. O anel tem posição fracionária, então ele atravessa os
/// pads em vez de saltar de anel em anel.
pub fn pulso(fase: f32, brilho: u8, cor: Option<Cor>, numero_do_pulso: u64) -> Option<Quadro> {
    if !(0.0..=1.0).contains(&fase) {
        return None;
    }
    let teto = brilho.min(3) as f32;
    let cor = matiz(cor, numero_do_pulso);
    // O raio passa de 2,12 para a onda sair pelos cantos antes de acabar.
    let raio = fase * 2.9;
    // E a onda inteira enfraquece no fim, para o pulso morrer sem corte seco.
    let desvanecer = 1.0 - fase * fase;
    let mut q = vazio();
    let mut algum = false;
    for pad in 1..=16u8 {
        let d = distancia_do_centro(pad) - raio;
        let nivel = ((1.0 - d.abs() / 1.1).max(0.0) * desvanecer * teto).round() as u8;
        if nivel > 0 {
            q[pad as usize - 1] = (cor, nivel.min(brilho.min(3)));
            algum = true;
        }
    }
    // Quadro todo apagado no meio do caminho não vale escrita.
    if algum {
        Some(q)
    } else {
        None
    }
}

/// Adormecer: as cores da página caindo B, B-1, 0 em três passos.
pub fn adormecer(repouso: &Quadro, brilho: u8, passo: u64) -> Quadro {
    let b = brilho.min(3) as i16;
    let nivel = match passo {
        0 => b,
        1 => (b - 1).max(0),
        _ => 0,
    };
    let mut q = vazio();
    for (i, (cor, _)) in repouso.iter().enumerate() {
        if *cor != Cor::Apagado {
            q[i] = (*cor, nivel as u8);
        }
    }
    q
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pagina_cheia() -> Quadro {
        let mut q = vazio();
        for i in 0..16 {
            q[i] = (Cor::Azul, 2);
        }
        q
    }

    #[test]
    fn contorno_segue_a_ordem_pedida() {
        // A cabeça (o único pad no brilho máximo) passa pela lista na ordem.
        for (passo, esperado) in CONTORNO.iter().enumerate() {
            let q = contorno(passo as f32, 2, None);
            let cabeca = q
                .iter()
                .position(|(c, b)| *c != Cor::Apagado && *b == 2)
                .map(|i| i as u8 + 1);
            assert_eq!(cabeca, Some(*esperado), "passo {passo}");
        }
    }

    #[test]
    fn contorno_troca_de_cor_a_cada_volta() {
        let antes = contorno(15.0, 2, None);
        let depois = contorno(16.0, 2, None);
        let cor_de = |q: &Quadro| q.iter().find(|(c, _)| *c != Cor::Apagado).map(|(c, _)| *c);
        assert_ne!(cor_de(&antes), cor_de(&depois));
    }

    #[test]
    fn contorno_com_brilho_zero_e_um_ponto_sem_rastro() {
        let q = contorno(3.0, 0, None);
        assert_eq!(q.iter().filter(|(c, _)| *c != Cor::Apagado).count(), 1);
    }

    #[test]
    fn colunas_vai_e_volta_sem_salto() {
        let ordem: Vec<usize> = (0..6)
            .map(|p| {
                let q = colunas(p as f32, 2, None);
                let pad = q.iter().position(|(c, b)| *c != Cor::Apagado && *b == 2).unwrap() as u8 + 1;
                (pad as usize - 1) % 4
            })
            .collect();
        assert_eq!(ordem, vec![0, 1, 2, 3, 2, 1]);
    }

    #[test]
    fn respiracao_mantem_vazios_apagados_e_nunca_apaga_os_cheios() {
        let mut repouso = pagina_cheia();
        repouso[4] = APAGADO; // pad 5 vazio
        for t in (0..4000).step_by(50) {
            let q = respiracao(&repouso, 2, None, t, 4000);
            assert_eq!(q[4], APAGADO, "pad vazio acendeu em {t}");
            assert_ne!(q[0].0, Cor::Apagado, "pad cheio apagou em {t}");
        }
    }

    #[test]
    fn respiracao_rapida_cai_no_laco() {
        // No rápido o período é 2000 ms; toda troca de nível tem que cair num
        // múltiplo de 25 ms, senão o laço passa por cima.
        let mut anterior = nivel_respiracao(3, 0, 2000);
        for t in (25..2000).step_by(25) {
            let n = nivel_respiracao(3, t, 2000);
            if n != anterior {
                assert_eq!(t % 25, 0);
                anterior = n;
            }
        }
        // E a tabela fecha em 4000 ms no médio: o último degrau volta ao topo.
        assert_eq!(nivel_respiracao(3, 3999, 4000), 2);
        assert_eq!(nivel_respiracao(3, 0, 4000), 3);
    }

    #[test]
    fn um_matiz_por_quadro() {
        for passo in 0..40u64 {
            for q in [
                contorno(passo as f32, 3, None),
                colunas(passo as f32, 3, None),
                pulso((passo % 6) as f32 / 6.0, 3, None, passo).unwrap_or_else(vazio),
            ] {
                let cores: std::collections::HashSet<Cor> =
                    q.iter().filter(|(c, _)| *c != Cor::Apagado).map(|(c, _)| *c).collect();
                assert!(cores.len() <= 1, "mais de um matiz no passo {passo}: {cores:?}");
            }
        }
    }

    #[test]
    fn nenhum_quadro_passa_do_brilho_geral() {
        for b in 0..=3u8 {
            for passo in 0..40u64 {
                for q in [
                    contorno(passo as f32, b, None),
                    colunas(passo as f32, b, None),
                    pulso((passo % 6) as f32 / 6.0, b, None, 0).unwrap_or_else(vazio),
                    respiracao(&pagina_cheia(), b, None, passo * 100, 4000),
                    adormecer(&pagina_cheia(), b, passo % 3),
                ] {
                    for (i, (c, n)) in q.iter().enumerate() {
                        if *c != Cor::Apagado {
                            assert!(*n <= b, "pad {} em {n} com teto {b}", i + 1);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn pulso_sai_do_centro_e_chega_na_borda() {
        // No comeco a onda esta no centro; no fim, nos cantos.
        let comeco = pulso(0.0, 3, None, 0).unwrap();
        for pad in CENTRO {
            assert!(comeco[pad as usize - 1].1 > 0, "centro apagado no comeco");
        }
        assert_eq!(comeco[0], APAGADO, "o canto acendeu antes da hora");

        let media: Vec<f32> = (0..=10)
            .map(|i| {
                let q = pulso(i as f32 / 10.0, 3, None, 0).unwrap_or_else(vazio);
                let soma: f32 = (1..=16u8)
                    .map(|pad| q[pad as usize - 1].1 as f32 * distancia_do_centro(pad))
                    .sum();
                let peso: f32 = (1..=16u8).map(|pad| q[pad as usize - 1].1 as f32).sum();
                if peso > 0.0 { soma / peso } else { f32::NAN }
            })
            .filter(|x| !x.is_nan())
            .collect();
        assert!(media.len() >= 5, "poucos quadros com luz: {media:?}");
        assert!(
            media.last().unwrap() > media.first().unwrap(),
            "a onda nao se afastou do centro: {media:?}"
        );
    }

    #[test]
    fn pulso_fora_da_fase_e_espera() {
        assert_eq!(pulso(1.5, 3, None, 0), None);
        assert_eq!(pulso(-0.1, 3, None, 0), None);
    }

    #[test]
    fn o_anel_do_pulso_anda_sem_saltar() {
        // Entre dois quadros vizinhos o desenho nunca muda de todos os pads de
        // uma vez: e isso que separa um movimento continuo de um piscar.
        let mut anterior = pulso(0.0, 3, None, 0).unwrap();
        for i in 1..=20 {
            let Some(q) = pulso(i as f32 / 20.0, 3, None, 0) else { break };
            let mudaram = (0..16).filter(|&k| q[k] != anterior[k]).count();
            assert!(mudaram <= 12, "{mudaram} pads mudaram de uma vez no quadro {i}");
            anterior = q;
        }
    }
}
