//! O envelope de um sample: ataque, decaimento, sustentação e liberação.
//!
//! É uma fonte que embrulha outra e multiplica o ganho quadro a quadro. Não
//! fala com o aparelho nem com o relógio: conta quadros, então o teste roda sem
//! placa de som e sem esperar tempo nenhum passar.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// O envelope, em milissegundos, com a sustentação e as tensões de 0 a 1.
///
/// Os estágios são os mesmos de um plugin: atraso, ataque, retenção,
/// decaimento, sustentação e liberação. As duas tensões curvam a subida e a
/// descida, como as duas alças de tensão do desenho.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// Silêncio antes de o som começar a subir.
    #[serde(default = "zero")]
    pub atraso_ms: u32,
    /// Quanto o som demora para chegar no volume cheio.
    #[serde(default = "zero")]
    pub ataque_ms: u32,
    /// Quanto ele fica no cheio antes de começar a cair.
    #[serde(default = "zero")]
    pub retencao_ms: u32,
    /// Quanto ele demora para cair do cheio até a sustentação.
    #[serde(default = "zero")]
    pub decaimento_ms: u32,
    /// O volume em que ele se segura, de 0 a 1.
    #[serde(default = "um")]
    pub sustentacao: f32,
    /// Quanto ele demora para sumir depois que o pad é solto.
    #[serde(default = "cem")]
    pub liberacao_ms: u32,
    /// Curva da subida, de -1 a 1. Zero é reta.
    #[serde(default = "zero_f")]
    pub tensao_ataque: f32,
    /// Curva das descidas (decaimento e liberação), de -1 a 1. Zero é reta.
    #[serde(default = "zero_f")]
    pub tensao_queda: f32,
}

fn zero() -> u32 {
    0
}
fn cem() -> u32 {
    100
}
fn um() -> f32 {
    1.0
}
fn zero_f() -> f32 {
    0.0
}

impl Default for Envelope {
    /// O padrão não muda nada no som: entra cheio, fica cheio, e some rápido
    /// quando solta, só para não estalar.
    fn default() -> Self {
        Self {
            atraso_ms: 0,
            ataque_ms: 0,
            retencao_ms: 0,
            decaimento_ms: 0,
            sustentacao: 1.0,
            liberacao_ms: 100,
            tensao_ataque: 0.0,
            tensao_queda: 0.0,
        }
    }
}

/// Curva uma fração de 0 a 1 pela tensão.
///
/// Tensão zero é reta. Positiva sobe rápido e assenta no fim; negativa demora a
/// sair e corre no fim. É a mesma ideia das alças de tensão de um plugin, feita
/// com expoente porque assim ela nunca sai da faixa de 0 a 1.
pub fn curvar(x: f32, tensao: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let t = tensao.clamp(-1.0, 1.0);
    if t.abs() < 0.001 {
        return x;
    }
    x.powf(2f32.powf(-t * 2.0))
}

impl Envelope {
    /// Quanto tempo o envelope leva do começo até chegar na sustentação.
    pub fn ate_a_sustentacao_ms(&self) -> u32 {
        self.atraso_ms + self.ataque_ms + self.retencao_ms + self.decaimento_ms
    }

    /// Ganho de 0 a 1 no instante `t`, com o pad ainda apertado.
    pub fn ganho_segurando(&self, t: Duration) -> f32 {
        let ms = t.as_secs_f32() * 1000.0;
        let s = self.sustentacao.clamp(0.0, 1.0);
        let atraso = self.atraso_ms as f32;
        let a = self.ataque_ms as f32;
        let h = self.retencao_ms as f32;
        let d = self.decaimento_ms as f32;
        if ms < atraso {
            0.0
        } else if ms < atraso + a {
            // Sem ataque, `a` é zero e este ramo nem roda.
            curvar((ms - atraso) / a, self.tensao_ataque)
        } else if ms < atraso + a + h {
            1.0
        } else if ms < atraso + a + h + d {
            let fracao = (ms - atraso - a - h) / d;
            1.0 - (1.0 - s) * curvar(fracao, self.tensao_queda)
        } else {
            s
        }
    }

    /// Ganho depois de soltar: cai do nível em que estava até zero.
    pub fn ganho_soltando(&self, nivel_no_solto: f32, desde_o_solto: Duration) -> f32 {
        let r = self.liberacao_ms as f32;
        if r <= 0.0 {
            return 0.0;
        }
        let ms = desde_o_solto.as_secs_f32() * 1000.0;
        let fracao = (ms / r).clamp(0.0, 1.0);
        (nivel_no_solto * (1.0 - curvar(fracao, self.tensao_queda))).max(0.0)
    }

    /// Se o som já acabou de sumir depois de soltar.
    pub fn terminou(&self, desde_o_solto: Duration) -> bool {
        desde_o_solto.as_secs_f32() * 1000.0 >= self.liberacao_ms as f32
    }
}

