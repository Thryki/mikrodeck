//! Gravar um sample pelo microfone.
//!
//! O limite curto é de propósito, como o Davi pediu: trinta a quarenta
//! segundos, um minuto no máximo. Isso mantém o arquivo pequeno, a memória
//! baixa e o sample utilizável como sample, não como podcast.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

/// Teto duro de gravação. Acima disso o gravador para sozinho.
pub const SEGUNDOS_MAXIMOS: u32 = 60;

/// O que a interface mostra no seletor de microfone.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Microfone {
    pub nome: String,
    /// Se é o que o Windows usa por padrão.
    pub padrao: bool,
}

/// Lista os microfones disponíveis. Lista vazia quando não há nenhum.
pub fn microfones() -> Vec<Microfone> {
    let host = cpal::default_host();
    let padrao = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();
    let Ok(dispositivos) = host.input_devices() else {
        return Vec::new();
    };
    dispositivos
        .filter_map(|d| d.name().ok())
        .map(|nome| Microfone {
            padrao: nome == padrao,
            nome,
        })
        .collect()
}

fn achar(nome: Option<&str>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    match nome {
        None | Some("") => host
            .default_input_device()
            .ok_or_else(|| "nao achei microfone nenhum".to_string()),
        Some(procurado) => host
            .input_devices()
            .map_err(|e| format!("nao consegui listar os microfones: {e}"))?
            .find(|d| d.name().map(|n| n == procurado).unwrap_or(false))
            .ok_or_else(|| format!("nao achei o microfone {procurado:?}")),
    }
}

/// Uma gravação em curso. Parar (ou soltar) fecha o WAV.
pub struct Gravacao {
    parar: Arc<AtomicBool>,
    quadros: Arc<AtomicU32>,
    taxa: u32,
    caminho: PathBuf,
    /// Avisa quando a thread terminou de fechar o arquivo.
    fim: Option<mpsc::Receiver<Result<(), String>>>,
}

impl Gravacao {
    /// Começa a gravar no arquivo dado. Para sozinha no limite de segundos.
    ///
    /// A gravação roda numa thread própria: o `Stream` do cpal não atravessa
    /// threads, então ele nasce e morre lá dentro.
    pub fn comecar(
        microfone: Option<&str>,
        caminho: &Path,
        segundos: u32,
    ) -> Result<Self, String> {
        let segundos = segundos.clamp(1, SEGUNDOS_MAXIMOS);
        let dispositivo = achar(microfone)?;
        let config = dispositivo
            .default_input_config()
            .map_err(|e| format!("microfone sem formato de entrada: {e}"))?;
        let taxa = config.sample_rate().0;
        let canais = config.channels();

        if let Some(pai) = caminho.parent() {
            std::fs::create_dir_all(pai)
                .map_err(|e| format!("nao consegui criar {}: {e}", pai.display()))?;
        }

        let parar = Arc::new(AtomicBool::new(false));
        let quadros = Arc::new(AtomicU32::new(0));
        let (avisar, fim) = mpsc::channel();

        let caminho_thread = caminho.to_path_buf();
        let parar_thread = parar.clone();
        let quadros_thread = quadros.clone();
        let formato = config.sample_format();
        let config_stream: cpal::StreamConfig = config.into();

        std::thread::Builder::new()
            .name("mikrodeck-gravacao".into())
            .spawn(move || {
                let r = gravar(
                    &dispositivo,
                    &config_stream,
                    formato,
                    &caminho_thread,
                    canais,
                    taxa,
                    segundos,
                    parar_thread,
                    quadros_thread,
                );
                let _ = avisar.send(r);
            })
            .map_err(|e| format!("nao consegui subir a thread de gravacao: {e}"))?;

        Ok(Self {
            parar,
            quadros,
            taxa,
            caminho: caminho.to_path_buf(),
            fim: Some(fim),
        })
    }

    /// Quanto já foi gravado.
    pub fn duracao(&self) -> Duration {
        Duration::from_secs_f64(self.quadros.load(Ordering::Relaxed) as f64 / self.taxa.max(1) as f64)
    }

