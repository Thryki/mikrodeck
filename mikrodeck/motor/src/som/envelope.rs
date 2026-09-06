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

/// Rampa contra estalo, em milissegundos.
///
/// Um sample raramente começa e acaba no zero da onda. Sem uma rampa, o salto
/// de zero para o meio da onda é um degrau, e degrau em áudio é estalo. Três
/// milissegundos são curtos demais para mudar o ataque que a pessoa ouve e
/// longos o bastante para matar o estalo.
pub const ANTIESTALO_MS: f32 = 3.0;

/// Quanto dura o corte suave de uma voz que foi substituída.
///
/// Apertar o mesmo pad de novo mata a voz anterior. Matar no meio da onda dá
/// estalo, que era o clique que aparecia ao apertar rápido. Dez milissegundos
/// somem no meio do som novo.
pub const CORTE_MS: f32 = 10.0;

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
    /// Pedido de corte suave: a voz foi substituída e tem que sair de cena.
    corte: Arc<AtomicBool>,
    /// Quadro em que o corte foi pedido, e o ganho naquele instante.
    corte_em: Option<(u64, f32)>,
    /// Quantos quadros o áudio tem, quando dá para saber. Serve para a rampa
    /// do fim: sem ela, um sample que acaba no meio da onda estala.
    total: Option<u64>,
}