/// Uma fonte com o envelope aplicado. `solto` é compartilhado com quem tocou:
/// levantar a bandeira faz o som entrar na liberação e terminar sozinho.
pub struct ComEnvelope<S> {
    fonte: S,
    envelope: Envelope,
    taxa: u32,
    canais: u16,
    /// Quadros já emitidos. Um quadro é uma amostra por canal.
    quadro: u64,
    /// Em qual canal do quadro atual estamos.
    canal: u16,
    /// Ganho do quadro atual, calculado uma vez e reusado nos canais dele.
    ganho: f32,
    solto: Arc<AtomicBool>,
    /// Quadro em que o pad foi solto, e o ganho naquele instante.
    solto_em: Option<(u64, f32)>,
}

impl<S> ComEnvelope<S>
where
    S: rodio::Source,
{
    pub fn novo(fonte: S, envelope: Envelope, solto: Arc<AtomicBool>) -> Self {
        let taxa = fonte.sample_rate().max(1);
        let canais = fonte.channels().max(1);
        Self {
            fonte,
            envelope,
            taxa,
            canais,
            quadro: 0,
            canal: 0,
            ganho: 0.0,
            solto,
            solto_em: None,
        }
    }

    fn instante(&self, quadro: u64) -> Duration {
        Duration::from_secs_f64(quadro as f64 / self.taxa as f64)
    }

    /// Calcula o ganho do quadro atual e diz se o som acabou.
    fn ganho_do_quadro(&mut self) -> Option<f32> {
        let segurando = self.envelope.ganho_segurando(self.instante(self.quadro));
        if self.solto_em.is_none() && self.solto.load(Ordering::Relaxed) {
            self.solto_em = Some((self.quadro, segurando));
        }
        match self.solto_em {
            None => Some(segurando),
            Some((quadro_do_solto, nivel)) => {
                let desde = self.instante(self.quadro.saturating_sub(quadro_do_solto));
                if self.envelope.terminou(desde) {
                    None
                } else {
                    Some(self.envelope.ganho_soltando(nivel, desde))
                }
            }
        }
    }
}

impl<S> Iterator for ComEnvelope<S>
where
    S: rodio::Source,
{
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.canal == 0 {
            self.ganho = self.ganho_do_quadro()?;
        }
        let amostra = self.fonte.next()?;
        self.canal += 1;
        if self.canal >= self.canais {
            self.canal = 0;
            self.quadro += 1;
        }
        Some(amostra * self.ganho)
    }
}

