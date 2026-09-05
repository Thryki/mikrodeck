//! Páginas prontas, para a pessoa não começar de uma tela em branco.
//!
//! Nada aqui sobrescreve o que já existe: cada página pronta é adicionada no fim
//! da lista. Os caminhos e entidades são palpites razoáveis, feitos para serem
//! editados depois.

use crate::acoes::{Acao, TeclaMidia};
use crate::config::{Controle, Pagina};
use crate::hid::Cor;
use serde::Serialize;
use std::collections::BTreeMap;

/// Uma página pronta com um rótulo para a interface mostrar.
#[derive(Debug, Clone, Serialize)]
pub struct Pronta {
    pub id: String,
    pub nome: String,
    pub descricao: String,
}

/// O que existe de página pronta, na ordem em que aparece na interface.
pub fn catalogo() -> Vec<Pronta> {
    vec![
        pronta("spotify", "Spotify", "Tocar, pular e mexer no volume, com os botões PLAY e STOP do aparelho."),
        pronta("claude", "Claude", "Abrir o Claude, começar um chat novo e ir para os projetos."),
        pronta("casa", "Casa", "Exemplos de automação pelo Home Assistant, para você trocar pelas suas entidades."),
        pronta("trabalho", "Trabalho", "Copiar, colar, desfazer, print da tela e as janelas virtuais do Windows."),
    ]
}

fn pronta(id: &str, nome: &str, descricao: &str) -> Pronta {
    Pronta {
        id: id.into(),
        nome: nome.into(),
        descricao: descricao.into(),
    }
}

/// Monta a página daquele id. `None` se o id não existe.
pub fn montar(id: &str) -> Option<Pagina> {
    match id {
        "spotify" => Some(spotify()),
        "claude" => Some(claude()),
        "casa" => Some(casa()),
        "trabalho" => Some(trabalho()),
        _ => None,
    }
}

fn spotify() -> Pagina {
    let mut pads = BTreeMap::new();
    // A fileira de cima é o Spotify em si, e ela muda de cor quando ele está aberto.
    pads.insert(
        13,
        pad_aberto(
            "Spotify",
            Acao::AbrirPrograma {
                caminho: "spotify.exe".into(),
                argumentos: vec![],
            },
            Cor::Verde,
            Cor::Lima,
        ),
    );
    pads.insert(14, pad_midia("Tocar", TeclaMidia::TocarPausar, Cor::Menta));
    pads.insert(15, pad_midia("Voltar", TeclaMidia::Anterior, Cor::Turquesa));
    pads.insert(16, pad_midia("Pular", TeclaMidia::Proxima, Cor::Turquesa));
    pads.insert(
        9,
        pad_midia("Volume -", TeclaMidia::DiminuirVolume, Cor::Azul),
    );
    pads.insert(
        10,
        pad_midia("Volume +", TeclaMidia::AumentarVolume, Cor::Azul),
    );
    pads.insert(11, pad_midia("Mudo", TeclaMidia::Mudo, Cor::Vermelho));
    pads.insert(
        12,
        pad(
            "Web",
            Acao::AbrirUrl {
                url: "https://open.spotify.com".into(),
            },
            Cor::Verde,
        ),
    );

    // Os botões de transporte do aparelho fazem o que o desenho deles promete.
    let mut botoes = BTreeMap::new();
    botoes.insert(
        "play".into(),
        botao("Tocar", Acao::Midia { tecla: TeclaMidia::TocarPausar }),
    );
    botoes.insert(
        "stop".into(),
        botao("Parar", Acao::Midia { tecla: TeclaMidia::Parar }),
    );
    botoes.insert(
        "restart".into(),
        botao("Voltar", Acao::Midia { tecla: TeclaMidia::Anterior }),
    );
    botoes.insert(
        "tap".into(),
        botao("Pular", Acao::Midia { tecla: TeclaMidia::Proxima }),
    );

    Pagina {
        nome: "Spotify".into(),
        pads,
        botoes,
    }
}

fn claude() -> Pagina {
    let mut pads = BTreeMap::new();
    pads.insert(13, pad_url("Claude", "https://claude.ai", Cor::Laranja));
    pads.insert(
        14,
        pad_url("Chat novo", "https://claude.ai/new", Cor::LaranjaClaro),
    );
    pads.insert(
        15,
        pad_url("Projetos", "https://claude.ai/projects", Cor::AmareloQuente),
    );
    pads.insert(
        16,
        pad_url("Console", "https://console.anthropic.com", Cor::Amarelo),
    );
    pads.insert(
        9,
        pad_url("Documentação", "https://docs.claude.com", Cor::Ameixa),
    );
    Pagina {
        nome: "Claude".into(),
        pads,
        botoes: BTreeMap::new(),
    }
}

