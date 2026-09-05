//! Módulo `hid`: a única parte do motor que fala com o aparelho.
//!
//! Nada fora daqui toca no HID. Quem usa este módulo recebe eventos já decodificados
//! por um canal, e pinta os LEDs mexendo num `FrameLeds` compartilhado.
//!
//! Duas threads:
//! - leitura: fica bloqueada esperando o aparelho e manda `Evento` pelo canal.
//! - escrita: acorda num tick fixo e só escreve se o frame mudou.

pub mod eventos;
pub mod frame;
pub mod protocolo;

pub use eventos::Evento;
pub use frame::FrameLeds;
pub use protocolo::{BrilhoBotao, Cor};

use hidapi::HidApi;
use protocolo::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Erros que o módulo pode devolver na abertura.
#[derive(Debug)]
pub enum ErroHid {
    /// O hidapi não iniciou.
    Api(String),
    /// O aparelho não está conectado, ou está sendo segurado por outro processo.
    NaoEncontrado(String),
}

impl std::fmt::Display for ErroHid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErroHid::Api(e) => write!(f, "falha ao iniciar o hidapi: {e}"),
            ErroHid::NaoEncontrado(e) => write!(
                f,
                "Maschine Mikro MK3 não encontrado ou ocupado: {e}\n\
                 Confira se o cabo está conectado. Se for a primeira vez neste aparelho,\n\
                 abra o Maschine 2 como administrador uma vez (ver docs/spike-hid.md)."
            ),
        }
    }
}

impl std::error::Error for ErroHid {}

/// Conexão viva com o aparelho.
///
/// Enquanto este objeto existir, as duas threads rodam. Ao ser descartado, elas param.
pub struct Aparelho {
    /// Frame de LEDs compartilhado. Mexa nele e a thread de escrita manda no próximo tick.
    pub leds: Arc<Mutex<FrameLeds>>,
    /// Tela compartilhada. Mesma ideia: desenhe e a thread de escrita manda.
    pub tela: Arc<Mutex<crate::render::Tela>>,
    rodando: Arc<AtomicBool>,
    escritor: Option<thread::JoinHandle<()>>,
    leitor: Option<thread::JoinHandle<()>>,
}

impl Aparelho {
    /// Abre o aparelho e sobe as duas threads.
    ///
    /// `hz` é a taxa da thread de escrita, entre 30 e 60 na prática. O aparelho aceita
    /// cerca de 37 escritas por segundo antes de começar a recusar, então 30 é seguro.
    ///
    /// Devolve a conexão e o canal por onde os eventos chegam.
    pub fn abrir(hz: u32) -> Result<(Self, Receiver<Evento>), ErroHid> {
        let api = HidApi::new().map_err(|e| ErroHid::Api(e.to_string()))?;
        let dev_leitura = api
            .open(VID, PID)
            .map_err(|e| ErroHid::NaoEncontrado(e.to_string()))?;
        // Um segundo handle para a escrita: assim a thread de leitura pode ficar
        // bloqueada sem segurar a de escrita.
        let dev_escrita = api
            .open(VID, PID)
            .map_err(|e| ErroHid::NaoEncontrado(e.to_string()))?;

        let (envia, recebe) = mpsc::channel();
        let leds = Arc::new(Mutex::new(FrameLeds::novo()));
        let tela = Arc::new(Mutex::new(crate::render::Tela::nova()));
        let rodando = Arc::new(AtomicBool::new(true));

        let leitor = {
            let rodando = rodando.clone();
            thread::Builder::new()
                .name("mikrodeck-hid-leitura".into())
                .spawn(move || {
                    let mut decodificador = Decodificador::novo();
                    // Buffer folgado: o report de pads tem 64 bytes, mas o aparelho
                    // pode mandar pacotes maiores. Buffer curto faz a leitura estourar
                    // e o pacote se perder em silêncio.
                    let mut buf = [0u8; 256];
                    while rodando.load(Ordering::Relaxed) {
                        match dev_leitura.read_timeout(&mut buf, 200) {
                            Ok(0) => continue,
                            Ok(n) => {
                                for evento in decodificador.decodificar(&buf[..n]) {
                                    if envia.send(evento).is_err() {
                                        return; // ninguém mais escutando
                                    }
                                }
                            }
                            Err(_) => {
                                let _ = envia.send(Evento::Desconectado);
                                return;
                            }
                        }
                    }
                })
                .expect("subir a thread de leitura")
        };

        let escritor = {
            let rodando = rodando.clone();
            let leds = leds.clone();
            let tela = tela.clone();
            let intervalo = Duration::from_micros(1_000_000 / hz.clamp(1, 60) as u64);
            thread::Builder::new()
                .name("mikrodeck-hid-escrita".into())
                .spawn(move || {
                    while rodando.load(Ordering::Relaxed) {
                        // Uma coisa por tique, e os LEDs têm prioridade.
                        //
                        // O aparelho aceita cerca de 31 escritas por segundo (medido no
                        // spike). Mandar o frame de LED e os dois pacotes de tela no mesmo
                        // tique passa de 90 por segundo, e aí ele descarta pacote e as
                        // cores saem erradas. Espaçar resolve, e não custa nada: a tela
                        // muda raramente, então a espera de um tique não aparece.
                        let pacote = {
                            let mut f = leds.lock().expect("frame de LEDs");
                            if f.esta_sujo() {
                                let bytes = f.bytes().to_vec();
                                f.marcar_limpo();
                                Some(bytes)
                            } else {
                                None
                            }
                        };

                        if let Some(bytes) = pacote {
                            let _ = dev_escrita.write(&bytes);
                        } else {
                            let pacotes_tela = {
                                let mut t = tela.lock().expect("tela");
                                if t.esta_suja() {
                                    let p = t.pacotes();
                                    t.marcar_limpa();
                                    Some(p)
                                } else {
                                    None
                                }
                            };
                            if let Some(pacotes) = pacotes_tela {
                                for p in pacotes {
                                    let _ = dev_escrita.write(&p);
                                    // As duas metades da tela também precisam de fôlego
                                    // entre elas, senão a segunda se perde.
                                    thread::sleep(Duration::from_millis(12));
                                }
                            }
                        }

                        thread::sleep(intervalo);
                    }
                })
                .expect("subir a thread de escrita")
        };

        Ok((
            Self {
                leds,
                tela,
                rodando,
                escritor: Some(escritor),
                leitor: Some(leitor),
            },
            recebe,
        ))
    }

