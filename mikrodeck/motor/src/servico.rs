//! O motor rodando de forma residente.
//!
//! Cuida do ciclo de vida inteiro: abre o aparelho, processa eventos, executa ações,
//! e reconecta sozinho quando o cabo é religado. A UI conversa com este módulo,
//! nunca com o `hid` direto.

use crate::acoes::{Acao, Executor};
use crate::audio::Volume;
use crate::config::{Config, FuncaoKnob, FuncaoStrip};
use crate::estado::{Estado, Reacao};
use crate::janelas;
use crate::luz::{Animador, Quadro};
use crate::vigias::Abertos;
use crate::hid::{Aparelho, Evento};
use crate::render::composicao::Compositor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Situação do aparelho, para a UI mostrar na barra de status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Situacao {
    Procurando,
    Conectado,
}

/// Avisos que o motor manda para a UI.
#[derive(Debug, Clone)]
pub enum Aviso {
    /// O aparelho conectou ou desconectou.
    Situacao(Situacao),
    /// Um pad foi apertado ou solto, para a UI piscar o desenho ao vivo.
    Pad { pad: u8, apertado: bool },
    /// Um botão físico foi apertado ou solto.
    Botao { nome: String, apertado: bool },
    /// A página mudou, seja por botão físico ou por ação de pad.
    Pagina { numero: usize, nome: String },
    /// O MikroDeck foi pausado ou retomado pelo botão do aparelho.
    Pausado(bool),
    /// Posição crua do dedo na touch strip, de 0 a 255. Serve para calibrar.
    Strip { posicao: u8 },
}

/// O motor residente. Enquanto este objeto existir, ele roda.
pub struct Servico {
    estado: Arc<Mutex<Estado>>,
    situacao: Arc<Mutex<Situacao>>,
    rodando: Arc<AtomicBool>,
    /// Pedido de teste de LEDs vindo da interface. O laço atende no próximo tick.
    teste_pedido: Arc<AtomicBool>,
    descanso_pedido: Arc<AtomicBool>,
    supervisor: Option<thread::JoinHandle<()>>,
}

impl Servico {
    /// Sobe o motor. `avisar` é chamado de outra thread a cada mudança relevante.
    pub fn iniciar<F>(config: Config, avisar: F) -> Self
    where
        F: Fn(Aviso) + Send + Sync + 'static,
    {
        let estado = Arc::new(Mutex::new(Estado::novo(config)));
        let situacao = Arc::new(Mutex::new(Situacao::Procurando));
        let rodando = Arc::new(AtomicBool::new(true));
        let teste_pedido = Arc::new(AtomicBool::new(false));
        let descanso_pedido = Arc::new(AtomicBool::new(false));

        let supervisor = {
            let estado = estado.clone();
            let situacao = situacao.clone();
            let rodando = rodando.clone();
            let teste_pedido = teste_pedido.clone();
            let descanso_pedido = descanso_pedido.clone();
            let avisar = Arc::new(avisar);
            thread::Builder::new()
                .name("mikrodeck-supervisor".into())
                .spawn(move || {
                    let executor = Executor::novo();
                    // Uma varredura de processos só, compartilhada por toda a vida
                    // do serviço: ela diz quais pads acendem com a cor de "aberto".
                    let abertos = Abertos::vigiar();
                    // O volume abre uma vez só: abrir a cada toque na strip
                    // custaria caro e travaria o laço.
                    let volume = Volume::abrir();
                    while rodando.load(Ordering::Relaxed) {
                        match Aparelho::abrir(30) {
                            Ok((aparelho, eventos)) => {
                                *situacao.lock().unwrap_or_else(|e| e.into_inner()) = Situacao::Conectado;
                                avisar(Aviso::Situacao(Situacao::Conectado));
                                laco_de_eventos(
                                    &aparelho,
                                    eventos,
                                    &estado,
                                    &executor,
                                    &rodando,
                                    &teste_pedido,
                                    &descanso_pedido,
                                    volume.as_ref(),
                                    &abertos,
                                    avisar.as_ref(),
                                );
                                *situacao.lock().unwrap_or_else(|e| e.into_inner()) = Situacao::Procurando;
                                avisar(Aviso::Situacao(Situacao::Procurando));
                            }
                            Err(_) => {
                                // Aparelho fora do ar. Tenta de novo daqui a pouco,
                                // sem gastar CPU e sem poluir o log a cada tentativa.
                                thread::sleep(Duration::from_millis(800));
                            }
                        }
                    }
                })
                .expect("subir o supervisor do motor")
        };

        Self {
            estado,
            situacao,
            rodando,
            teste_pedido,
            descanso_pedido,
            supervisor: Some(supervisor),
        }
    }

