//! Samples nos pads: tocar arquivo de áudio e gravar do microfone.
//!
//! Nada aqui bloqueia o caminho crítico. O arquivo é decodificado uma vez e
//! fica em memória; apertar o pad só empurra um buffer já pronto para a placa
//! de som. Quem executa continua sendo a thread de ações, nunca a de HID.
//!
//! O volume do sistema fica no módulo `audio`, que é outra coisa: aquilo lê e
//! escreve o volume do Windows, isto toca som.

pub mod envelope;
pub mod gravador;

use envelope::{ComEnvelope, Envelope};
use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Como o pad dispara o sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoDisparo {
    /// Apertou, toca até o fim. Apertar de novo recomeça do zero.
    #[default]
    AteOFim,
    /// Toca enquanto estiver apertado. Soltar entra na liberação do envelope.
    Segurando,
}

/// Um arquivo já decodificado, pronto para tocar sem tocar no disco.
#[derive(Debug, Clone)]
pub struct Amostra {
    pub canais: u16,
    pub taxa: u32,
    pub dados: Arc<Vec<f32>>,
}

impl Amostra {
    /// Duração do áudio.
    pub fn duracao(&self) -> std::time::Duration {
        let quadros = self.dados.len() as f64 / (self.canais.max(1) as f64 * self.taxa.max(1) as f64);
        std::time::Duration::from_secs_f64(quadros)
    }
}

/// Uma nota tocando. Soltar entra na liberação; o som acaba sozinho depois.
pub struct Voz {
    sink: Sink,
    solto: Arc<AtomicBool>,
}

impl Voz {
    /// O pad foi solto: entra na liberação do envelope e some.
    pub fn soltar(&self) {
        self.solto.store(true, Ordering::Relaxed);
    }

    /// Corta na hora, sem liberação. É o que o "pausar" e o fechar usam.
    pub fn cortar(&self) {
        self.sink.stop();
    }

    /// Se já acabou de tocar.
    pub fn acabou(&self) -> bool {
        self.sink.empty()
    }
}

/// A saída de áudio, com o cache dos arquivos já decodificados.
///
/// Abrir a placa de som pode falhar (máquina sem saída, driver fora do ar). Isso
/// não pode derrubar o MikroDeck: sem saída, tocar um sample só não faz nada.
/// Como saber se o arquivo em disco ainda é o mesmo que está em memória.
/// Guardar só o caminho não basta: gravar um sample novo por cima do antigo
/// mantém o caminho e troca o conteúdo, e o pad continuava tocando o som velho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Assinatura {
    tamanho: u64,
    modificado: Option<std::time::SystemTime>,
}

impl Assinatura {
    fn do_arquivo(caminho: &Path) -> Option<Self> {
        let meta = std::fs::metadata(caminho).ok()?;
        Some(Self {
            tamanho: meta.len(),
            modificado: meta.modified().ok(),
        })
    }
}

pub struct Saida {
    fluxo: Option<OutputStream>,
    cache: Mutex<HashMap<PathBuf, (Assinatura, Arc<Amostra>)>>,
}