fn casa() -> Pagina {
    let mut pads = BTreeMap::new();
    pads.insert(13, pad_casa("Luz da sala", "light.toggle", "light.sala", Cor::AmareloQuente));
    pads.insert(14, pad_casa("Luz do quarto", "light.toggle", "light.quarto", Cor::Amarelo));
    pads.insert(15, pad_casa("Tudo apagado", "light.turn_off", "all", Cor::Azul));
    pads.insert(16, pad_casa("Cena noite", "scene.turn_on", "scene.noite", Cor::Ameixa));
    pads.insert(9, pad_casa("Ar condicionado", "switch.toggle", "switch.ar", Cor::Ciano));
    pads.insert(10, pad_casa("Ventilador", "switch.toggle", "switch.ventilador", Cor::Turquesa));
    Pagina {
        nome: "Casa".into(),
        pads,
        botoes: BTreeMap::new(),
    }
}

fn trabalho() -> Pagina {
    let mut pads = BTreeMap::new();
    pads.insert(13, pad_atalho("Desfazer", "ctrl+z", Cor::Violeta));
    pads.insert(14, pad_atalho("Refazer", "ctrl+shift+z", Cor::Violeta));
    pads.insert(15, pad_atalho("Copiar", "ctrl+c", Cor::Azul));
    pads.insert(16, pad_atalho("Colar", "ctrl+v", Cor::Azul));
    pads.insert(9, pad_atalho("Print da tela", "win+shift+s", Cor::Magenta));
    pads.insert(10, pad_atalho("Área nova", "win+ctrl+d", Cor::Ciano));
    pads.insert(11, pad_atalho("Área <", "win+ctrl+left", Cor::Turquesa));
    pads.insert(12, pad_atalho("Área >", "win+ctrl+right", Cor::Turquesa));
    pads.insert(
        5,
        pad(
            "Explorador",
            Acao::AbrirPrograma {
                caminho: "explorer.exe".into(),
                argumentos: vec![],
            },
            Cor::AmareloQuente,
        ),
    );
    pads.insert(6, pad_atalho("Bloquear", "win+l", Cor::Vermelho));
    Pagina {
        nome: "Trabalho".into(),
        pads,
        botoes: BTreeMap::new(),
    }
}

fn pad(nome: &str, acao: Acao, cor: Cor) -> Controle {
    Controle {
        nome: nome.into(),
        acao,
        cor,
        cor_pressionado: None,
        cor_aberto: None,
        brilho: None,
        gerenciar_janela: false,
    }
}

fn pad_aberto(nome: &str, acao: Acao, cor: Cor, cor_aberto: Cor) -> Controle {
    Controle {
        cor_aberto: Some(cor_aberto),
        ..pad(nome, acao, cor)
    }
}

fn pad_url(nome: &str, url: &str, cor: Cor) -> Controle {
    pad(nome, Acao::AbrirUrl { url: url.into() }, cor)
}

fn pad_atalho(nome: &str, teclas: &str, cor: Cor) -> Controle {
    pad(nome, Acao::Atalho { teclas: teclas.into() }, cor)
}

fn pad_midia(nome: &str, tecla: TeclaMidia, cor: Cor) -> Controle {
    pad(nome, Acao::Midia { tecla }, cor)
}

fn pad_casa(nome: &str, servico: &str, entidade: &str, cor: Cor) -> Controle {
    pad(
        nome,
        Acao::HomeAssistant {
            servico: servico.into(),
            entidade: entidade.into(),
        },
        cor,
    )
}

fn botao(nome: &str, acao: Acao) -> Controle {
    pad(nome, acao, Cor::Branco)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn todo_id_do_catalogo_monta_uma_pagina() {
        for p in catalogo() {
            let pagina = montar(&p.id).unwrap_or_else(|| panic!("id sem página: {}", p.id));
            assert!(!pagina.pads.is_empty(), "página {} sem pads", p.id);
        }
    }

    #[test]
    fn id_desconhecido_nao_monta_nada() {
        assert!(montar("nao_existe").is_none());
    }

    #[test]
    fn nenhum_pad_cai_fora_da_faixa_de_1_a_16() {
        for p in catalogo() {
            for pad in montar(&p.id).unwrap().pads.keys() {
                assert!((1..=16).contains(pad), "pad {pad} fora da faixa em {}", p.id);
            }
        }
    }

    #[test]
    fn todo_botao_usado_existe_no_aparelho() {
        use crate::hid::protocolo::BOTOES;
        for p in catalogo() {
            for nome in montar(&p.id).unwrap().botoes.keys() {
                assert!(
                    BOTOES.contains(&nome.as_str()),
                    "botão inventado: {nome} em {}",
                    p.id
                );
            }
        }
    }

    #[test]
    fn a_pagina_da_casa_so_usa_servico_com_dominio() {
        // Sem o ponto, o Home Assistant não sabe para onde mandar.
        for (_, controle) in montar("casa").unwrap().pads {
            let Acao::HomeAssistant { servico, .. } = &controle.acao else {
                panic!("pad da casa sem ação de Home Assistant");
            };
            assert!(servico.contains('.'), "serviço sem domínio: {servico}");
        }
    }
}
