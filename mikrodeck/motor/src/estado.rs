//! O estado do motor: qual página está aberta, quais pads estão apertados,
//! e como isso vira cor nos LEDs.
//!
//! Este módulo não fala com o HID nem executa ação. Ele recebe evento, decide,
//! e devolve o que precisa acontecer. Isso deixa ele testável sem aparelho.

use crate::acoes::{Acao, EfeitoNoEstado};
use crate::config::{Config, Controle};
use crate::luz::Quadro;
use crate::vigias::Abertos;
use crate::hid::protocolo::NUM_STRIP;
use crate::hid::{BrilhoBotao, Cor, Evento, FrameLeds};
use std::collections::HashSet;

/// O que o motor deve fazer depois de processar um evento.
#[derive(Debug, Clone, PartialEq)]
pub enum Reacao {
    /// Nada a fazer.
    Nada,
    /// Executar esta ação.
    Executar(Acao),
    /// A página mudou. Quem cuida da tela deve se atualizar.
    PaginaMudou { numero: usize, nome: String },
    /// O MikroDeck foi pausado ou retomado.
    Pausado(bool),
}

pub struct Estado {
    config: Config,
    /// Índice da página atual, contando de 0.
    pagina: usize,
    /// Pads apertados agora, pelo número impresso.
    apertados: HashSet<u8>,
    /// Pausado, o aparelho apaga e nenhum controle executa ação. Serve para o
    /// Davi voltar a usar o Mikro como Maschine sem fechar o MikroDeck.
    pausado: bool,
    /// Quanto da touch strip acender, de 0 a 1. `None` deixa apagada.
    nivel_strip: Option<f32>,
}

impl Estado {
    pub fn novo(config: Config) -> Self {
        Self {
            config,
            pagina: 0,
            apertados: HashSet::new(),
            pausado: false,
            nivel_strip: None,
        }
    }

