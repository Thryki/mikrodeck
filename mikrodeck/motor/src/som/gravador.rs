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

    /// Para e espera o arquivo fechar. Devolve o caminho do WAV.
    pub fn parar(mut self) -> Result<PathBuf, String> {
        self.parar.store(true, Ordering::Relaxed);
        match self.fim.take() {
            Some(f) => match f.recv() {
                Ok(Ok(())) => Ok(self.caminho.clone()),
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

    #[test]
    fn o_limite_de_gravacao_e_curto_de_proposito() {
        // O Davi pediu trinta a quarenta segundos, um minuto no maximo.
        assert_eq!(SEGUNDOS_MAXIMOS, 60);
    }
}