impl Saida {
    /// Abre a saída padrão. Devolve uma `Saida` muda se não conseguir.
    pub fn nova() -> Self {
        let fluxo = match OutputStreamBuilder::open_default_stream() {
            Ok(f) => Some(f),
            Err(e) => {
                eprintln!("sem saida de audio: {e}");
                None
            }
        };
        Self {
            fluxo,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Se a placa de som abriu.
    pub fn disponivel(&self) -> bool {
        self.fluxo.is_some()
    }

    /// Decodifica o arquivo, ou devolve o que já está em memória.
    ///
    /// O que está em memória só vale enquanto o arquivo em disco não mudar. Sem
    /// essa conferência, gravar um sample novo por cima do antigo deixava o pad
    /// tocando o som velho para sempre.
    pub fn carregar(&self, caminho: &Path) -> Result<Arc<Amostra>, String> {
        let agora = Assinatura::do_arquivo(caminho);
        if let Ok(c) = self.cache.lock() {
            if let Some((assinatura, amostra)) = c.get(caminho) {
                if agora == Some(*assinatura) {
                    return Ok(amostra.clone());
                }
            }
        }
        let amostra = Arc::new(decodificar(caminho)?);
        if let (Ok(mut c), Some(assinatura)) = (self.cache.lock(), agora) {
            c.insert(caminho.to_path_buf(), (assinatura, amostra.clone()));
        }
        Ok(amostra)
    }

    /// Esquece o que está em memória. O `carregar` já percebe sozinho quando o
    /// arquivo muda; isto existe para liberar memória de um sample que saiu.
    pub fn esquecer(&self, caminho: &Path) {
        if let Ok(mut c) = self.cache.lock() {
            c.remove(caminho);
        }
    }

    /// Toca o arquivo. `volume` é de 0 a 1.
    pub fn tocar(
        &self,
        caminho: &Path,
        volume: f32,
        envelope: Envelope,
    ) -> Result<Voz, String> {
        let fluxo = self.fluxo.as_ref().ok_or("sem saida de audio")?;
        let amostra = self.carregar(caminho)?;
        let fonte = SamplesBuffer::new(
            amostra.canais.max(1),
            amostra.taxa.max(1),
            amostra.dados.as_slice(),
        );
        let solto = Arc::new(AtomicBool::new(false));
        let sink = Sink::connect_new(fluxo.mixer());
        sink.set_volume(volume.clamp(0.0, 1.0));
        sink.append(ComEnvelope::novo(fonte, envelope, solto.clone()));
        Ok(Voz { sink, solto })
    }
}

/// Lê um arquivo de áudio inteiro para memória, em f32 intercalado.
///
/// O `symphonia` por trás do rodio cobre wav, mp3, flac, ogg, m4a e aac. O que
/// ele não abrir vira erro com o nome do arquivo, para a interface mostrar.
pub fn decodificar(caminho: &Path) -> Result<Amostra, String> {
    let arquivo = std::fs::File::open(caminho)
        .map_err(|e| format!("nao consegui abrir {}: {e}", caminho.display()))?;
    let fonte = rodio::Decoder::try_from(arquivo)
        .map_err(|e| format!("nao consegui ler o audio de {}: {e}", caminho.display()))?;
    let canais = fonte.channels().max(1);
    let taxa = fonte.sample_rate().max(1);
    let dados: Vec<f32> = fonte.collect();
    if dados.is_empty() {
        return Err(format!("{} nao tem audio nenhum", caminho.display()));
    }
    Ok(Amostra {
        canais,
        taxa,
        dados: Arc::new(dados),
    })
}

/// O desenho do som: um pico por coluna, de 0 a 1.
///
/// Pico e não média: a média achata tudo e um som percussivo vira uma linha
/// reta. O que se quer ver é o contorno.
pub fn picos(amostra: &Amostra, colunas: usize) -> Vec<f32> {
    let colunas = colunas.clamp(1, 4000);
    let canais = amostra.canais.max(1) as usize;
    let quadros = amostra.dados.len() / canais;
    if quadros == 0 {
        return vec![0.0; colunas];
    }
    let mut maior = 0.0f32;
    let saida: Vec<f32> = (0..colunas)
        .map(|c| {
            let de = quadros * c / colunas;
            let ate = (quadros * (c + 1) / colunas).max(de + 1).min(quadros);
            let pico = amostra.dados[de * canais..(ate * canais).min(amostra.dados.len())]
                .iter()
                .fold(0.0f32, |m, a| m.max(a.abs()));
            maior = maior.max(pico);
            pico
        })
        .collect();
    // Normaliza pelo maior pico: um sample gravado baixo tem que aparecer.
    if maior <= f32::EPSILON {
        return saida;
    }
    saida.into_iter().map(|p| (p / maior).min(1.0)).collect()
}

/// A pasta onde ficam os samples gravados: `~/.mikrodeck/samples`.
pub fn pasta_dos_samples() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(".mikrodeck").join("samples")
}

/// Quem guarda as vozes tocando, uma por pad.
///
/// Fica no serviço, não na thread de ações: soltar o pad precisa alcançar a voz
/// que o aperto começou, e para isso os dois têm que passar pelo mesmo lugar.
pub struct Tocador {
    saida: Saida,
    vozes: HashMap<u8, (Voz, ModoDisparo)>,
}

impl Default for Tocador {
    fn default() -> Self {
        Self::novo()
    }
}

impl Tocador {
    pub fn novo() -> Self {
        Self {
            saida: Saida::nova(),
            vozes: HashMap::new(),
        }
    }