    /// Define o quanto da touch strip fica aceso, de 0 a 1.
    pub fn definir_nivel_strip(&mut self, nivel: Option<f32>) {
        self.nivel_strip = nivel.map(|n| n.clamp(0.0, 1.0));
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Troca a config mantendo onde a pessoa estava. Salvar um pad na interface
    /// não pode jogar o aparelho de volta para a primeira página nem cancelar a
    /// pausa. Se a página atual sumiu, cai na última que sobrou.
    pub fn trocar_config(&mut self, config: Config) {
        self.config = config;
        let ultima = self.config.paginas.len().saturating_sub(1);
        self.pagina = self.pagina.min(ultima);
        self.apertados.clear();
    }

    /// Número da página atual para mostrar ao usuário, contando de 1.
    pub fn numero_pagina(&self) -> usize {
        self.pagina + 1
    }

    pub fn nome_pagina(&self) -> &str {
        self.config
            .paginas
            .get(self.pagina)
            .map(|p| p.nome.as_str())
            .unwrap_or("(vazia)")
    }

    pub fn total_paginas(&self) -> usize {
        self.config.total_paginas()
    }

    pub fn controle(&self, pad: u8) -> Option<&Controle> {
        self.config.paginas.get(self.pagina)?.pads.get(&pad)
    }

    /// Nome configurado de um pad, para mostrar na tela do aparelho.
    pub fn nome_do_pad(&self, pad: u8) -> Option<&str> {
        self.controle(pad).map(|c| c.nome.as_str()).filter(|n| !n.is_empty())
    }

    /// Nome configurado de um botão, para mostrar na tela do aparelho.
    pub fn nome_do_botao(&self, nome: &str) -> Option<&str> {
        self.controle_botao(nome).map(|c| c.nome.as_str()).filter(|n| !n.is_empty())
    }

    fn controle_botao(&self, nome: &str) -> Option<&Controle> {
        self.config.paginas.get(self.pagina)?.botoes.get(nome)
    }

    /// Ação que a página dá a um botão, se houver alguma de verdade. Um controle
    /// com ação `Nenhuma` conta como não configurado.
    fn acao_configurada(&self, nome: &str) -> Option<Acao> {
        match self.controle_botao(nome) {
            Some(c) if c.acao != Acao::Nenhuma => Some(c.acao.clone()),
            _ => None,
        }
    }

    /// Processa um evento vindo do aparelho e diz o que fazer.
    pub fn processar(&mut self, evento: &Evento) -> Reacao {
        // Pausado, só o botão de retomar responde. Todo o resto é ignorado,
        // inclusive troca de página, para não surpreender ao voltar.
        if self.pausado {
            if let Evento::Botao { nome, apertado: true } = evento {
                let acao = self
                    .acao_configurada(nome)
                    .unwrap_or_else(|| padrao_do_botao(nome));
                if acao == Acao::PausarRetomar {
                    return self.aplicar_efeito(EfeitoNoEstado::PausarRetomar);
                }
            }
            return Reacao::Nada;
        }
        match evento {
            Evento::PadApertado { pad, .. } => {
                self.apertados.insert(*pad);
                let Some(controle) = self.controle(*pad) else {
                    return Reacao::Nada;
                };
                let acao = controle.acao.clone();
                match acao.efeito_no_estado() {
                    Some(efeito) => self.aplicar_efeito(efeito),
                    None => Reacao::Executar(acao),
                }
            }
            Evento::PadSolto { pad } => {
                self.apertados.remove(pad);
                Reacao::Nada
            }
            Evento::Botao { nome, apertado: true } => {
                // Os dois botões escolhidos para trocar de página são reservados:
                // eles ganham de qualquer configuração de página. Sem isso a
                // pessoa se tranca fora da navegação sem perceber, e a interface
                // por sua vez impede de programar esses dois.
                if *nome == self.config.botao_proxima_pagina {
                    return self.aplicar_efeito(EfeitoNoEstado::ProximaPagina);
                }
                if *nome == self.config.botao_pagina_anterior {
                    return self.aplicar_efeito(EfeitoNoEstado::PaginaAnterior);
                }
                // Entrada sem ação não conta: ela aparece só de clicar no botão
                // na interface, e não pode apagar em silêncio a função de fábrica.
                if let Some(acao) = self.acao_configurada(nome) {
                    return match acao.efeito_no_estado() {
                        Some(efeito) => self.aplicar_efeito(efeito),
                        None => Reacao::Executar(acao),
                    };
                }
                // Sem configuração, alguns botões têm função de fábrica.
                match padrao_do_botao(nome) {
                    Acao::Nenhuma => Reacao::Nada,
                    acao => match acao.efeito_no_estado() {
                        Some(efeito) => self.aplicar_efeito(efeito),
                        None => Reacao::Executar(acao),
                    },
                }
            }
            // O clique do knob é um botão como os outros, só que sem luz.
            Evento::KnobApertado { apertado: true } => {
                self.processar(&Evento::Botao {
                    nome: BOTAO_KNOB,
                    apertado: true,
                })
            }
            _ => Reacao::Nada,
        }
    }

    /// Vai direto para uma página, contando de 1. Usado pela UI.
    /// Aplica um efeito de página vindo de fora do laço de eventos, como o knob.
    pub fn aplicar_efeito_publico(&mut self, efeito: EfeitoNoEstado) -> Reacao {
        self.aplicar_efeito(efeito)
    }

    pub fn ir_para_pagina(&mut self, numero: usize) -> Reacao {
        self.aplicar_efeito(EfeitoNoEstado::IrParaPagina(numero))
    }

    /// Muda o brilho geral. Usado pela touch strip.
    pub fn definir_brilho(&mut self, brilho: u8) {
        self.config.brilho = brilho.min(3);
    }

    pub fn esta_pausado(&self) -> bool {
        self.pausado
    }

    fn aplicar_efeito(&mut self, efeito: EfeitoNoEstado) -> Reacao {
        if efeito == EfeitoNoEstado::PausarRetomar {
            self.pausado = !self.pausado;
            self.apertados.clear();
            return Reacao::Pausado(self.pausado);
        }
        let total = self.total_paginas();
        let antes = self.pagina;
        self.pagina = match efeito {
            // Dá a volta nas duas direções: da última vai para a primeira e vice-versa.
            EfeitoNoEstado::ProximaPagina => (self.pagina + 1) % total,
            EfeitoNoEstado::PaginaAnterior => (self.pagina + total - 1) % total,
            EfeitoNoEstado::IrParaPagina(n) => n.saturating_sub(1).min(total - 1),
            // Tratado logo acima, antes de mexer em página.
            EfeitoNoEstado::PausarRetomar => unreachable!(),
        };
        if self.pagina == antes {
            Reacao::Nada
        } else {
            Reacao::PaginaMudou {
                numero: self.numero_pagina(),
                nome: self.nome_pagina().to_string(),
            }
        }
    }

    /// Pinta sem saber o que está aberto. Usado em teste e onde não há vigia.
    pub fn pintar(&self, frame: &mut FrameLeds) {
        self.pintar_com(frame, &Abertos::default(), None);
    }

    /// As cores de repouso dos 16 pads, já com brilho por pad e cor de programa
    /// aberto. É o que a Respiração e o eco usam de base.
    pub fn quadro_de_repouso(&self, abertos: &Abertos) -> Quadro {
        let brilho = self.config.brilho.min(3);
        let mut q = [(Cor::Apagado, 0u8); 16];
        for pad in 1..=16u8 {
            if let Some(c) = self.controle(pad) {
                q[pad as usize - 1] = (
                    cor_do_controle(c, abertos),
                    c.brilho.unwrap_or(brilho).min(3),
                );
            }
        }
        q
    }

    /// Pinta o frame de LEDs inteiro a partir do estado atual.
    /// Chamado depois de cada mudança; o frame só fica sujo se algo mudou de verdade.
    ///
    /// `luz` é o quadro da animação, se houver: ele pinta os pads no lugar das
    /// cores de repouso, e a strip cai para o fraco. Botões não animam.
    pub fn pintar_com(&self, frame: &mut FrameLeds, abertos: &Abertos, luz: Option<&Quadro>) {
        if self.pausado {
            frame.limpar();
            // Deixa só o botão de retomar aceso, senão o aparelho fica sem pista
            // de como voltar.
            frame.botao(BOTAO_PAUSAR, BrilhoBotao::Fraco);
            return;
        }
        let brilho = self.config.brilho.min(3);

        for pad in 1..=16u8 {
            let (cor, b) = match (luz, self.controle(pad)) {
                // Pad apertado sempre mostra o aperto, animação ou não.
                (_, Some(c)) if self.apertados.contains(&pad) => {
                    (c.cor_pressionado.unwrap_or(Cor::Branco), 3)
                }
                (Some(q), _) => q[pad as usize - 1],
                // O brilho do próprio pad ganha do geral, quando existe.
                (None, Some(c)) => (
                    cor_do_controle(c, abertos),
                    c.brilho.unwrap_or(brilho).min(3),
                ),
                (None, None) => (Cor::Apagado, 0),
            };
            frame.pad(pad, cor, b);
        }

        // Os botões de página ficam acesos só quando há mais de uma página.
        let brilho_pagina = if self.total_paginas() > 1 {
            BrilhoBotao::Fraco
        } else {
            BrilhoBotao::Apagado
        };
        frame.botao(&self.config.botao_proxima_pagina, brilho_pagina);
        frame.botao(&self.config.botao_pagina_anterior, brilho_pagina);

        // A strip acompanha o que ela controla: volume, brilho ou página. O LED
        // dela é azul de fábrica, então só a quantidade acesa carrega informação.
        if let Some(nivel) = self.nivel_strip {
            let acesos = (nivel * NUM_STRIP as f32).round() as usize;
            // Com a luz animando, a strip cai para o fraco e segue mostrando o nível.
            let brilho_strip = if luz.is_some() { 0 } else { brilho.max(1) };
            for i in 0..NUM_STRIP {
                let b = if i < acesos { brilho_strip } else { 0 };
                let cor = if i < acesos { Cor::Azul } else { Cor::Apagado };
                frame.strip(i, cor, b);
            }
        }

        // Os três botões de fábrica ficam sempre acesos fraco, para a pessoa
        // achar sem manual. Quem tem função configurada nesta página acende forte.
        for nome in [BOTAO_PAUSAR, BOTAO_FAVORITOS, BOTAO_BUSCA] {
            frame.botao(nome, BrilhoBotao::Fraco);
        }
        if let Some(pagina) = self.config.paginas.get(self.pagina) {
            for (nome, controle) in &pagina.botoes {
                // Entrada sem ação não acende: ela nasce só de clicar no botão na
                // interface, e um LED aceso prometeria uma função que não existe.
                if controle.acao != Acao::Nenhuma {
                    frame.botao(nome, BrilhoBotao::Forte);
                }
            }
        }
    }
}

/// Cor de repouso de um pad, já contando se o programa dele está aberto.
fn cor_do_controle(controle: &Controle, abertos: &Abertos) -> Cor {
    let Some(cor_aberto) = controle.cor_aberto else {
        return controle.cor;
    };
    match &controle.acao {
        Acao::AbrirPrograma { caminho, .. } if abertos.tem(caminho) => cor_aberto,
        _ => controle.cor,
    }
}

/// Nome do knob dentro da configuração. Ele não está na tabela `BOTOES` porque não
/// tem LED, mas por fora é um botão programável como qualquer outro.
pub const BOTAO_KNOB: &str = "knob";

/// Botões com função de fábrica, usada só quando a página não configura nada.
/// Todos continuam remapeáveis: basta configurar o botão na página.
pub const BOTAO_PAUSAR: &str = "maschine";
pub const BOTAO_FAVORITOS: &str = "estrela";
pub const BOTAO_BUSCA: &str = "busca";

fn padrao_do_botao(nome: &str) -> Acao {
    match nome {
        // O círculo dentro do círculo liga e desliga o MikroDeck.
        BOTAO_PAUSAR => Acao::PausarRetomar,
        // A estrela vai para a primeira página, que faz o papel de favoritos.
        BOTAO_FAVORITOS => Acao::IrParaPagina { numero: 1 },
        // A lupa abre a busca do Windows.
        BOTAO_BUSCA => Acao::Atalho {
            teclas: "win".into(),
        },
        _ => Acao::Nenhuma,
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::config::Pagina;
    use std::collections::BTreeMap;

    fn config_de_teste() -> Config {
        let mut p1 = BTreeMap::new();
        p1.insert(
            13,
            Controle {
                nome: "Abrir".into(),
                acao: Acao::AbrirUrl {
                    url: "https://exemplo.com".into(),
                },
                cor: Cor::Azul,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        p1.insert(
            1,
            Controle {
                nome: "Ir para a 2".into(),
                acao: Acao::ProximaPagina,
                cor: Cor::Verde,
                cor_pressionado: Some(Cor::Vermelho),
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        Config {
            versao: 1,
            paginas: vec![
                Pagina {
                    nome: "Um".into(),
                    pads: p1,
                    botoes: BTreeMap::new(),
                },
                Pagina {
                    nome: "Dois".into(),
                    pads: BTreeMap::new(),
                    botoes: BTreeMap::new(),
                },
            ],
            botao_proxima_pagina: "seta_direita".into(),
            botao_pagina_anterior: "seta_esquerda".into(),
            brilho: 2,
            strip: crate::config::FuncaoStrip::Nenhuma,
            calibracao_strip: Default::default(),
            knob: Default::default(),
            home_assistant: Default::default(),
            descanso: Default::default(),
            ao_apertar: Default::default(),
        }
    }

    #[test]
    fn botao_maschine_pausa_e_retoma() {
        let mut e = Estado::novo(config_de_teste());
        assert!(!e.esta_pausado());
        let r = e.processar(&Evento::Botao { nome: "maschine", apertado: true });
        assert_eq!(r, Reacao::Pausado(true));
        assert!(e.esta_pausado());
        let r = e.processar(&Evento::Botao { nome: "maschine", apertado: true });
        assert_eq!(r, Reacao::Pausado(false));
        assert!(!e.esta_pausado());
    }

    #[test]
    fn pausado_ignora_pad_e_troca_de_pagina() {
        let mut e = Estado::novo(config_de_teste());
        e.processar(&Evento::Botao { nome: "maschine", apertado: true });
        assert_eq!(
            e.processar(&Evento::PadApertado { pad: 13, pressao: 900 }),
            Reacao::Nada
        );
        e.processar(&Evento::Botao { nome: "seta_direita", apertado: true });
        assert_eq!(e.numero_pagina(), 1, "pausado não troca de página");
    }

    #[test]
    fn pausado_apaga_os_leds_menos_o_botao_de_retomar() {
        let mut e = Estado::novo(config_de_teste());
        e.processar(&Evento::Botao { nome: "maschine", apertado: true });
        let mut f = FrameLeds::novo();
        e.pintar(&mut f);
        assert_eq!(f.bytes()[bruto(13)], 0, "pad deve apagar quando pausado");
        let i = crate::hid::protocolo::BOTOES.iter().position(|&b| b == BOTAO_PAUSAR).unwrap();
        assert_ne!(f.bytes()[crate::hid::protocolo::OFFSET_BOTOES + i], 0);
    }

    /// Onde o byte de um botão cai dentro do frame de LEDs.
    fn posicao_do_botao(nome: &str) -> usize {
        use crate::hid::protocolo::{BOTOES, OFFSET_BOTOES};
        OFFSET_BOTOES + BOTOES.iter().position(|&b| b == nome).unwrap()
    }

    #[test]
    fn botao_de_pagina_e_reservado_e_ganha_de_configuracao() {
        // Programar por cima da seta não pode roubar a navegação: senão a pessoa
        // se tranca fora das páginas sem entender por quê.
        let mut config = config_de_teste();
        let seta = config.botao_proxima_pagina.clone();
        config.paginas[0].botoes.insert(
            seta.clone(),
            Controle {
                nome: "Copiar".into(),
                acao: Acao::Atalho {
                    teclas: "ctrl+c".into(),
                },
                cor: Cor::Azul,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        let mut estado = Estado::novo(config);
        let r = estado.processar(&Evento::Botao {
            nome: Box::leak(seta.into_boxed_str()),
            apertado: true,
        });
        assert!(
            matches!(r, Reacao::PaginaMudou { .. }),
            "a seta deixou de trocar de página: {r:?}"
        );
    }

    #[test]
    fn o_clique_do_knob_dispara_a_acao_configurada() {
        let mut config = config_de_teste();
        config.paginas[0].botoes.insert(
            BOTAO_KNOB.to_string(),
            Controle {
                nome: "Bloquear".into(),
                acao: Acao::Atalho {
                    teclas: "win+l".into(),
                },
                cor: Cor::Azul,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        let mut estado = Estado::novo(config);
        let r = estado.processar(&Evento::KnobApertado { apertado: true });
        assert_eq!(
            r,
            Reacao::Executar(Acao::Atalho {
                teclas: "win+l".into()
            })
        );
    }

    #[test]
    fn soltar_o_knob_nao_dispara_nada() {
        let mut estado = Estado::novo(config_de_teste());
        assert_eq!(
            estado.processar(&Evento::KnobApertado { apertado: false }),
            Reacao::Nada
        );
    }

    #[test]
    fn o_knob_nao_acende_nada_porque_nao_tem_led() {
        let mut config = config_de_teste();
        config.paginas[0].botoes.insert(
            BOTAO_KNOB.to_string(),
            Controle {
                nome: "Teste".into(),
                acao: Acao::Atalho {
                    teclas: "ctrl+c".into(),
                },
                cor: Cor::Azul,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        // Pintar com o knob configurado não pode estourar nem sujar byte nenhum
        // fora do lugar: ele simplesmente não existe no frame de LEDs.
        let estado = Estado::novo(config);
        let mut frame = FrameLeds::novo();
        estado.pintar(&mut frame);
        assert_eq!(frame.bytes().len(), 81);
    }

    #[test]
    fn a_strip_acende_so_ate_o_nivel_pedido() {
        let mut estado = Estado::novo(config_de_teste());
        estado.definir_nivel_strip(Some(0.5));
        let mut frame = FrameLeds::novo();
        estado.pintar(&mut frame);
        let inicio = crate::hid::protocolo::OFFSET_STRIP;
        let acesos = (0..NUM_STRIP)
            .filter(|i| frame.bytes()[inicio + i] != 0)
            .count();
        // Metade dos 25, arredondado.
        assert_eq!(acesos, 13);
    }

    #[test]
    fn sem_nivel_a_strip_fica_apagada() {
        let estado = Estado::novo(config_de_teste());
        let mut frame = FrameLeds::novo();
        estado.pintar(&mut frame);
        let inicio = crate::hid::protocolo::OFFSET_STRIP;
        assert!((0..NUM_STRIP).all(|i| frame.bytes()[inicio + i] == 0));
    }

    #[test]
    fn nivel_fora_da_faixa_e_aparado() {
        let mut estado = Estado::novo(config_de_teste());
        estado.definir_nivel_strip(Some(9.0));
        let mut frame = FrameLeds::novo();
        estado.pintar(&mut frame);
        let inicio = crate::hid::protocolo::OFFSET_STRIP;
        assert!((0..NUM_STRIP).all(|i| frame.bytes()[inicio + i] != 0));
    }

    #[test]
    fn trocar_config_nao_volta_para_a_primeira_pagina() {
        let mut estado = Estado::novo(config_de_teste());
        estado.ir_para_pagina(2);
        assert_eq!(estado.numero_pagina(), 2);
        estado.trocar_config(config_de_teste());
        assert_eq!(estado.numero_pagina(), 2, "a página mudou sozinha");
    }

    #[test]
    fn trocar_config_nao_cancela_a_pausa() {
        let mut estado = Estado::novo(config_de_teste());
        estado.processar(&Evento::Botao {
            nome: BOTAO_PAUSAR.into(),
            apertado: true,
        });
        estado.trocar_config(config_de_teste());
        // Continua pausado: só o botão de retomar responde.
        let r = estado.processar(&Evento::PadApertado {
            pad: 1,
            pressao: 2000,
        });
        assert_eq!(r, Reacao::Nada);
    }

    #[test]
    fn trocar_config_com_menos_paginas_cai_na_ultima() {
        let mut estado = Estado::novo(config_de_teste());
        estado.ir_para_pagina(2);
        let mut menor = config_de_teste();
        menor.paginas.truncate(1);
        estado.trocar_config(menor);
        assert_eq!(estado.numero_pagina(), 1);
    }

    #[test]
    fn a_cor_de_aberto_so_vale_para_abrir_programa() {
        let mut controle = Controle {
            nome: "Chrome".into(),
            acao: Acao::AbrirPrograma {
                caminho: "chrome.exe".into(),
                argumentos: vec![],
            },
            cor: Cor::Azul,
            cor_pressionado: None,
            cor_aberto: Some(Cor::Verde),
            brilho: None,
            gerenciar_janela: false,
        };
        let nada = Abertos::default();
        assert_eq!(cor_do_controle(&controle, &nada), Cor::Azul);

        // Ação que não abre programa nunca usa a cor de aberto: não há como saber.
        controle.acao = Acao::Atalho {
            teclas: "ctrl+c".into(),
        };
        assert_eq!(cor_do_controle(&controle, &nada), Cor::Azul);
    }

    #[test]
    fn sem_cor_de_aberto_a_cor_de_repouso_manda() {
        let controle = Controle {
            nome: "Chrome".into(),
            acao: Acao::AbrirPrograma {
                caminho: "chrome.exe".into(),
                argumentos: vec![],
            },
            cor: Cor::Azul,
            cor_pressionado: None,
            cor_aberto: None,
            brilho: None,
            gerenciar_janela: false,
        };
        assert_eq!(cor_do_controle(&controle, &Abertos::default()), Cor::Azul);
    }

    #[test]
    fn botao_so_criado_na_interface_nao_apaga_o_padrao_de_fabrica() {
        // Clicar num botão na interface cria uma entrada com ação `Nenhuma`.
        // Isso não pode matar em silêncio a função de fábrica.
        let mut config = config_de_teste();
        config.paginas[0].botoes.insert(
            BOTAO_PAUSAR.to_string(),
            Controle {
                nome: String::new(),
                acao: Acao::Nenhuma,
                cor: Cor::Azul,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        let mut estado = Estado::novo(config);
        let r = estado.processar(&Evento::Botao {
            nome: BOTAO_PAUSAR.into(),
            apertado: true,
        });
        assert_eq!(r, Reacao::Pausado(true));
    }

    #[test]
    fn botao_sem_acao_nao_acende_forte() {
        let mut config = config_de_teste();
        config.paginas[0].botoes.insert(
            "solo".to_string(),
            Controle {
                nome: String::new(),
                acao: Acao::Nenhuma,
                cor: Cor::Azul,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        let estado = Estado::novo(config);
        let mut frame = FrameLeds::novo();
        estado.pintar(&mut frame);
        let pos = posicao_do_botao("solo");
        assert_eq!(frame.bytes()[pos], BrilhoBotao::Apagado as u8);
    }

    #[test]
    fn botoes_de_fabrica_acendem_fraco_para_serem_achados() {
        let estado = Estado::novo(config_de_teste());
        let mut frame = FrameLeds::novo();
        estado.pintar(&mut frame);
        for nome in [BOTAO_PAUSAR, BOTAO_FAVORITOS, BOTAO_BUSCA] {
            let pos = posicao_do_botao(nome);
            assert_eq!(
                frame.bytes()[pos],
                BrilhoBotao::Fraco as u8,
                "botão de fábrica apagado: {nome}"
            );
        }
    }

    #[test]
    fn botao_configurado_ganha_do_padrao_de_fabrica() {
        let mut config = config_de_teste();
        config.paginas[0].botoes.insert(
            "maschine".into(),
            Controle {
                nome: "Meu".into(),
                acao: Acao::AbrirUrl { url: "https://exemplo.com".into() },
                cor: Cor::Azul,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        let mut e = Estado::novo(config);
        let r = e.processar(&Evento::Botao { nome: "maschine", apertado: true });
        assert!(matches!(r, Reacao::Executar(Acao::AbrirUrl { .. })));
        assert!(!e.esta_pausado(), "o padrão de fábrica não deve rodar");
    }

    #[test]
    fn busca_abre_a_tecla_windows_por_padrao() {
        let mut e = Estado::novo(config_de_teste());
        let r = e.processar(&Evento::Botao { nome: "busca", apertado: true });
        assert_eq!(r, Reacao::Executar(Acao::Atalho { teclas: "win".into() }));
    }

    #[test]
    fn pad_configurado_dispara_a_acao() {
        let mut e = Estado::novo(config_de_teste());
        let r = e.processar(&Evento::PadApertado {
            pad: 13,
            pressao: 500,
        });
        assert_eq!(
            r,
            Reacao::Executar(Acao::AbrirUrl {
                url: "https://exemplo.com".into()
            })
        );
    }

    #[test]
    fn pad_sem_configuracao_nao_faz_nada() {
        let mut e = Estado::novo(config_de_teste());
        let r = e.processar(&Evento::PadApertado {
            pad: 7,
            pressao: 500,
        });
        assert_eq!(r, Reacao::Nada);
    }

    #[test]
    fn pad_de_pagina_troca_de_pagina_sem_executar_acao() {
        let mut e = Estado::novo(config_de_teste());
        let r = e.processar(&Evento::PadApertado {
            pad: 1,
            pressao: 500,
        });
        assert_eq!(
            r,
            Reacao::PaginaMudou {
                numero: 2,
                nome: "Dois".into()
            }
        );
        assert_eq!(e.numero_pagina(), 2);
    }

    #[test]
    fn botao_fisico_troca_de_pagina_nos_dois_sentidos() {
        let mut e = Estado::novo(config_de_teste());
        e.processar(&Evento::Botao {
            nome: "seta_direita",
            apertado: true,
        });
        assert_eq!(e.numero_pagina(), 2);
        e.processar(&Evento::Botao {
            nome: "seta_esquerda",
            apertado: true,
        });
        assert_eq!(e.numero_pagina(), 1);
    }

    #[test]
    fn paginas_dao_a_volta_nas_duas_direcoes() {
        let mut e = Estado::novo(config_de_teste());
        // Da última volta para a primeira.
        e.processar(&Evento::Botao { nome: "seta_direita", apertado: true });
        assert_eq!(e.numero_pagina(), 2);
        e.processar(&Evento::Botao { nome: "seta_direita", apertado: true });
        assert_eq!(e.numero_pagina(), 1, "da última deve voltar para a primeira");
        // Da primeira volta para a última.
        e.processar(&Evento::Botao { nome: "seta_esquerda", apertado: true });
        assert_eq!(e.numero_pagina(), 2, "da primeira deve ir para a última");
    }

    #[test]
    fn soltar_botao_nao_troca_de_pagina() {
        let mut e = Estado::novo(config_de_teste());
        let r = e.processar(&Evento::Botao {
            nome: "seta_direita",
            apertado: false,
        });
        assert_eq!(r, Reacao::Nada);
        assert_eq!(e.numero_pagina(), 1);
    }

    #[test]
    fn pad_apertado_usa_a_cor_de_pressionado() {
        let mut e = Estado::novo(config_de_teste());
        let mut f = FrameLeds::novo();

        e.pintar(&mut f);
        let repouso = f.bytes()[bruto(1)];

        e.processar(&Evento::PadApertado { pad: 1, pressao: 900 });
        // A ação do pad 1 troca de página, então volto para a página 1 para conferir a cor.
        let mut e2 = Estado::novo(config_de_teste());
        e2.apertados.insert(1);
        let mut f2 = FrameLeds::novo();
        e2.pintar(&mut f2);
        assert_ne!(f2.bytes()[bruto(1)], repouso);
        assert_eq!(f2.bytes()[bruto(1)], Cor::Vermelho.byte(3));
    }

    #[test]
    fn pad_sem_configuracao_fica_apagado() {
        let e = Estado::novo(config_de_teste());
        let mut f = FrameLeds::novo();
        e.pintar(&mut f);
        assert_eq!(f.bytes()[bruto(7)], 0, "pad sem config deve ficar apagado");
    }

    #[test]
    fn trocar_de_pagina_repinta_os_pads() {
        let mut e = Estado::novo(config_de_teste());
        let mut f = FrameLeds::novo();
        e.pintar(&mut f);
        assert_ne!(f.bytes()[bruto(13)], 0, "pad 13 aceso na página 1");

        e.processar(&Evento::Botao { nome: "seta_direita", apertado: true });
        e.pintar(&mut f);
        assert_eq!(f.bytes()[bruto(13)], 0, "página 2 não tem pad 13 configurado");
    }

    /// Posição do pad impresso dentro do frame de LEDs.
    fn bruto(impresso: u8) -> usize {
        crate::hid::protocolo::OFFSET_PADS
            + crate::hid::protocolo::pad_impresso_para_bruto(impresso).unwrap()
    }
}