    /// Atalho para mexer no frame de LEDs sem lidar com o cadeado na mão.
    pub fn pintar<F: FnOnce(&mut FrameLeds)>(&self, f: F) {
        let mut frame = self.leds.lock().expect("frame de LEDs");
        f(&mut frame);
    }

    /// Atalho para desenhar na tela do aparelho.
    pub fn desenhar<F: FnOnce(&mut crate::render::Tela)>(&self, f: F) {
        let mut tela = self.tela.lock().expect("tela");
        f(&mut tela);
    }
}

impl Drop for Aparelho {
    fn drop(&mut self) {
        self.rodando.store(false, Ordering::Relaxed);
        if let Some(h) = self.escritor.take() {
            let _ = h.join();
        }
        // A leitura pode estar bloqueada até o timeout; não vale a pena esperar por ela.
        drop(self.leitor.take());
    }
}

/// Traduz os pacotes crus do aparelho em `Evento`.
///
/// Guarda o estado anterior porque o aparelho manda o estado inteiro dos botões a cada
/// mudança, não o que mudou. A diferença é calculada aqui.
struct Decodificador {
    botoes_antes: u64,
    knob_pos_antes: Option<u8>,
    strip_antes: Option<u8>,
}

impl Decodificador {
    fn novo() -> Self {
        Self {
            botoes_antes: 0,
            knob_pos_antes: None,
            strip_antes: None,
        }
    }

    fn decodificar(&mut self, buf: &[u8]) -> Vec<Evento> {
        match buf.first() {
            Some(&REPORT_PADS) => self.decodificar_pad(buf),
            Some(&REPORT_BOTOES) => self.decodificar_botoes(buf),
            _ => Vec::new(),
        }
    }

    fn decodificar_pad(&mut self, buf: &[u8]) -> Vec<Evento> {
        if buf.len() < 4 {
            return Vec::new();
        }
        let Some(pad) = pad_bruto_para_impresso(buf[1]) else {
            return Vec::new();
        };
        let pressao = ((buf[2] & 0x0F) as u16) << 8 | buf[3] as u16;
        // O nibble alto do byte 2 diz o que aconteceu.
        let evento = match buf[2] & 0xF0 {
            0x40 => Evento::PadTocado { pad, pressao },
            0x10 => Evento::PadApertado { pad, pressao },
            0x20 | 0x30 => Evento::PadSolto { pad },
            _ => return Vec::new(),
        };
        vec![evento]
    }