    pub fn saida(&self) -> &Saida {
        &self.saida
    }

    /// Toca no pad. Uma voz por pad: apertar de novo recomeça do zero, que é o
    /// que se espera de um pad de sample.
    pub fn tocar(
        &mut self,
        pad: u8,
        caminho: &Path,
        volume: f32,
        envelope: Envelope,
        modo: ModoDisparo,
    ) -> Result<(), String> {
        if let Some((antiga, _)) = self.vozes.remove(&pad) {
            antiga.cortar();
        }
        let voz = self.saida.tocar(caminho, volume, envelope)?;
        self.vozes.insert(pad, (voz, modo));
        Ok(())
    }

    /// O pad foi solto. Só entra na liberação quem está no modo segurando; o
    /// modo "até o fim" ignora, que é a diferença entre os dois.
    pub fn soltar(&mut self, pad: u8) {
        if let Some((voz, modo)) = self.vozes.get(&pad) {
            if *modo == ModoDisparo::Segurando {
                voz.soltar();
            }
        }
        self.limpar();
    }

    /// Corta tudo na hora. É o que o pausar usa: pausado, o aparelho volta a ser
    /// um Maschine comum, e som nenhum sobra tocando.
    pub fn cortar_tudo(&mut self) {
        for (_, (voz, _)) in self.vozes.drain() {
            voz.cortar();
        }
    }

    /// Joga fora as vozes que já acabaram.
    fn limpar(&mut self) {
        self.vozes.retain(|_, (voz, _)| !voz.acabou());
    }