    pub fn situacao(&self) -> Situacao {
        *self.situacao.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Config atual, para a UI mostrar e editar.
    pub fn config(&self) -> Config {
        travar(&self.estado).config().clone()
    }

    /// Troca a config em uso. Os LEDs se ajustam no próximo tick.
    pub fn aplicar_config(&self, config: Config) {
        let mut estado = travar(&self.estado);
        estado.trocar_config(config);
    }

    /// Página atual, para a UI destacar a aba certa.
    pub fn pagina_atual(&self) -> (usize, String) {
        let e = travar(&self.estado);
        (e.numero_pagina(), e.nome_pagina().to_string())
    }

    /// Manda o motor ir para uma página, a partir da UI.
    pub fn ir_para_pagina(&self, numero: usize) {
        let mut e = travar(&self.estado);
        e.ir_para_pagina(numero);
    }

    /// Força o descanso agora, para a pessoa ver a luz sem esperar a espera toda.
    pub fn previsualizar_descanso(&self) {
        self.descanso_pedido.store(true, Ordering::Relaxed);
    }

    /// Pede um teste visual: acende tudo por um instante e volta ao normal.
    pub fn testar_leds(&self) {
        self.teste_pedido.store(true, Ordering::Relaxed);
    }
}

impl Drop for Servico {
    fn drop(&mut self) {
        self.rodando.store(false, Ordering::Relaxed);
        if let Some(h) = self.supervisor.take() {
            let _ = h.join();
        }
    }
}

/// Roda até o aparelho desconectar ou o motor ser encerrado.
fn laco_de_eventos<F>(
    aparelho: &Aparelho,
    eventos: std::sync::mpsc::Receiver<Evento>,
    estado: &Arc<Mutex<Estado>>,
    executor: &Executor,
    rodando: &Arc<AtomicBool>,
    teste_pedido: &Arc<AtomicBool>,
    descanso_pedido: &Arc<AtomicBool>,
    volume: Option<&Volume>,
    abertos: &Abertos,
    avisar: &F,
) where
    F: Fn(Aviso) + Send + Sync + ?Sized,
{
    // A luz dos pads no descanso e o eco ao soltar.
    let mut animador = {
        let e = travar(estado);
        Animador::novo(e.config().descanso.luz, e.config().ao_apertar)
    };
    // Quantos tiques rápidos já passaram, para ler o volume a cada quatro.
    let mut tiques_rapidos: u32 = 0;

    // Pinta o estado inicial assim que conecta.
    repintar(aparelho, estado, abertos, None, false);

    // Pad que cuida da janela: qual, o que ele abre, e desde quando está apertado.
    let mut segurando_janela: Option<(u8, AlvoDoPad, Instant)> = None;
    // Quando cada pad foi solto pela última vez, para reconhecer o toque duplo.
    let mut ultimo_toque: std::collections::HashMap<u8, Instant> = std::collections::HashMap::new();

    // A tela acompanha a página e o controle que estiver sendo segurado.
    let mut compositor = {
        let e = travar(estado);
        let mut c =
            Compositor::novo(e.numero_pagina(), e.total_paginas(), e.nome_pagina().to_string());
        c.definir_descanso(descanso_da_config(&e));
        c
    };
    atualizar_tela(aparelho, &mut compositor, esta_pausado(estado));

    loop {
        if !rodando.load(Ordering::Relaxed) {
            return;
        }
        if teste_pedido.swap(false, Ordering::Relaxed) {
            acender_tudo(aparelho);
        }
        if descanso_pedido.swap(false, Ordering::Relaxed) {
            compositor.forcar_descanso();
        }
        // Timeout curto para o encerramento não ficar preso esperando evento.
        // Dormindo ou em eco, o laço acelera para 25 ms; no resto, 100 ms bastam.
        let rapido = animador.precisa_de_tique();
        let espera = Duration::from_millis(if rapido { 25 } else { 100 });
        let evento = match eventos.recv_timeout(espera) {
            Ok(e) => e,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // O volume é uma chamada COM: no ritmo rápido, uma a cada quatro.
                tiques_rapidos = tiques_rapidos.wrapping_add(1);
                if !rapido || tiques_rapidos % 4 == 0 {
                    atualizar_nivel_strip(estado, volume);
                }
                // O descanso e a luz podem ter mudado pela interface ou pelo MCP.
                let pausado = esta_pausado(estado);
                let (brilho, repouso) = {
                    let e = travar(estado);
                    compositor.definir_descanso(descanso_da_config(&e));
                    animador.configurar(e.config().descanso.luz, e.config().ao_apertar);
                    (e.config().brilho, e.quadro_de_repouso(abertos))
                };
                let dormindo = compositor.dormindo() && !pausado;
                animador.tique(Instant::now(), dormindo, brilho, &repouso, None);
                // Repinta mesmo sem evento: a config pode ter mudado pela interface
                // (brilho, cor, página) e ninguém encostou no aparelho. O frame só
                // vai para o aparelho se algum byte mudou de verdade.
                repintar(aparelho, estado, abertos, animador.quadro(), animador.dormindo());
                // A tela também precisa do tick: o aviso passageiro some sozinho e
                // o texto do descanso anda a cada quadro.
                atualizar_tela(aparelho, &mut compositor, pausado);
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        };

        if evento == Evento::Desconectado {
            return;
        }

        // A touch strip controla o que estiver configurado.
        if let Evento::Strip { posicao: Some(p) } = &evento {
            // A UI precisa da posição crua para poder calibrar.
            avisar(Aviso::Strip { posicao: *p });
            let (fracao, funcao) = {
                let e = travar(estado);
                (e.config().calibracao_strip.fracao(*p), e.config().strip)
            };
            match funcao {
                FuncaoStrip::Volume => {
                    if let Some(v) = volume {
                        v.definir(fracao);
                        compositor.avisar(crate::render::composicao::Cena::Aviso {
                            titulo: format!("Volume {}%", (fracao * 100.0).round() as u32),
                            barra: Some(fracao),
                        });
                    }
                }
                FuncaoStrip::BrilhoPads => {
                    let mut e = travar(estado);
                    let novo = (fracao * 3.0).round() as u8;
                    e.definir_brilho(novo);
                    compositor.avisar(crate::render::composicao::Cena::Aviso {
                        titulo: format!("Brilho {novo}"),
                        barra: Some(novo as f32 / 3.0),
                    });
                }
                FuncaoStrip::Paginas => {
                    let mut e = travar(estado);
                    let total = e.total_paginas();
                    let alvo = ((fracao * total as f32).floor() as usize + 1).min(total);
                    if alvo != e.numero_pagina() {
                        if let Reacao::PaginaMudou { numero, nome } = e.ir_para_pagina(alvo) {
                            compositor.pagina(numero, total, nome.clone());
                            drop(e);
                            avisar(Aviso::Pagina { numero, nome });
                        }
                    }
                }
                FuncaoStrip::Nenhuma => {}
            }
        }

        // Girar o knob controla o que estiver configurado. Ele dá passos, não
        // posição, então tudo aqui é relativo ao valor de agora.
        if let Evento::KnobGirado { delta } = &evento {
            girar_knob(*delta, estado, volume, &mut compositor, avisar);
        }

        // A tela mostra o nome do que está sendo segurado.
        {
            // Qualquer evento do aparelho adia o descanso, e acorda a luz com
            // corte seco: quem apertou quer o pad agora.
            compositor.tocou();
            animador.acordar();
            let e = travar(estado);
            match &evento {
                // Encostar de leve já mostra o nome na tela, sem executar nada.
                // O aparelho manda esse evento separado do aperto de verdade.
                Evento::PadTocado { pad, .. } | Evento::PadApertado { pad, .. } => {
                    compositor.segurando(e.nome_do_pad(*pad).map(str::to_string))
                }
                Evento::PadSolto { pad } => {
                    compositor.segurando(None);
                    // Só ecoa o que foi apertado de verdade. O aparelho manda
                    // `PadSolto` também quando o dedo só encostou, e piscar num
                    // toque leve contradiz "toque leve não executa".
                    if e.esta_apertado(*pad) {
                        animador.eco(*pad, Instant::now());
                    }
                }
                Evento::Botao { nome, apertado } => {
                    if *apertado {
                        compositor.segurando(e.nome_do_botao(nome).map(str::to_string));
                    } else {
                        compositor.segurando(None);
                    }
                }
                _ => {}
            }
        }

        match &evento {
            Evento::PadApertado { pad, .. } => avisar(Aviso::Pad {
                pad: *pad,
                apertado: true,
            }),
            Evento::PadSolto { pad } => avisar(Aviso::Pad {
                pad: *pad,
                apertado: false,
            }),
            Evento::Botao { nome, apertado } => avisar(Aviso::Botao {
                nome: nome.to_string(),
                apertado: *apertado,
            }),
            _ => {}
        }

        // Pad que cuida da janela age quando é solto, não quando é apertado:
        // é a única forma de distinguir um toque de um "segurar para fechar".
        let mut este_pad_cuida_da_janela = false;
        match &evento {
            Evento::PadApertado { pad, .. } => {
                let cuidar = {
                    let e = travar(estado);
                    alvo_com_janela(&e, *pad)
                };
                if let Some(alvo) = cuidar {
                    segurando_janela = Some((*pad, alvo, Instant::now()));
                    este_pad_cuida_da_janela = true;
                }
            }
            Evento::PadSolto { pad } => {
                // Só mexe se for o mesmo pad: soltar outro pad não pode cancelar
                // o "segurar para fechar" que está em andamento.
                let e_este = matches!(&segurando_janela, Some((p, _, _)) if p == pad);
                if e_este {
                    if let Some((_, alvo, desde)) = segurando_janela.take() {
                        let anterior = ultimo_toque.get(pad).map(|t| t.elapsed());
                        agir_no_alvo(&alvo, desde.elapsed(), anterior, executor, estado);
                        ultimo_toque.insert(*pad, Instant::now());
                        // O estado ainda precisa saber que soltou, para o LED voltar.
                        let _ = travar(estado).processar(&evento);
                        repintar(aparelho, estado, abertos, animador.quadro(), animador.dormindo());
                        atualizar_tela(aparelho, &mut compositor, esta_pausado(estado));
                        continue;
                    }
                }
            }
            _ => {}
        }

        let reacao = {
            let mut e = travar(estado);
            e.processar(&evento)
        };

        // Quando o pad cuida da janela, apertar não dispara nada: quem decide é
        // o soltar, logo acima.
        let reacao = match reacao {
            Reacao::Executar(_) if este_pad_cuida_da_janela => Reacao::Nada,
            outra => outra,
        };

        match reacao {
            Reacao::Executar(acao) => {
                // A ligação com a casa pode ter mudado pela interface desde a última
                // ação, e a thread de ações não enxerga o estado.
                executor.definir_home_assistant(
                    travar(estado).config().home_assistant.clone(),
                );
                executor.disparar(acao)
            }
            Reacao::PaginaMudou { numero, nome } => {
                let total = travar(estado).total_paginas();
                compositor.pagina(numero, total, nome.clone());
                avisar(Aviso::Pagina { numero, nome });
            }
            Reacao::Pausado(pausado) => {
                if pausado {
                    // Sai de cena: apaga a tela e deixa o aparelho para o
                    // Maschine 2 ou para o que a pessoa quiser usar.
                    apagar_tela(aparelho);
                } else {
                    compositor.avisar(crate::render::composicao::Cena::Aviso {
                        titulo: "Ativo".into(),
                        barra: None,
                    });
                }
                avisar(Aviso::Pausado(pausado));
            }
            Reacao::Nada => {}
        }

        atualizar_nivel_strip(estado, volume);
        {
            let (brilho, repouso) = {
                let e = travar(estado);
                (e.config().brilho, e.quadro_de_repouso(abertos))
            };
            animador.tique(Instant::now(), false, brilho, &repouso, None);
        }
        repintar(aparelho, estado, abertos, animador.quadro(), animador.dormindo());
        atualizar_tela(aparelho, &mut compositor, esta_pausado(estado));
    }
}

/// Acende a touch strip na altura do que ela controla. Roda a cada tique porque o
/// volume também muda por fora, pelas teclas de mídia do teclado.
fn atualizar_nivel_strip(estado: &Arc<Mutex<Estado>>, volume: Option<&Volume>) {
    let funcao = travar(estado).config().strip;
    let nivel = match funcao {
        FuncaoStrip::Nenhuma => None,
        FuncaoStrip::Volume => volume.and_then(|v| v.ler()),
        FuncaoStrip::BrilhoPads => {
            Some(travar(estado).config().brilho.min(3) as f32 / 3.0)
        }
        FuncaoStrip::Paginas => {
            let e = travar(estado);
            let total = e.total_paginas().max(1);
            Some(e.numero_pagina() as f32 / total as f32)
        }
    };
    travar(estado).definir_nivel_strip(nivel);
}

/// Acende todos os pads, a strip e os botões por um instante. Serve para a pessoa
/// confirmar que o aparelho responde, sem precisar configurar nada.
fn acender_tudo(aparelho: &Aparelho) {
    use crate::hid::{BrilhoBotao, Cor};
    for cor in [Cor::Vermelho, Cor::Verde, Cor::Azul, Cor::Branco] {
        aparelho.pintar(|f| {
            for pad in 1..=16u8 {
                f.pad(pad, cor, 3);
            }
            for i in 0..25 {
                f.strip(i, cor, 3);
            }
            for nome in crate::hid::protocolo::BOTOES {
                f.botao(nome, BrilhoBotao::Forte);
            }
        });
        thread::sleep(Duration::from_millis(280));
    }
}

/// O que um pad que pediu para cuidar da janela abre.
#[derive(Debug, Clone)]
enum AlvoDoPad {
    Programa(String),
    Link(String),
}

/// Alvo de um pad que pediu para cuidar da janela. `None` se o pad não existe,
/// não abre nada com janela, ou não pediu isso.
fn alvo_com_janela(estado: &Estado, pad: u8) -> Option<AlvoDoPad> {
    let controle = estado.controle(pad)?;
    if !controle.gerenciar_janela {
        return None;
    }
    match &controle.acao {
        Acao::AbrirPrograma { caminho, .. } => Some(AlvoDoPad::Programa(caminho.clone())),
        Acao::AbrirUrl { url } => Some(AlvoDoPad::Link(url.clone())),
        _ => None,
    }
}

/// Faz o que o gesto pediu, no programa ou no link do pad.
///
/// A ordem importa. Tentar mexer na janela primeiro, e só abrir se não houver
/// janela nenhuma, é mais confiável do que perguntar antes se está aberto: um
/// lançador abre janela com outro nome (`git-bash.exe` abre `mintty.exe`) e a
/// varredura de processos não enxerga isso.
fn agir_no_alvo(
    alvo: &AlvoDoPad,
    segurado: Duration,
    desde_o_toque_anterior: Option<Duration>,
    executor: &Executor,
    estado: &Arc<Mutex<Estado>>,
) {
    match alvo {
        AlvoDoPad::Programa(caminho) => {
            // `aberto: true` porque quem decide se há janela é o `agir` logo
            // abaixo, que sabe de verdade.
            match janelas::ao_soltar(segurado, true, desde_o_toque_anterior) {
                janelas::Depois::Fechar => {
                    janelas::agir(caminho, janelas::Alvo::Fechar);
                }
                janelas::Depois::Maximizar => {
                    janelas::agir(caminho, janelas::Alvo::Maximizar);
                }
                _ => {
                    if !janelas::agir(caminho, janelas::Alvo::AlternarFrente) {
                        abrir(executor, estado, Acao::AbrirPrograma {
                            caminho: caminho.clone(),
                            argumentos: vec![],
                        });
                    }
                }
            }
        }
        AlvoDoPad::Link(url) => {
            let rotulo = janelas::rotulo_do_site(url).unwrap_or_default();
            let abrir_link = || {
                abrir(executor, estado, Acao::AbrirUrl { url: url.clone() });
            };
            match janelas::ao_soltar_link(segurado, desde_o_toque_anterior) {
                // Segurar abre mais uma, mesmo já tendo: é o pedido explícito
                // de uma janela nova.
                janelas::DepoisNoLink::AbrirOutra | janelas::DepoisNoLink::Abrir => abrir_link(),
                janelas::DepoisNoLink::Maximizar => {
                    janelas::agir_por_titulo(&rotulo, janelas::Alvo::Maximizar);
                }
                janelas::DepoisNoLink::IrParaJanela => {
                    if !janelas::agir_por_titulo(&rotulo, janelas::Alvo::TrazerParaFrente) {
                        abrir_link();
                    }
                }
            }
        }
    }
}

/// Enfileira uma ação de abrir, com a ligação da casa atualizada.
fn abrir(executor: &Executor, estado: &Arc<Mutex<Estado>>, acao: Acao) {
    executor.definir_home_assistant(travar(estado).config().home_assistant.clone());
    executor.disparar(acao);
}

/// Pega o cadeado do estado sem morrer se ele estiver envenenado.
///
/// Um `unwrap` aqui transforma o pânico de uma thread no pânico de todas: o
/// cadeado fica envenenado e o motor para em silêncio, com o app aberto e o
/// aparelho morto. Seguir com o estado que sobrou é sempre melhor.
fn travar(estado: &Arc<Mutex<Estado>>) -> std::sync::MutexGuard<'_, Estado> {
    estado.lock().unwrap_or_else(|e| e.into_inner())
}

