//! A luz dos pads no descanso e o eco ao soltar.
//!
//! O `Animador` recebe o tempo e devolve um quadro para pintar por cima dos
//! pads. Ele não fala com o HID nem com o relógio de verdade: quem chama passa
//! o `Instant`. Isso deixa o orçamento de escritas provável por teste, com um
//! relógio falso, antes de qualquer modo chegar no aparelho.
//!
//! O desenho completo, com o motivo de cada regra, está em `docs/animacoes.md`.

pub mod modos;

use crate::config::{AoApertar, CorLuz, LuzDescanso, ModoLuz, Ritmo};
use modos::passos_do_pulso;
use crate::hid::Cor;
use std::time::{Duration, Instant};

/// Um quadro de luz: cor e brilho de cada pad. Índice = pad impresso menos 1.
pub type Quadro = [(Cor, u8); 16];

/// Pad apagado num quadro.
pub const APAGADO: (Cor, u8) = (Cor::Apagado, 0);

/// Passo mínimo em todo modo: teto duro de 20 quadros por segundo, não importa
/// a frequência do laço. É a primeira trava do orçamento de escritas.
///
/// Era 125 ms (8 por segundo) e o movimento saía picotado. O aparelho aceita
/// cerca de 31 escritas por segundo; 20 para a luz deixam 11 para a tela, que
/// no descanso só precisa de umas 4. Quem garante que a tela não fica sem vez é
/// a thread de escrita, não este número.
pub const PASSO_MINIMO: Duration = Duration::from_millis(50);

/// Os três passos do adormecer, antes de o modo começar.
const ADORMECER: Duration = Duration::from_millis(750);
const PASSO_ADORMECER: Duration = Duration::from_millis(250);

/// O eco ao soltar: brilho 3, depois 2, depois o de repouso.
const ECO_FORTE: Duration = Duration::from_millis(100);
const ECO_MEDIO: Duration = Duration::from_millis(200);

pub struct Animador {
    luz: LuzDescanso,
    ao_apertar: AoApertar,
    /// Quando adormeceu. `None` é acordado.
    inicio: Option<Instant>,
    /// O que está pintado por cima dos pads agora, se algo.
    quadro: Option<Quadro>,
    /// Pad em eco e quando foi solto.
    eco: Option<(u8, Instant)>,
}

impl Animador {
    pub fn novo(luz: LuzDescanso, ao_apertar: AoApertar) -> Self {
        Self {
            luz,
            ao_apertar,
            inicio: None,
            quadro: None,
            eco: None,
        }
    }

    /// A config mudou pela interface ou pelo MCP. Trocar de modo no meio do
    /// descanso recomeça a animação, para o modo novo entrar do primeiro quadro.
    pub fn configurar(&mut self, luz: LuzDescanso, ao_apertar: AoApertar) {
        if self.luz != luz && self.inicio.is_some() {
            self.inicio = None;
            self.quadro = None;
        }
        self.luz = luz;
        self.ao_apertar = ao_apertar;
    }

    /// O que pintar por cima dos pads, se houver.
    pub fn quadro(&self) -> Option<&Quadro> {
        self.quadro.as_ref()
    }

    /// Corte seco: quem apertou quer o pad agora. Zera o descanso e o quadro.
    pub fn acordar(&mut self) {
        self.inicio = None;
        self.quadro = None;
    }

    /// Um pad foi solto: começa o eco, se ele estiver ligado.
    pub fn eco(&mut self, pad: u8, agora: Instant) {
        if self.ao_apertar == AoApertar::Eco && (1..=16).contains(&pad) {
            self.eco = Some((pad, agora));
        }
    }

    /// Se o descanso está animando os pads agora. É o que faz a touch strip cair
    /// para o fraco: no eco, que acontece acordado, ela não pode piscar.
    pub fn dormindo(&self) -> bool {
        self.inicio.is_some() && self.luz.modo != ModoLuz::Nenhuma
    }

    /// O laço acelera para 25 ms só quando há o que animar. Com o modo `Nenhuma`
    /// não há quadro nenhum, e acordar 40 vezes por segundo seria CPU à toa.
    pub fn precisa_de_tique(&self) -> bool {
        self.dormindo() || self.eco.is_some()
    }

    /// Modo Som e dormindo: o laço lê o medidor. O modo Som ainda cai na
    /// Respiração (passo 9 do desenho), então por enquanto nunca pede.
    pub fn quer_som(&self) -> bool {
        false
    }