    /// Quantas vozes estão tocando agora.
    pub fn tocando(&self) -> usize {
        self.vozes.values().filter(|(v, _)| !v.acabou()).count()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Escreve um WAV curto de verdade, para provar a decodificacao sem
    /// depender de arquivo de exemplo no repositorio.
    fn wav_de_teste(caminho: &Path, quadros: u32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(caminho, spec).unwrap();
        for i in 0..quadros {
            w.write_sample((i as i16 % 100) * 300).unwrap();
        }
        w.finalize().unwrap();
    }

    fn temporario(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("mikrodeck-teste-{nome}.wav"));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn decodifica_um_wav_de_verdade() {
        let p = temporario("decodifica");
        wav_de_teste(&p, 8000);
        let a = decodificar(&p).unwrap();
        assert_eq!(a.canais, 1);
        assert_eq!(a.taxa, 8000);
        assert_eq!(a.dados.len(), 8000);
        // Um segundo de audio, com folga para o arredondamento do decodificador.
        let d = a.duracao().as_secs_f32();
        assert!((d - 1.0).abs() < 0.05, "{d} s");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn arquivo_que_nao_existe_da_erro_com_o_nome() {
        let e = decodificar(Path::new("C:/nao/existe/isso.wav")).unwrap_err();
        assert!(e.contains("isso.wav"), "{e}");
    }

    #[test]
    fn arquivo_que_nao_e_audio_da_erro_em_vez_de_estourar() {
        let p = std::env::temp_dir().join("mikrodeck-teste-nao-e-audio.wav");
        std::fs::write(&p, b"isto aqui nao e audio nenhum").unwrap();
        assert!(decodificar(&p).is_err());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn gravar_por_cima_troca_o_som_em_vez_de_repetir_o_antigo() {
        // O bug: o cache guardava por caminho, e gravar um sample novo por cima
        // mantinha o caminho. O pad continuava tocando o som velho.
        let p = temporario("regravado");
        wav_de_teste(&p, 800);
        let s = Saida::nova();
        let antigo = s.carregar(&p).unwrap();
        assert_eq!(antigo.dados.len(), 800);

        // Grava outro som no mesmo caminho, com duracao diferente.
        // O carimbo de tempo do sistema de arquivos tem resolucao grossa, entao
        // o tamanho diferente e o que garante a deteccao neste teste.
        std::thread::sleep(std::time::Duration::from_millis(20));
        wav_de_teste(&p, 2400);

        let novo = s.carregar(&p).unwrap();
        assert_eq!(novo.dados.len(), 2400, "devolveu o audio antigo");
        assert!(!Arc::ptr_eq(&antigo, &novo));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn arquivo_intocado_nao_e_decodificado_de_novo() {
        let p = temporario("intocado");
        wav_de_teste(&p, 800);
        let s = Saida::nova();
        let a = s.carregar(&p).unwrap();
        let b = s.carregar(&p).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "decodificou duas vezes o mesmo arquivo");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn o_cache_devolve_a_mesma_amostra_e_esquecer_limpa() {
        let p = temporario("cache");
        wav_de_teste(&p, 800);
        let s = Saida::nova();
        let a = s.carregar(&p).unwrap();
        let b = s.carregar(&p).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "o cache decodificou duas vezes");
        s.esquecer(&p);
        let c = s.carregar(&p).unwrap();
        assert!(!Arc::ptr_eq(&a, &c), "esquecer nao limpou o cache");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn os_picos_desenham_o_contorno_do_som() {
        // Silencio, som alto, silencio: o desenho tem que mostrar isso.
        let mut dados = vec![0.0f32; 1000];
        dados.extend((0..1000).map(|i| (i as f32 / 5.0).sin() * 0.5));
        dados.extend(vec![0.0f32; 1000]);
        let a = Amostra { canais: 1, taxa: 3000, dados: Arc::new(dados) };
        let p = picos(&a, 30);
        assert_eq!(p.len(), 30);
        assert!(p[..8].iter().all(|x| *x < 0.05), "o comeco devia ser mudo");
        assert!(p[12..18].iter().any(|x| *x > 0.9), "o meio devia ser alto");
        assert!(p[24..].iter().all(|x| *x < 0.05), "o fim devia ser mudo");
        assert!(p.iter().all(|x| (0.0..=1.0).contains(x)));
    }

    #[test]
    fn um_som_baixinho_aparece_no_desenho() {
        // Normalizado pelo maior pico, senao um sample gravado baixo viraria
        // uma linha reta e a pessoa acharia que nao gravou nada.
        let a = Amostra {
            canais: 1,
            taxa: 1000,
            dados: Arc::new((0..1000).map(|i| (i as f32 / 5.0).sin() * 0.01).collect()),
        };
        let p = picos(&a, 20);
        assert!(p.iter().any(|x| *x > 0.9), "o som baixo sumiu: {p:?}");
    }

    #[test]
    fn os_picos_aguentam_som_curto_e_pedido_grande() {
        // Mais colunas do que quadros: nao pode dividir por zero nem sair da faixa.
        let a = Amostra { canais: 2, taxa: 1000, dados: Arc::new(vec![0.5; 8]) };
        let p = picos(&a, 100);
        assert_eq!(p.len(), 100);
        assert!(p.iter().all(|x| x.is_finite()));
        let vazia = Amostra { canais: 1, taxa: 1000, dados: Arc::new(vec![]) };
        assert_eq!(picos(&vazia, 10), vec![0.0; 10]);
    }

    #[test]
    fn a_pasta_dos_samples_fica_dentro_do_mikrodeck() {
        let p = pasta_dos_samples();
        assert!(p.ends_with("samples"));
        assert!(p.to_string_lossy().contains(".mikrodeck"));
    }
}
