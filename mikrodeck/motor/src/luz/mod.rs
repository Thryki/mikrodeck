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
use crate::hid::Cor;
use std::time::{Duration, Instant};

/// Um quadro de luz: cor e brilho de cada pad. Índice = pad impresso menos 1.
pub type Quadro = [(Cor, u8); 16];

/// Pad apagado num quadro.
pub const APAGADO: (Cor, u8) = (Cor::Apagado, 0);

/// Passo mínimo em todo modo: teto duro de 8 quadros por segundo, não importa
/// a frequência do laço. É a primeira trava do orçamento de escritas.
pub const PASSO_MINIMO: Duration = Duration::from_millis(125);

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
                let passo = passos(passado - ADORMECER, passo_ms(self.luz.ritmo));
                Some(modos::contorno(passo, brilho, cor))
            }
            ModoLuz::Colunas => {
                let passo = passos(passado - ADORMECER, passo_ms(self.luz.ritmo));
                Some(modos::colunas(passo, brilho, cor))
            }
            ModoLuz::Pulso => {
                let periodo = periodo_ms(self.luz.ritmo);
                let t = (passado - ADORMECER).as_millis() as u64;
                let numero = t / periodo;
                // Os quadros do pulso andam no passo do ritmo, não no passo mínimo:
                // no médio são 5 quadros em 1,25 s, como o desenho pede. O resto do
                // período é espera, em que `pulso` devolve None e o quadro apaga.
                let quadro = (t % periodo) / passo_ms(self.luz.ritmo).max(1);
                Some(modos::pulso(quadro, brilho, cor, numero).unwrap_or_else(modos::vazio))
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

/// Arredonda o tempo para o laço de 25 ms, para a Respiração não gerar quadros
/// diferentes por diferença de microssegundos.
fn quantizar(d: Duration) -> Duration {
    Duration::from_millis((d.as_millis() / 25 * 25) as u64)
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
        // Teto: 8 quadros por segundo, mais os 3 do adormecer. Todo modo novo
        // entra aqui antes de entrar no aparelho.
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
                        n <= 80 + 3,
                        "{modo:?} {ritmo:?} B={brilho}: {n} quadros em 10 s"
                    );
                }
            }
        }
    }

    #[test]
    fn contorno_rapido_fica_perto_do_previsto() {
        // 150 ms por passo: uns 6,7 por segundo, mais o adormecer.
        let n = quadros_em_10s(ModoLuz::Contorno, Ritmo::Rapido, 2);
        assert!((55..=70).contains(&n), "{n}");
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
    fn pulso_anda_no_passo_do_ritmo() {
        // No medio o passo e 250 ms: cinco quadros em 1,25 s, como o desenho pede.
        let mut a = Animador::novo(luz(ModoLuz::Pulso, Ritmo::Medio), AoApertar::Nenhuma);
        let t0 = Instant::now();
        let repouso = pagina();
        // Passa o adormecer (750 ms) e mede quando o quadro muda.
        let mut trocas = 0;
        for ms in (750..2000).step_by(25) {
            if a.tique(t0 + Duration::from_millis(ms), true, 2, &repouso, None) {
                trocas += 1;
            }
        }
        // Cinco quadros de pulso em 1,25 s, e nao dez.
        assert!((4..=6).contains(&trocas), "{trocas} trocas em 1,25 s");
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