impl<S> ComEnvelope<S>
where
    S: rodio::Source,
{
    pub fn novo(fonte: S, envelope: Envelope, solto: Arc<AtomicBool>) -> Self {
        Self::com_corte(fonte, envelope, solto, Arc::new(AtomicBool::new(false)))
    }

    pub fn com_corte(
        fonte: S,
        envelope: Envelope,
        solto: Arc<AtomicBool>,
        corte: Arc<AtomicBool>,
    ) -> Self {
        let taxa = fonte.sample_rate().max(1);
        let canais = fonte.channels().max(1);
        let total = fonte
            .total_duration()
            .map(|d| (d.as_secs_f64() * taxa as f64).round() as u64);
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
            corte,
            corte_em: None,
            total,
        }
    }

    /// A rampa das pontas: sobe nos primeiros milissegundos e desce nos
    /// últimos. Devolve 1 no meio do som, que é quase sempre.
    fn antiestalo(&self, quadro: u64) -> f32 {
        let janela = (ANTIESTALO_MS / 1000.0 * self.taxa as f32).max(1.0);
        let entrada = (quadro as f32 / janela).min(1.0);
        let saida = match self.total {
            Some(total) if total > 0 => {
                ((total.saturating_sub(quadro)) as f32 / janela).min(1.0)
            }
            _ => 1.0,
        };
        entrada.min(saida)
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
        let do_envelope = match self.solto_em {
            None => Some(segurando),
            Some((quadro_do_solto, nivel)) => {
                let desde = self.instante(self.quadro.saturating_sub(quadro_do_solto));
                if self.envelope.terminou(desde) {
                    None
                } else {
                    Some(self.envelope.ganho_soltando(nivel, desde))
                }
            }
        }?;

        // O corte suave ganha do envelope: ele existe para tirar esta voz de
        // cena depressa, sem estalo, quando outra tomou o lugar dela.
        if self.corte_em.is_none() && self.corte.load(Ordering::Relaxed) {
            self.corte_em = Some((self.quadro, do_envelope));
        }
        let ganho = match self.corte_em {
            None => do_envelope,
            Some((quadro_do_corte, nivel)) => {
                let ms = self.instante(self.quadro.saturating_sub(quadro_do_corte)).as_secs_f32()
                    * 1000.0;
                if ms >= CORTE_MS {
                    return None;
                }
                (nivel * (1.0 - ms / CORTE_MS)).min(do_envelope)
            }
        };
        Some(ganho * self.antiestalo(self.quadro))
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
        // 100 quadros com o pad apertado. Os primeiros milissegundos sobem pela
        // rampa antiestalo; depois dela o ganho e cheio.
        let inicio: Vec<f32> = (0..100).map(|_| com.next().unwrap()).collect();
        assert!(inicio[0] < 0.5, "a rampa devia comecar baixo: {}", inicio[0]);
        assert!(
            inicio[50..].iter().all(|g| (*g - 1.0).abs() < 0.001),
            "depois da rampa o ganho devia ser cheio"
        );
        solto.store(true, Ordering::Relaxed);
        // A liberacao dura 100 ms, que a 1000 Hz sao 100 quadros.
        let restantes = com.count();
        assert_eq!(restantes, 100, "a liberacao devia durar 100 quadros");
    }

    /// O maior salto entre duas amostras vizinhas. Estalo e degrau: se nenhum
    /// passo e grande, nao ha estalo.
    fn maior_degrau(v: &[f32]) -> f32 {
        v.windows(2).fold(0.0f32, |m, p| m.max((p[1] - p[0]).abs()))
    }

    /// Sinal constante em 1: o pior caso possivel para estalo, porque qualquer
    /// corte seco vira um degrau de 1 inteiro.
    fn constante(quadros: usize) -> rodio::buffer::SamplesBuffer {
        rodio::buffer::SamplesBuffer::new(1, 48000, vec![1.0f32; quadros])
    }

    #[test]
    fn o_som_entra_e_sai_sem_degrau() {
        // Com ataque zero, sem a rampa antiestalo a primeira amostra saltaria
        // de nada para 1, e a ultima de 1 para nada.
        let com = ComEnvelope::novo(
            constante(48000),
            Envelope::default(),
            Arc::new(AtomicBool::new(false)),
        );
        let saida: Vec<f32> = com.collect();
        assert!(!saida.is_empty());
        assert!(saida[0].abs() < 0.05, "comecou em {}", saida[0]);
        assert!(
            saida[saida.len() - 1].abs() < 0.2,
            "acabou em {}",
            saida[saida.len() - 1]
        );
        assert!(saida.iter().any(|a| *a > 0.99), "o meio devia estar cheio");
        assert!(
            maior_degrau(&saida) < 0.05,
            "degrau de {} no som",
            maior_degrau(&saida)
        );
    }

    #[test]
    fn o_corte_suave_desce_ate_zero_em_vez_de_estalar() {
        // Era o clique do Davi: apertar o pad de novo matava a voz anterior no
        // meio da onda.
        let corte = Arc::new(AtomicBool::new(false));
        let mut com = ComEnvelope::com_corte(
            constante(48000),
            Envelope::default(),
            Arc::new(AtomicBool::new(false)),
            corte.clone(),
        );
        let mut saida: Vec<f32> = Vec::new();
        for _ in 0..4800 {
            saida.push(com.next().unwrap());
        }
        corte.store(true, Ordering::Relaxed);
        let cauda: Vec<f32> = com.collect();

        // Dez milissegundos a 48 kHz sao 480 quadros, com folga de um.
        assert!(
            (470..=490).contains(&cauda.len()),
            "a rampa do corte durou {} quadros",
            cauda.len()
        );
        assert!(cauda[0] > 0.9, "a rampa devia comecar de onde estava");
        assert!(
            *cauda.last().unwrap() < 0.05,
            "acabou em {}",
            cauda.last().unwrap()
        );
        saida.extend(cauda);
        assert!(
            maior_degrau(&saida) < 0.05,
            "degrau de {} no corte",
            maior_degrau(&saida)
        );
    }

    #[test]
    fn o_corte_suave_nunca_aumenta_o_volume() {
        // Cortar uma voz que ja estava baixa nao pode levantar ela de volta.
        let corte = Arc::new(AtomicBool::new(false));
        let envelope = Envelope {
            ataque_ms: 0,
            decaimento_ms: 100,
            sustentacao: 0.2,
            ..Envelope::default()
        };
        let mut com = ComEnvelope::com_corte(
            constante(48000),
            envelope,
            Arc::new(AtomicBool::new(false)),
            corte.clone(),
        );
        for _ in 0..24000 {
            com.next();
        }
        let antes = com.next().unwrap();
        corte.store(true, Ordering::Relaxed);
        let cauda: Vec<f32> = com.collect();
        assert!(cauda[0] <= antes + 0.01, "{} subiu para {}", antes, cauda[0]);
    }

    #[test]
    fn a_rampa_antiestalo_nao_engole_o_ataque_configurado() {
        // Tres milissegundos nao podem virar o ataque de quem pediu 500 ms.
        let e = Envelope { ataque_ms: 500, ..Envelope::default() };
        let com = ComEnvelope::novo(constante(48000), e, Arc::new(AtomicBool::new(false)));
        let saida: Vec<f32> = com.collect();
        // No meio do ataque configurado o ganho tem que estar no meio, nao no topo.
        let meio = saida[48000 / 4];
        assert!((meio - 0.5).abs() < 0.05, "ganho {meio} no meio do ataque");
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