impl<S> rodio::Source for ComEnvelope<S>
where
    S: rodio::Source,
{
    fn current_span_len(&self) -> Option<usize> {
        self.fonte.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.canais
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.taxa
    }

    fn total_duration(&self) -> Option<Duration> {
        self.fonte.total_duration()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn env() -> Envelope {
        Envelope {
            ataque_ms: 100,
            decaimento_ms: 200,
            sustentacao: 0.5,
            liberacao_ms: 300,
            ..Envelope::default()
        }
    }

    #[test]
    fn o_ataque_sobe_do_zero_ate_o_cheio() {
        let e = env();
        assert_eq!(e.ganho_segurando(Duration::ZERO), 0.0);
        assert!((e.ganho_segurando(Duration::from_millis(50)) - 0.5).abs() < 0.01);
        // No fim do ataque ja esta em 1, e o decaimento comeca dali.
        assert!((e.ganho_segurando(Duration::from_millis(100)) - 1.0).abs() < 0.01);
    }

    #[test]
    fn o_decaimento_cai_ate_a_sustentacao_e_para_la() {
        let e = env();
        let meio = e.ganho_segurando(Duration::from_millis(200));
        assert!((meio - 0.75).abs() < 0.01, "{meio}");
        assert!((e.ganho_segurando(Duration::from_millis(300)) - 0.5).abs() < 0.01);
        // Depois disso fica parado na sustentacao, por mais tempo que passe.
        assert!((e.ganho_segurando(Duration::from_secs(10)) - 0.5).abs() < 0.01);
    }

    #[test]
    fn a_liberacao_cai_do_nivel_em_que_estava() {
        let e = env();
        assert!((e.ganho_soltando(0.5, Duration::ZERO) - 0.5).abs() < 0.01);
        assert!((e.ganho_soltando(0.5, Duration::from_millis(150)) - 0.25).abs() < 0.01);
        assert_eq!(e.ganho_soltando(0.5, Duration::from_millis(300)), 0.0);
        assert!(e.terminou(Duration::from_millis(300)));
        assert!(!e.terminou(Duration::from_millis(299)));
    }

    #[test]
    fn o_atraso_segura_o_som_no_zero() {
        let e = Envelope {
            atraso_ms: 200,
            ataque_ms: 100,
            ..Envelope::default()
        };
        assert_eq!(e.ganho_segurando(Duration::from_millis(0)), 0.0);
        assert_eq!(e.ganho_segurando(Duration::from_millis(199)), 0.0);
        // O ataque comeca depois do atraso, nao do zero.
        assert!((e.ganho_segurando(Duration::from_millis(250)) - 0.5).abs() < 0.02);
        assert!((e.ganho_segurando(Duration::from_millis(300)) - 1.0).abs() < 0.02);
    }

    #[test]
    fn a_retencao_segura_no_cheio_antes_de_cair() {
        let e = Envelope {
            ataque_ms: 100,
            retencao_ms: 300,
            decaimento_ms: 200,
            sustentacao: 0.0,
            ..Envelope::default()
        };
        for ms in [100, 200, 350, 399] {
            let g = e.ganho_segurando(Duration::from_millis(ms));
            assert!((g - 1.0).abs() < 0.01, "caiu cedo em {ms} ms: {g}");
        }
        // Passada a retencao, o decaimento comeca.
        assert!(e.ganho_segurando(Duration::from_millis(500)) < 0.6);
    }

    #[test]
    fn a_tensao_curva_sem_sair_da_faixa() {
        // Em qualquer tensao a curva sai do zero, chega no um e nao passa disso.
        for tensao in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            assert_eq!(curvar(0.0, tensao), 0.0);
            assert!((curvar(1.0, tensao) - 1.0).abs() < 0.001);
            for i in 0..=10 {
                let y = curvar(i as f32 / 10.0, tensao);
                assert!((0.0..=1.0).contains(&y), "tensao {tensao} deu {y}");
            }
        }
    }

    #[test]
    fn tensao_positiva_sobe_mais_rapido_que_a_reta() {
        let meio_reta = curvar(0.5, 0.0);
        assert!(curvar(0.5, 1.0) > meio_reta, "positiva devia subir antes");
        assert!(curvar(0.5, -1.0) < meio_reta, "negativa devia demorar");
    }

    #[test]
    fn a_tensao_nao_muda_quando_o_som_acaba() {
        // A curva muda o caminho, nunca a duracao: senao mexer na tensao
        // mudaria quanto tempo o som dura, que nao e o que se espera.
        let reto = Envelope { liberacao_ms: 300, ..Envelope::default() };
        let curvo = Envelope { tensao_queda: 0.8, ..reto };
        assert_eq!(reto.terminou(Duration::from_millis(299)), curvo.terminou(Duration::from_millis(299)));
        assert_eq!(reto.terminou(Duration::from_millis(300)), curvo.terminou(Duration::from_millis(300)));
    }

    #[test]
    fn ate_a_sustentacao_soma_os_quatro_primeiros_estagios() {
        let e = Envelope {
            atraso_ms: 10,
            ataque_ms: 20,
            retencao_ms: 30,
            decaimento_ms: 40,
            ..Envelope::default()
        };
        assert_eq!(e.ate_a_sustentacao_ms(), 100);
    }

    #[test]
    fn o_envelope_padrao_nao_mexe_no_som() {
        let e = Envelope::default();
        for ms in [0, 1, 100, 5000] {
            assert_eq!(e.ganho_segurando(Duration::from_millis(ms)), 1.0);
        }
    }

    #[test]
    fn liberacao_zero_corta_na_hora_sem_dividir_por_zero() {
        let e = Envelope {
            liberacao_ms: 0,
            ..Envelope::default()
        };
        assert_eq!(e.ganho_soltando(1.0, Duration::ZERO), 0.0);
        assert!(e.terminou(Duration::ZERO));
    }

    #[test]
    fn soltar_termina_a_fonte_mesmo_que_o_audio_seja_longo() {
        use rodio::buffer::SamplesBuffer;
        use rodio::Source;
        // Um segundo de som em 1000 Hz, mono.
        let fonte = SamplesBuffer::new(1, 1000, vec![1.0f32; 1000]);
        let solto = Arc::new(AtomicBool::new(false));
        let e = Envelope {
            liberacao_ms: 100,
            ..Envelope::default()
        };
        let mut com = ComEnvelope::novo(fonte, e, solto.clone());
        // 100 quadros com o pad apertado: ganho cheio.
        for _ in 0..100 {
            assert_eq!(com.next(), Some(1.0));
        }
        solto.store(true, Ordering::Relaxed);
        // A liberacao dura 100 ms, que a 1000 Hz sao 100 quadros.
        let restantes = com.count();
        assert_eq!(restantes, 100, "a liberacao devia durar 100 quadros");
    }

    #[test]
    fn o_ganho_e_o_mesmo_nos_dois_canais_do_quadro() {
        use rodio::buffer::SamplesBuffer;
        // Estereo: cada quadro tem duas amostras e as duas levam o mesmo ganho,
        // senao o ataque viraria um deslocamento entre os canais.
        let fonte = SamplesBuffer::new(2, 1000, vec![1.0f32; 400]);
        let e = Envelope {
            ataque_ms: 100,
            ..Envelope::default()
        };
        let com = ComEnvelope::novo(fonte, e, Arc::new(AtomicBool::new(false)));
        let saida: Vec<f32> = com.collect();
        for par in saida.chunks(2) {
            if par.len() == 2 {
                assert_eq!(par[0], par[1], "canais com ganho diferente");
            }
        }
    }
}