    /// Avança o tempo. Devolve `true` se o quadro mudou e vale uma escrita.
    ///
    /// `dormindo` vem do compositor da tela; `repouso` são as cores da página
    /// agora, já com brilho por pad e cor de programa aberto. `pico` fica para
    /// o modo Som.
    pub fn tique(
        &mut self,
        agora: Instant,
        dormindo: bool,
        brilho: u8,
        repouso: &Quadro,
        _pico: Option<(f32, f32)>,
    ) -> bool {
        // O eco vale só acordado: dormindo, qualquer toque já acordou.
        if !dormindo {
            let antes = self.quadro;
            self.inicio = None;
            self.quadro = self.quadro_do_eco(agora, repouso);
            return antes != self.quadro;
        }

        // Dormindo. O eco não sobrevive ao descanso.
        self.eco = None;
        let inicio = *self.inicio.get_or_insert(agora);
        let passado = agora.saturating_duration_since(inicio);
        let novo = self.quadro_do_descanso(passado, brilho, repouso);
        if novo != self.quadro {
            self.quadro = novo;
            true
        } else {
            false
        }
    }

    /// O quadro do eco, ou `None` se não há eco ou ele já terminou.
    fn quadro_do_eco(&mut self, agora: Instant, repouso: &Quadro) -> Option<Quadro> {
        let (pad, desde) = self.eco?;
        let (cor, brilho_repouso) = repouso[pad as usize - 1];
        let passado = agora.saturating_duration_since(desde);
        // Pad vazio, ou já no brilho 3: não há para onde pousar.
        if cor == Cor::Apagado || brilho_repouso >= 3 || passado >= ECO_MEDIO {
            self.eco = None;
            return None;
        }
        let nivel = if passado < ECO_FORTE { 3 } else { 2.max(brilho_repouso) };
        let mut q = *repouso;
        q[pad as usize - 1] = (cor, nivel);
        Some(q)
    }

    /// O quadro do descanso no instante `passado` desde que adormeceu.
    fn quadro_do_descanso(&self, passado: Duration, brilho: u8, repouso: &Quadro) -> Option<Quadro> {
        let brilho = brilho.min(3);
        let cor = match self.luz.cor {
            CorLuz::Auto => None,
            CorLuz::Fixa(c) => Some(c),
        };
        // O modo Som ainda não existe: cai na Respiração, como o desenho manda
        // fazer no silêncio.
        let modo = match self.luz.modo {
            ModoLuz::Som => ModoLuz::Respiracao,
            outro => outro,
        };
        match modo {
            ModoLuz::Nenhuma => None,
            ModoLuz::Respiracao => {
                let periodo = periodo_ms(self.luz.ritmo);
                let t = quantizar(passado).as_millis() as u64;
                Some(modos::respiracao(repouso, brilho, cor, t, periodo))
            }
            _ if passado < ADORMECER => {
                let passo = (passado.as_millis() / PASSO_ADORMECER.as_millis()) as u64;
                Some(modos::adormecer(repouso, brilho, passo))
            }
            ModoLuz::Contorno => {
                let pos = posicao(passado - ADORMECER, passo_ms(self.luz.ritmo));
                Some(modos::contorno(pos, brilho, cor))
            }
            ModoLuz::Colunas => {
                let pos = posicao(passado - ADORMECER, passo_ms(self.luz.ritmo));
                Some(modos::colunas(pos, brilho, cor))
            }
            ModoLuz::Pulso => {
                let periodo = periodo_ms(self.luz.ritmo);
                let t = quantizar(passado - ADORMECER).as_millis() as u64;
                let numero = t / periodo;
                // A parte ativa dura uns poucos passos do ritmo; o resto do
                // período é espera. A fase vai de 0 a 1 dentro da parte ativa, e
                // é fracionária, para o anel atravessar os pads sem saltar.
                let ativo = passos_do_pulso(brilho) * passo_ms(self.luz.ritmo) as f32;
                let fase = (t % periodo) as f32 / ativo.max(1.0);
                Some(modos::pulso(fase, brilho, cor, numero).unwrap_or_else(modos::vazio))
            }
            ModoLuz::Som => None,
        }
    }
}

/// Passo dos modos de varredura, por ritmo. O rápido não é 125 ms de
/// propósito: coincidir com o passo do texto na tela dá tremor visível.
fn passo_ms(ritmo: Ritmo) -> u64 {
    match ritmo {
        Ritmo::Lento => 500,
        Ritmo::Medio => 250,
        Ritmo::Rapido => 150,
    }
}