    /// Se a thread já fechou o arquivo, por limite de tempo ou por `parar`.
    pub fn terminou(&self) -> bool {
        self.fim
            .as_ref()
            .map(|f| matches!(f.try_recv(), Err(mpsc::TryRecvError::Disconnected)))
            .unwrap_or(true)
    }

    /// Para, espera o arquivo fechar e apara o silêncio das pontas.
    /// Devolve o caminho do WAV.
    pub fn parar(mut self) -> Result<PathBuf, String> {
        self.parar.store(true, Ordering::Relaxed);
        match self.fim.take() {
            Some(f) => match f.recv() {
                Ok(Ok(())) => {
                    // O silencio antes de a pessoa falar e depois de ela parar
                    // morre aqui, sem ela precisar pedir.
                    if let Err(e) = aparar_silencio(&self.caminho) {
                        eprintln!("nao consegui aparar o silencio: {e}");
                    }
                    Ok(self.caminho.clone())
                }
                Ok(Err(e)) => Err(e),
                Err(_) => Err("a gravacao morreu sem dizer o porque".into()),
            },
            None => Ok(self.caminho.clone()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn gravar(
    dispositivo: &cpal::Device,
    config: &cpal::StreamConfig,
    formato: cpal::SampleFormat,
    caminho: &Path,
    canais: u16,
    taxa: u32,
    segundos: u32,
    parar: Arc<AtomicBool>,
    quadros: Arc<AtomicU32>,
) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: canais,
        sample_rate: taxa,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let escritor = hound::WavWriter::create(caminho, spec)
        .map_err(|e| format!("nao consegui criar {}: {e}", caminho.display()))?;
    let escritor = Arc::new(std::sync::Mutex::new(Some(escritor)));

    let limite = taxa.saturating_mul(segundos);
    let erro = |e| eprintln!("erro na gravacao: {e}");

    // O cpal entrega o formato nativo do microfone; convertemos tudo para f32.
    let escrever = {
        let escritor = escritor.clone();
        let quadros = quadros.clone();
        move |amostras: &[f32]| {
            let Ok(mut guarda) = escritor.lock() else { return };
            let Some(w) = guarda.as_mut() else { return };
            for a in amostras {
                let _ = w.write_sample(*a);
            }
            quadros.fetch_add((amostras.len() / canais.max(1) as usize) as u32, Ordering::Relaxed);
        }
    };

    let stream = match formato {
        cpal::SampleFormat::F32 => dispositivo.build_input_stream(
            config,
            move |dados: &[f32], _: &_| escrever(dados),
            erro,
            None,
        ),
        cpal::SampleFormat::I16 => dispositivo.build_input_stream(
            config,
            move |dados: &[i16], _: &_| {
                let f: Vec<f32> = dados.iter().map(|a| *a as f32 / i16::MAX as f32).collect();
                escrever(&f)
            },
            erro,
            None,
        ),
        cpal::SampleFormat::U16 => dispositivo.build_input_stream(
            config,
            move |dados: &[u16], _: &_| {
                let f: Vec<f32> = dados
                    .iter()
                    .map(|a| (*a as f32 - 32768.0) / 32768.0)
                    .collect();
                escrever(&f)
            },
            erro,
            None,
        ),
        outro => return Err(format!("formato de microfone que eu nao trato: {outro:?}")),
    }
    .map_err(|e| format!("nao consegui abrir o microfone: {e}"))?;

    stream
        .play()
        .map_err(|e| format!("nao consegui comecar a gravar: {e}"))?;

    while !parar.load(Ordering::Relaxed) && quadros.load(Ordering::Relaxed) < limite {
        std::thread::sleep(Duration::from_millis(25));
    }
    drop(stream);

    let mut guarda = escritor.lock().map_err(|_| "escritor travado".to_string())?;
    if let Some(w) = guarda.take() {
        w.finalize()
            .map_err(|e| format!("nao consegui fechar o arquivo: {e}"))?;
    }
    Ok(())
}

/// Abaixo disto conta como silêncio. Uns -46 dB: baixo o bastante para pegar
/// só o chiado do microfone, alto o bastante para não comer o começo suave de
/// um som de verdade.
pub const LIMIAR_DE_SILENCIO: f32 = 0.005;

/// Quanto de silêncio fica de cada lado, em milissegundos. Cortar rente dá
/// estalo no começo e engole a cauda no fim.
pub const MARGEM_MS: u32 = 30;

/// Onde começa e onde acaba o som de verdade, em quadros.
///
/// Devolve `None` quando o arquivo inteiro é silêncio. Fora da função de
/// arquivo de propósito: assim o teste roda sobre um vetor, sem tocar no disco.
pub fn faixa_com_som(
    amostras: &[f32],
    canais: u16,
    taxa: u32,
) -> Option<(usize, usize)> {
    let canais = canais.max(1) as usize;
    let quadros = amostras.len() / canais;
    if quadros == 0 {
        return None;
    }
    let alto = |quadro: usize| {
        amostras[quadro * canais..(quadro + 1) * canais]
            .iter()
            .any(|a| a.abs() > LIMIAR_DE_SILENCIO)
    };
    let primeiro = (0..quadros).find(|q| alto(*q))?;
    let ultimo = (0..quadros).rev().find(|q| alto(*q))?;
    let margem = (taxa.max(1) as u64 * MARGEM_MS as u64 / 1000) as usize;
    Some((
        primeiro.saturating_sub(margem),
        (ultimo + margem + 1).min(quadros),
    ))
}

/// Reescreve o WAV sem o silêncio das pontas.
///
/// É o que o Davi pediu: gravou, o silêncio morre sozinho. Se o arquivo for só
/// silêncio, ele fica como está: apagar o que a pessoa acabou de gravar seria
/// pior do que deixar um arquivo mudo que ela pode ouvir e refazer.
pub fn aparar_silencio(caminho: &Path) -> Result<Duration, String> {
    let mut leitor = hound::WavReader::open(caminho)
        .map_err(|e| format!("nao consegui reabrir {}: {e}", caminho.display()))?;
    let spec = leitor.spec();
    let amostras: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => leitor.samples::<f32>().filter_map(|a| a.ok()).collect(),
        hound::SampleFormat::Int => {
            let escala = (1i64 << (spec.bits_per_sample - 1)) as f32;
            leitor
                .samples::<i32>()
                .filter_map(|a| a.ok())
                .map(|a| a as f32 / escala)
                .collect()
        }
    };
    let canais = spec.channels.max(1) as usize;
    let Some((inicio, fim)) = faixa_com_som(&amostras, spec.channels, spec.sample_rate) else {
        return Ok(Duration::ZERO);
    };
    let recorte = &amostras[inicio * canais..(fim * canais).min(amostras.len())];
    let mut w = hound::WavWriter::create(caminho, spec)
        .map_err(|e| format!("nao consegui reescrever {}: {e}", caminho.display()))?;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for a in recorte {
                w.write_sample(*a).map_err(|e| e.to_string())?;
            }
        }
        hound::SampleFormat::Int => {
            let escala = (1i64 << (spec.bits_per_sample - 1)) as f32;
            for a in recorte {
                let v = (a * escala).clamp(-escala, escala - 1.0) as i32;
                w.write_sample(v).map_err(|e| e.to_string())?;
            }
        }
    }
    w.finalize().map_err(|e| e.to_string())?;
    Ok(Duration::from_secs_f64(
        (fim - inicio) as f64 / spec.sample_rate.max(1) as f64,
    ))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn listar_microfones_nao_estoura_sem_microfone() {
        // Numa maquina sem entrada nenhuma a lista e vazia, e isso nao e erro.
        let _ = microfones();
    }

