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

/// O caminho do cometa, exatamente como o Davi pediu: sobe a coluna da
/// esquerda, atravessa a fileira de cima, desce a da direita, volta pela de
/// baixo e fecha pelo centro.
pub const CONTORNO: [u8; 16] = [1, 5, 9, 13, 14, 15, 16, 12, 8, 4, 3, 2, 6, 10, 11, 7];

/// Os quatro pads do meio. A borda é o resto.
pub const CENTRO: [u8; 4] = [6, 7, 10, 11];

/// As quatro colunas, de baixo para cima, e a ordem de varredura que vai e
/// volta sem saltar da última para a primeira.
pub const COLUNAS: [[u8; 4]; 4] = [[1, 5, 9, 13], [2, 6, 10, 14], [3, 7, 11, 15], [4, 8, 12, 16]];
pub const VARREDURA: [usize; 6] = [0, 1, 2, 3, 2, 1];

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
/// Cabeça em B, o pad anterior em B-1, e assim até o fraco; o resto apagado.
/// A cor anda um passo na roda a cada volta, no salto do 7 para o 1.
pub fn contorno(passo: u64, brilho: u8, cor: Option<Cor>) -> Quadro {
    let n = CONTORNO.len() as u64;
    let volta = passo / n;
    let cabeca = (passo % n) as usize;
    let cor = matiz(cor, volta);
    let mut q = vazio();
    for atras in 0..=brilho.min(3) as usize {
        let indice = (cabeca + CONTORNO.len() - atras) % CONTORNO.len();
        let pad = CONTORNO[indice];
        q[pad as usize - 1] = (cor, brilho.min(3) - atras as u8);
    }
    q
}

/// Colunas: uma coluna varre da esquerda para a direita e volta, sem salto.
/// Coluna atual em B, a de onde ela veio no fraco, o resto apagado. A cor anda
/// na roda a cada duas varreduras.
pub fn colunas(passo: u64, brilho: u8, cor: Option<Cor>) -> Quadro {
    let n = VARREDURA.len() as u64;
    let cor = matiz(cor, passo / (2 * n));
    let atual = VARREDURA[(passo % n) as usize];
    let anterior = VARREDURA[((passo + n - 1) % n) as usize];
    let mut q = vazio();
    for pad in COLUNAS[anterior] {
        q[pad as usize - 1] = (cor, 0);
    }
    for pad in COLUNAS[atual] {
        q[pad as usize - 1] = (cor, brilho.min(3));
    }
    q
}

/// Quantos quadros um pulso tem para um teto de brilho: o centro desce de B
/// até apagado e a borda vai um nível atrás.
pub fn quadros_do_pulso(brilho: u8) -> u64 {
    brilho.min(3) as u64 + 3
}

/// Pulso: uma gota no centro se espalha para a borda e some. Devolve `None`
/// nos quadros de espera, em que nada muda e não vale escrita.
pub fn pulso(quadro: u64, brilho: u8, cor: Option<Cor>, numero_do_pulso: u64) -> Option<Quadro> {
    let b = brilho.min(3) as i16;
    if quadro >= quadros_do_pulso(brilho) {
        return None;
    }
    let cor = matiz(cor, numero_do_pulso);
    // Nível do centro cai um por quadro a partir de B; a borda começa um quadro
    // depois. Nível -1 é apagado.
    let centro = b - quadro as i16;
    let borda = b - quadro as i16 + 1;
    let mut q = vazio();
    for pad in 1..=16u8 {
        let no_centro = CENTRO.contains(&pad);
        let nivel = if no_centro { centro } else { borda };
        if nivel >= 0 && (no_centro || quadro >= 1) {
            q[pad as usize - 1] = (cor, nivel.min(b) as u8);
        }
    }
    Some(q)
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
    fn contorno_segue_a_lista_do_davi() {
        // A cabeça (o único pad no brilho máximo) passa pela lista na ordem.
        for (passo, esperado) in CONTORNO.iter().enumerate() {
            let q = contorno(passo as u64, 2, None);
            let cabeca = q
                .iter()
                .position(|(c, b)| *c != Cor::Apagado && *b == 2)
                .map(|i| i as u8 + 1);
            assert_eq!(cabeca, Some(*esperado), "passo {passo}");
        }
    }

    #[test]
    fn contorno_troca_de_cor_a_cada_volta() {
        let antes = contorno(15, 2, None);
        let depois = contorno(16, 2, None);
        let cor_de = |q: &Quadro| q.iter().find(|(c, _)| *c != Cor::Apagado).map(|(c, _)| *c);
        assert_ne!(cor_de(&antes), cor_de(&depois));
    }

    #[test]
    fn contorno_com_brilho_zero_e_um_ponto_sem_rastro() {
        let q = contorno(3, 0, None);
        assert_eq!(q.iter().filter(|(c, _)| *c != Cor::Apagado).count(), 1);
    }

    #[test]
    fn colunas_vai_e_volta_sem_salto() {
        let ordem: Vec<usize> = (0..6)
            .map(|p| {
                let q = colunas(p, 2, None);
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
                contorno(passo, 3, None),
                colunas(passo, 3, None),
                pulso(passo % 6, 3, None, passo).unwrap_or_else(vazio),
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
                    contorno(passo, b, None),
                    colunas(passo, b, None),
                    pulso(passo % 6, b, None, 0).unwrap_or_else(vazio),
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
    fn pulso_comeca_no_centro_e_termina_apagado() {
        let primeiro = pulso(0, 2, None, 0).unwrap();
        for pad in CENTRO {
            assert_eq!(primeiro[pad as usize - 1].1, 2);
        }
        assert_eq!(primeiro[0], APAGADO, "a borda so entra no segundo quadro");
        let ultimo = pulso(quadros_do_pulso(2) - 1, 2, None, 0).unwrap();
        assert!(ultimo.iter().all(|(c, _)| *c == Cor::Apagado));
        assert_eq!(pulso(quadros_do_pulso(2), 2, None, 0), None, "depois e espera");
    }
}