    fn decodificar_botoes(&mut self, buf: &[u8]) -> Vec<Evento> {
        if buf.len() < 13 {
            return Vec::new();
        }
        let mut eventos = Vec::new();

        // Bytes 1 a 5 são o bitmask dos 39 botões mais o "apertar knob" no bit 39.
        let mut agora: u64 = 0;
        for (i, b) in buf[1..6].iter().enumerate() {
            agora |= (*b as u64) << (8 * i);
        }
        let mudou = agora ^ self.botoes_antes;
        if mudou != 0 {
            for bit in 0..=BIT_KNOB_APERTADO {
                if mudou & (1 << bit) == 0 {
                    continue;
                }
                let apertado = agora & (1 << bit) != 0;
                if bit == BIT_KNOB_APERTADO {
                    eventos.push(Evento::KnobApertado { apertado });
                } else if let Some(nome) = BOTOES.get(bit) {
                    eventos.push(Evento::Botao {
                        nome,
                        apertado,
                    });
                }
            }
        }
        self.botoes_antes = agora;

        // Byte 7: posição do knob, 0 a 15 e dá a volta.
        let pos = buf[7] & 0x0F;
        match self.knob_pos_antes {
            None => self.knob_pos_antes = Some(pos),
            Some(antes) if antes != pos => {
                // Menor caminho no círculo de 16 posições, para o giro de 15 para 0
                // contar como +1 e não como -15.
                let bruto = pos as i16 - antes as i16;
                let delta = if bruto > 8 {
                    bruto - 16
                } else if bruto < -8 {
                    bruto + 16
                } else {
                    bruto
                };
                if delta != 0 {
                    eventos.push(Evento::KnobGirado {
                        delta: delta.signum() as i8,
                    });
                }
                self.knob_pos_antes = Some(pos);
            }
            _ => {}
        }
        // Byte 10: posição do dedo na touch strip. Zero significa dedo fora.
        let strip = if buf[10] == 0 { None } else { Some(buf[10]) };
        if strip != self.strip_antes {
            eventos.push(Evento::Strip { posicao: strip });
            self.strip_antes = strip;
        }

        eventos
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pacote_botoes(bits: u64, knob_pos: u8, strip: u8) -> Vec<u8> {
        let mut buf = vec![0u8; 14];
        buf[0] = REPORT_BOTOES;
        for i in 0..5 {
            buf[1 + i] = ((bits >> (8 * i)) & 0xFF) as u8;
        }
        buf[7] = knob_pos;
        buf[10] = strip;
        buf
    }

    #[test]
    fn detecta_botao_apertado_e_solto() {
        let mut d = Decodificador::novo();
        // bit 22 é o "play" na tabela de botões
        let play = BOTOES.iter().position(|&b| b == "play").unwrap();
        let ev = d.decodificar(&pacote_botoes(1 << play, 0, 0));
        assert_eq!(
            ev,
            vec![Evento::Botao {
                nome: "play",
                apertado: true
            }]
        );
        let ev = d.decodificar(&pacote_botoes(0, 0, 0));
        assert_eq!(
            ev,
            vec![Evento::Botao {
                nome: "play",
                apertado: false
            }]
        );
    }

    #[test]
    fn knob_da_a_volta_sem_pular() {
        let mut d = Decodificador::novo();
        d.decodificar(&pacote_botoes(0, 15, 0)); // firma a posição inicial
        let ev = d.decodificar(&pacote_botoes(0, 0, 0));
        assert_eq!(ev, vec![Evento::KnobGirado { delta: 1 }]);
        let ev = d.decodificar(&pacote_botoes(0, 15, 0));
        assert_eq!(ev, vec![Evento::KnobGirado { delta: -1 }]);
    }

    #[test]
    fn strip_avisa_quando_o_dedo_sai() {
        let mut d = Decodificador::novo();
        let ev = d.decodificar(&pacote_botoes(0, 0, 100));
        assert_eq!(ev, vec![Evento::Strip { posicao: Some(100) }]);
        let ev = d.decodificar(&pacote_botoes(0, 0, 0));
        assert_eq!(ev, vec![Evento::Strip { posicao: None }]);
    }

    #[test]
    fn decodifica_pad_com_pressao() {
        let mut d = Decodificador::novo();
        // índice bruto 0 é o pad impresso 13; 0x10 = apertado; pressão 0x123
        let ev = d.decodificar(&[REPORT_PADS, 0, 0x11, 0x23]);
        assert_eq!(
            ev,
            vec![Evento::PadApertado {
                pad: 13,
                pressao: 0x123
            }]
        );
    }

    #[test]
    fn pacote_curto_nao_derruba_o_decodificador() {
        let mut d = Decodificador::novo();
        assert!(d.decodificar(&[REPORT_PADS]).is_empty());
        assert!(d.decodificar(&[REPORT_BOTOES, 0, 0]).is_empty());
        assert!(d.decodificar(&[]).is_empty());
    }
}