/// Período da Respiração e do Pulso, por ritmo.
fn periodo_ms(ritmo: Ritmo) -> u64 {
    match ritmo {
        Ritmo::Lento => 8000,
        Ritmo::Medio => 4000,
        Ritmo::Rapido => 2000,
    }
}

fn passos(passado: Duration, passo_ms: u64) -> u64 {
    (passado.as_millis() / passo_ms.max(1) as u128) as u64
}

/// Posição da cabeça em passos, fracionária. É o que deixa o movimento
/// contínuo: entre dois pads, os dois acendem em meio-termo.
fn posicao(passado: Duration, passo_ms: u64) -> f32 {
    quantizar(passado).as_millis() as f32 / passo_ms.max(1) as f32
}

/// Arredonda o tempo para o laço de 25 ms, para a Respiração não gerar quadros
/// diferentes por diferença de microssegundos.
fn quantizar(d: Duration) -> Duration {
    let passo = PASSO_MINIMO.as_millis().max(1);
    Duration::from_millis((d.as_millis() / passo * passo) as u64)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pagina() -> Quadro {
        let mut q = [APAGADO; 16];
        for (i, slot) in q.iter_mut().enumerate() {
            if i % 2 == 0 {
                *slot = (Cor::Verde, 2);
            }
        }
        q
    }

    fn luz(modo: ModoLuz, ritmo: Ritmo) -> LuzDescanso {
        LuzDescanso {
            modo,
            ritmo,
            cor: CorLuz::Auto,
        }
    }

    /// Simula 10 s de tiques de 25 ms e conta quantos quadros distintos saíram.
    fn quadros_em_10s(modo: ModoLuz, ritmo: Ritmo, brilho: u8) -> usize {
        let mut a = Animador::novo(luz(modo, ritmo), AoApertar::Nenhuma);
        let t0 = Instant::now();
        let repouso = pagina();
        let mut n = 0;
        for i in 0..400u64 {
            let agora = t0 + Duration::from_millis(i * 25);
            if a.tique(agora, true, brilho, &repouso, None) {
                n += 1;
            }
        }
        n
    }

    #[test]
    fn orcamento_de_escritas() {
        // Teto: 20 quadros por segundo, mais os 3 do adormecer. O aparelho
        // aceita cerca de 31 escritas por segundo no total, e a tela no
        // descanso precisa de umas 4. Todo modo novo entra aqui antes de
        // entrar no aparelho.
        for modo in [
            ModoLuz::Nenhuma,
            ModoLuz::Respiracao,
            ModoLuz::Contorno,
            ModoLuz::Colunas,
            ModoLuz::Pulso,
            ModoLuz::Som,
        ] {
            for ritmo in [Ritmo::Lento, Ritmo::Medio, Ritmo::Rapido] {
                for brilho in 0..=3u8 {
                    let n = quadros_em_10s(modo, ritmo, brilho);
                    assert!(
                        n <= 200 + 3,
                        "{modo:?} {ritmo:?} B={brilho}: {n} quadros em 10 s"
                    );
                }
            }
        }
    }

    #[test]
    fn contorno_rapido_fica_perto_do_previsto() {
        // Com o movimento continuo o passo do ritmo nao manda mais na taxa de
        // quadros: manda o passo minimo, 50 ms. O ritmo muda a velocidade da
        // cabeca, nao quantas vezes por segundo a luz e reescrita.
        let n = quadros_em_10s(ModoLuz::Contorno, Ritmo::Rapido, 2);
        assert!((170..=203).contains(&n), "{n}");
    }

    #[test]
    fn respiracao_gera_poucos_quadros() {
        let n = quadros_em_10s(ModoLuz::Respiracao, Ritmo::Medio, 2);
        assert!((5..=15).contains(&n), "{n}");
        assert_eq!(quadros_em_10s(ModoLuz::Respiracao, Ritmo::Medio, 0), 1, "brilho 0 e fraco parado");
    }

    #[test]
    fn modo_nenhuma_nao_acelera_o_laco() {
        // Sem quadro para animar, acordar 40 vezes por segundo seria CPU a toa.
        let mut a = Animador::novo(luz(ModoLuz::Nenhuma, Ritmo::Medio), AoApertar::Nenhuma);
        let t0 = Instant::now();
        a.tique(t0, true, 2, &pagina(), None);
        assert!(!a.precisa_de_tique());
        assert!(!a.dormindo());
    }

    #[test]
    fn o_eco_nao_conta_como_descanso() {
        // A strip so escurece dormindo; no eco ela nao pode piscar.
        let mut a = Animador::novo(luz(ModoLuz::Nenhuma, Ritmo::Medio), AoApertar::Eco);
        let t0 = Instant::now();
        a.eco(1, t0);
        a.tique(t0, false, 2, &pagina(), None);
        assert!(a.quadro().is_some());
        assert!(!a.dormindo(), "eco nao e descanso");
        assert!(a.precisa_de_tique(), "mas o laco precisa correr para o eco pousar");
    }

    #[test]
    fn pulso_repete_no_periodo_do_ritmo() {
        // O ritmo manda em quantos pulsos por minuto, nao mais em quantos
        // quadros por segundo: a onda e continua e anda a 20 por segundo.
        let mut a = Animador::novo(luz(ModoLuz::Pulso, Ritmo::Medio), AoApertar::Nenhuma);
        let t0 = Instant::now();
        let repouso = pagina();
        let mut acesos = Vec::new();
        for ms in (750..9000).step_by(50) {
            a.tique(t0 + Duration::from_millis(ms), true, 3, &repouso, None);
            let n = a
                .quadro()
                .map(|q| q.iter().filter(|(c, _)| *c != Cor::Apagado).count())
                .unwrap_or(0);
            acesos.push((ms, n));
        }
        // Duas partes ativas em 8 s, com o periodo medio de 4 s.
        let mut inicios = 0;
        let mut estava_apagado = true;
        for (_, n) in &acesos {
            if *n > 0 && estava_apagado {
                inicios += 1;
            }
            estava_apagado = *n == 0;
        }
        assert!((2..=3).contains(&inicios), "{inicios} pulsos em 8 s");
    }

    #[test]
    fn acordar_e_corte_seco() {
        let mut a = Animador::novo(luz(ModoLuz::Contorno, Ritmo::Medio), AoApertar::Nenhuma);
        let t0 = Instant::now();
        a.tique(t0, true, 2, &pagina(), None);
        a.tique(t0 + Duration::from_secs(2), true, 2, &pagina(), None);
        assert!(a.quadro().is_some());
        a.acordar();
        assert!(a.quadro().is_none());
        assert!(!a.precisa_de_tique());
    }

    #[test]
    fn eco_pousa_em_200_ms() {
        let mut a = Animador::novo(luz(ModoLuz::Nenhuma, Ritmo::Medio), AoApertar::Eco);
        let t0 = Instant::now();
        let repouso = pagina(); // pad 1 verde em 2
        a.eco(1, t0);
        assert!(a.tique(t0, false, 2, &repouso, None));
        assert_eq!(a.quadro().unwrap()[0], (Cor::Verde, 3));
        assert!(a.tique(t0 + Duration::from_millis(100), false, 2, &repouso, None));
        assert_eq!(a.quadro().unwrap()[0], (Cor::Verde, 2));
        assert!(a.tique(t0 + Duration::from_millis(200), false, 2, &repouso, None));
        assert!(a.quadro().is_none(), "depois de 200 ms o pad pousa no repouso");
        assert!(!a.precisa_de_tique());
    }

    #[test]
    fn eco_em_pad_no_brilho_3_nao_gera_quadro() {
        let mut a = Animador::novo(luz(ModoLuz::Nenhuma, Ritmo::Medio), AoApertar::Eco);
        let t0 = Instant::now();
        let mut repouso = pagina();
        repouso[0] = (Cor::Verde, 3);
        a.eco(1, t0);
        assert!(!a.tique(t0, false, 3, &repouso, None));
        assert!(a.quadro().is_none());
    }

    #[test]
    fn eco_desligado_nao_faz_nada() {
        let mut a = Animador::novo(luz(ModoLuz::Nenhuma, Ritmo::Medio), AoApertar::Nenhuma);
        a.eco(1, Instant::now());
        assert!(!a.precisa_de_tique());
    }

    #[test]
    fn trocar_o_modo_dormindo_recomeca() {
        let mut a = Animador::novo(luz(ModoLuz::Contorno, Ritmo::Medio), AoApertar::Nenhuma);
        let t0 = Instant::now();
        a.tique(t0, true, 2, &pagina(), None);
        a.configurar(luz(ModoLuz::Colunas, Ritmo::Medio), AoApertar::Nenhuma);
        assert!(a.quadro().is_none());
        assert!(a.tique(t0 + Duration::from_secs(1), true, 2, &pagina(), None));
    }
}