/// Se o MikroDeck está desligado agora.
fn esta_pausado(estado: &Arc<Mutex<Estado>>) -> bool {
    travar(estado).esta_pausado()
}

/// Aplica um passo do knob no que ele estiver controlando.
fn girar_knob<F>(
    delta: i8,
    estado: &Arc<Mutex<Estado>>,
    volume: Option<&Volume>,
    compositor: &mut Compositor,
    avisar: &F,
) where
    F: Fn(Aviso) + Send + Sync + ?Sized,
{
    use crate::render::composicao::Cena;
    let funcao = travar(estado).config().knob;
    match funcao {
        FuncaoKnob::Nenhuma => {}
        FuncaoKnob::Volume => {
            let Some(v) = volume else { return };
            let Some(atual) = v.ler() else { return };
            // 2% por passo: fino o bastante para acertar, grosso o bastante para
            // atravessar a escala sem girar a noite toda.
            let novo = (atual + delta as f32 * 0.02).clamp(0.0, 1.0);
            v.definir(novo);
            compositor.avisar(Cena::Aviso {
                titulo: format!("Volume {}%", (novo * 100.0).round() as u32),
                barra: Some(novo),
            });
        }
        FuncaoKnob::BrilhoPads => {
            let mut e = travar(estado);
            let atual = e.config().brilho.min(3) as i16;
            let novo = (atual + delta as i16).clamp(0, 3) as u8;
            e.definir_brilho(novo);
            drop(e);
            compositor.avisar(Cena::Aviso {
                titulo: format!("Brilho {novo}"),
                barra: Some(novo as f32 / 3.0),
            });
        }
        FuncaoKnob::Rolagem => {
            // Girar para cima rola para cima, como a roda do mouse. Dois passos
            // por clique, senão rolar uma página inteira vira exercício de pulso.
            crate::acoes::teclado::rolar(if delta > 0 { 2 } else { -2 });
        }
        FuncaoKnob::Paginas => {
            let mut e = travar(estado);
            let efeito = if delta > 0 {
                crate::acoes::EfeitoNoEstado::ProximaPagina
            } else {
                crate::acoes::EfeitoNoEstado::PaginaAnterior
            };
            if let Reacao::PaginaMudou { numero, nome } = e.aplicar_efeito_publico(efeito) {
                let total = e.total_paginas();
                drop(e);
                compositor.pagina(numero, total, nome.clone());
                avisar(Aviso::Pagina { numero, nome });
            }
        }
    }
}