    #[test]
    fn microfone_que_nao_existe_da_erro_com_o_nome() {
        let Err(e) = achar(Some("Microfone Que Nao Existe 12345")) else {
            panic!("achou um microfone que nao existe");
        };
        assert!(e.contains("Microfone Que Nao Existe"), "{e}");
    }

    /// Meio segundo de silencio, meio de som, meio de silencio.
    fn com_silencio_nas_pontas(taxa: u32) -> Vec<f32> {
        let meio = (taxa / 2) as usize;
        let mut v = vec![0.0f32; meio];
        for i in 0..meio {
            v.push((i as f32 / 20.0).sin() * 0.5);
        }
        v.extend(std::iter::repeat_n(0.0f32, meio));
        v
    }

    #[test]
    fn a_faixa_com_som_pula_o_silencio_das_pontas() {
        let taxa = 8000;
        let v = com_silencio_nas_pontas(taxa);
        let (inicio, fim) = faixa_com_som(&v, 1, taxa).unwrap();
        let margem = (taxa * MARGEM_MS / 1000) as usize;
        // Comeca uma margem antes do som e acaba uma margem depois. O seno
        // cruza o zero, entao o primeiro quadro alto pode ser o 4000 ou o 4001.
        assert!(
            (4000 - margem..=4002 - margem).contains(&inicio),
            "inicio em {inicio}"
        );
        assert!(
            (8000 + margem..=8000 + margem + 2).contains(&fim),
            "fim em {fim}"
        );
    }

