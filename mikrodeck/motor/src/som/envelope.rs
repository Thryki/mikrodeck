//! O envelope de um sample: ataque, decaimento, sustentação e liberação.
//!
//! É uma fonte que embrulha outra e multiplica o ganho quadro a quadro. Não
//! fala com o aparelho nem com o relógio: conta quadros, então o teste roda sem
//! placa de som e sem esperar tempo nenhum passar.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// O envelope, em milissegundos, com a sustentação de 0 a 1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// Quanto o som demora para chegar no volume cheio.
    #[serde(default = "zero")]
    pub ataque_ms: u32,
    /// Quanto ele demora para cair do cheio até a sustentação.
    #[serde(default = "zero")]
    pub decaimento_ms: u32,
    /// O volume em que ele se segura, de 0 a 1.
    #[serde(default = "um")]
    pub sustentacao: f32,
    /// Quanto ele demora para sumir depois que o pad é solto.
    #[serde(default = "cem")]
    pub liberacao_ms: u32,
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

impl Default for Envelope {
    /// O padrão não muda nada no som: entra cheio, fica cheio, e some rápido
    /// quando solta, só para não estalar.
    fn default() -> Self {
        Self {
            ataque_ms: 0,
            decaimento_ms: 0,
            sustentacao: 1.0,
            liberacao_ms: 100,
        }
    }
}

impl Envelope {
    /// Ganho de 0 a 1 no instante `t`, com o pad ainda apertado.
    pub fn ganho_segurando(&self, t: Duration) -> f32 {
        let ms = t.as_secs_f32() * 1000.0;
        let s = self.sustentacao.clamp(0.0, 1.0);
        let a = self.ataque_ms as f32;
        let d = self.decaimento_ms as f32;
        if ms < a {
            // Sem ataque, `a` é zero e este ramo nem roda.
            ms / a
        } else if ms < a + d {
            1.0 - (1.0 - s) * (ms - a) / d
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
        (nivel_no_solto * (1.0 - ms / r)).max(0.0)
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
            ataque_ms: 0,
            decaimento_ms: 0,
            sustentacao: 1.0,
            liberacao_ms: 100,
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