/// Texto e espera do descanso, do jeito que o compositor quer. `None` desliga.
fn descanso_da_config(estado: &Estado) -> Option<(String, Duration)> {
    let d = &estado.config().descanso;
    if !d.ativo {
        return None;
    }
    Some((d.texto.clone(), Duration::from_secs(d.segundos.max(5))))
}

/// Atualiza a tela, a não ser que o MikroDeck esteja desligado.
///
/// Desligado, ele tem que sumir do aparelho para o Maschine 2 poder usar a tela
/// e os LEDs sem briga. A tela é apagada uma vez na hora de desligar e depois
/// ninguém mais escreve nela.
fn atualizar_tela(aparelho: &Aparelho, compositor: &mut Compositor, pausado: bool) {
    if pausado {
        return;
    }
    aparelho.desenhar(|tela| {
        compositor.desenhar(tela);
    });
}

/// Apaga a tela do aparelho. Usado ao desligar o MikroDeck.
fn apagar_tela(aparelho: &Aparelho) {
    aparelho.desenhar(|tela| tela.limpar());
}

fn repintar(
    aparelho: &Aparelho,
    estado: &Arc<Mutex<Estado>>,
    abertos: &Abertos,
    luz: Option<&Quadro>,
    dormindo: bool,
) {
    let e = travar(estado);
    aparelho.pintar(|f| e.pintar_com(f, abertos, luz, dormindo));
}