    #[test]
    fn a_margem_nao_estoura_os_limites_do_arquivo() {
        // Som do primeiro ao ultimo quadro: a margem nao pode sair do arquivo.
        let v: Vec<f32> = (0..1000).map(|i| (i as f32).sin() * 0.5).collect();
        let (inicio, fim) = faixa_com_som(&v, 1, 8000).unwrap();
        assert_eq!(inicio, 0);
        assert_eq!(fim, 1000);
    }

    #[test]
    fn arquivo_so_de_silencio_nao_tem_faixa() {
        assert_eq!(faixa_com_som(&vec![0.0; 1000], 1, 8000), None);
        assert_eq!(faixa_com_som(&[], 1, 8000), None);
        // Chiado abaixo do limiar tambem conta como silencio.
        assert_eq!(faixa_com_som(&vec![0.001; 1000], 1, 8000), None);
    }

    #[test]
    fn o_estereo_conta_quadro_e_nao_amostra() {
        // Dois canais: o som comeca no quadro 2, que e a amostra 4.
        let v = vec![0.0, 0.0, 0.0, 0.0, 0.9, 0.9, 0.0, 0.0];
        let (inicio, fim) = faixa_com_som(&v, 2, 1000).unwrap();
        assert_eq!((inicio, fim), (0, 4), "contou amostra em vez de quadro");
    }

    #[test]
    fn aparar_encurta_o_arquivo_de_verdade() {
        let taxa = 8000;
        let caminho = std::env::temp_dir().join("mikrodeck-teste-aparar.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: taxa,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut w = hound::WavWriter::create(&caminho, spec).unwrap();
        for a in com_silencio_nas_pontas(taxa) {
            w.write_sample(a).unwrap();
        }
        w.finalize().unwrap();

        let antes = hound::WavReader::open(&caminho).unwrap().duration();
        let duracao = aparar_silencio(&caminho).unwrap();
        let depois = hound::WavReader::open(&caminho).unwrap().duration();

        assert!(depois < antes, "nao cortou nada: {antes} -> {depois}");
        // Sobra o meio segundo de som mais as duas margens.
        let esperado = taxa / 2 + 2 * (taxa * MARGEM_MS / 1000);
        assert!(
            depois.abs_diff(esperado) < 100,
            "sobrou {depois}, esperava perto de {esperado}"
        );
        assert!((duracao.as_secs_f32() - 0.56).abs() < 0.05, "{duracao:?}");
        let _ = std::fs::remove_file(&caminho);
    }

    #[test]
    fn aparar_um_arquivo_mudo_deixa_ele_como_esta() {
        // Apagar o que a pessoa acabou de gravar seria pior do que devolver um
        // arquivo mudo que ela pode ouvir e refazer.
        let caminho = std::env::temp_dir().join("mikrodeck-teste-mudo.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut w = hound::WavWriter::create(&caminho, spec).unwrap();
        for _ in 0..8000 {
            w.write_sample(0.0f32).unwrap();
        }
        w.finalize().unwrap();
        assert_eq!(aparar_silencio(&caminho).unwrap(), Duration::ZERO);
        assert_eq!(hound::WavReader::open(&caminho).unwrap().duration(), 8000);
        let _ = std::fs::remove_file(&caminho);
    }

    #[test]
    fn o_limite_de_gravacao_e_curto_de_proposito() {
        // O Davi pediu trinta a quarenta segundos, um minuto no maximo.
        assert_eq!(SEGUNDOS_MAXIMOS, 60);
    }
}
