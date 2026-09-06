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
        pronta("navegador", "Navegador", "Abas, histórico, downloads, zoom e tela cheia do navegador."),
        pronta("windows", "Windows", "Encaixar janelas, trocar de app, gravar a tela e a área de transferência."),
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
        "navegador" => Some(navegador()),
        "windows" => Some(windows()),
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

/// Atalhos do navegador. Valem em Chrome, Edge e Firefox sem mudar nada.
fn navegador() -> Pagina {
    let mut pads = BTreeMap::new();
    pads.insert(13, pad_atalho("Aba nova", "ctrl+t", Cor::Ciano));
    pads.insert(14, pad_atalho("Fechar aba", "ctrl+w", Cor::Vermelho));
    pads.insert(15, pad_atalho("Reabrir aba", "ctrl+shift+t", Cor::Lima));
    pads.insert(16, pad_atalho("Anônima", "ctrl+shift+n", Cor::Ameixa));
    pads.insert(9, pad_atalho("Voltar", "alt+left", Cor::Azul));
    pads.insert(10, pad_atalho("Avançar", "alt+right", Cor::Azul));
    pads.insert(11, pad_atalho("Recarregar", "f5", Cor::Turquesa));
    pads.insert(12, pad_atalho("Buscar", "ctrl+f", Cor::Amarelo));
    pads.insert(5, pad_atalho("Zoom +", "ctrl+shift+equal", Cor::Laranja));
    pads.insert(6, pad_atalho("Zoom -", "ctrl+minus", Cor::Laranja));
    pads.insert(7, pad_atalho("Zoom 100%", "ctrl+0", Cor::LaranjaClaro));
    pads.insert(8, pad_atalho("Tela cheia", "f11", Cor::Violeta));
    pads.insert(1, pad_atalho("Histórico", "ctrl+h", Cor::Magenta));
    pads.insert(2, pad_atalho("Downloads", "ctrl+j", Cor::Magenta));
    pads.insert(3, pad_atalho("Favoritar", "ctrl+d", Cor::AmareloQuente));
    pads.insert(4, pad_atalho("Favoritos", "ctrl+shift+o", Cor::AmareloQuente));
    Pagina {
        nome: "Navegador".into(),
        pads,
        botoes: BTreeMap::new(),
    }
}

/// Janelas e teclas do próprio Windows, as que se usa o dia inteiro.
fn windows() -> Pagina {
    let mut pads = BTreeMap::new();
    pads.insert(13, pad_atalho("Encaixar <", "win+left", Cor::Azul));
    pads.insert(14, pad_atalho("Encaixar >", "win+right", Cor::Azul));
    pads.insert(15, pad_atalho("Maximizar", "win+up", Cor::Turquesa));
    pads.insert(16, pad_atalho("Minimizar", "win+down", Cor::Turquesa));
    pads.insert(9, pad_atalho("Trocar app", "alt+tab", Cor::Ciano));
    pads.insert(10, pad_atalho("Mostrar tudo", "win+tab", Cor::Ciano));
    pads.insert(11, pad_atalho("Área de trabalho", "win+d", Cor::Menta));
    pads.insert(12, pad_atalho("Fechar janela", "alt+f4", Cor::Vermelho));
    pads.insert(5, pad_atalho("Recorte", "win+shift+s", Cor::Lima));
    pads.insert(6, pad_atalho("Gravar tela", "win+alt+r", Cor::Vermelho));
    pads.insert(7, pad_atalho("Área de transf.", "win+v", Cor::Violeta));
    pads.insert(8, pad_atalho("Emoji", "win+period", Cor::Amarelo));
    pads.insert(
        1,
        pad(
            "Explorador",
            Acao::AbrirPrograma {
                caminho: "explorer.exe".into(),
                argumentos: vec![],
            },
            Cor::Laranja,
        ),
    );
    pads.insert(2, pad_atalho("Configurações", "win+i", Cor::Branco));
    pads.insert(3, pad_atalho("Projetar", "win+p", Cor::Roxo));
    pads.insert(4, pad_atalho("Bloquear", "win+l", Cor::Fucsia));
    Pagina {
        nome: "Windows".into(),
        pads,
        botoes: BTreeMap::new(),
    }
}

/// Monta as páginas da casa a partir do que o Home Assistant respondeu.
///
/// Cada pad alterna uma entidade. Passando de dezesseis, sobra página: a casa do
/// Davi tem mais dispositivo do que pad, e cortar a lista em silêncio seria pior
/// do que continuar na página seguinte.
pub fn paginas_da_casa(entidades: &[crate::rede::Entidade]) -> Vec<Pagina> {
    if entidades.is_empty() {
        return Vec::new();
    }
    // Divide em páginas do mesmo tamanho em vez de encher uma e deixar o resto
    // sobrando: dezoito dispositivos ficam 9 e 9, não 16 e 2.
    let total = entidades.len().div_ceil(16);
    let por_pagina = entidades.len().div_ceil(total.max(1));
    let partes: Vec<_> = entidades.chunks(por_pagina.max(1)).collect();
    partes
        .iter()
        .enumerate()
        .map(|(i, parte)| {
            let mut pads = BTreeMap::new();
            let nomes = nomes_curtos(parte);
            for (k, e) in parte.iter().enumerate() {
                // Preenche de cima para baixo, na ordem que se lê: 13 a 16,
                // depois 9 a 12, e assim por diante.
                let pad_numero = ORDEM_DE_LEITURA[k];
                pads.insert(
                    pad_numero,
                    pad(
                        &nomes[k],
                        Acao::HomeAssistant {
                            servico: e.servico_de_alternar(),
                            entidade: e.id.clone(),
                        },
                        cor_do_dominio(&e.dominio),
                    ),
                );
            }
            Pagina {
                nome: if total > 1 {
                    format!("Casa {}", i + 1)
                } else {
                    "Casa".to_string()
                },
                pads,
                botoes: BTreeMap::new(),
            }
        })
        .collect()
}

/// Os dezesseis pads na ordem em que se lê o aparelho: de cima para baixo, da
/// esquerda para a direita.
pub const ORDEM_DE_LEITURA: [u8; 16] = [
    13, 14, 15, 16, 9, 10, 11, 12, 5, 6, 7, 8, 1, 2, 3, 4,
];

/// Uma cor por tipo de dispositivo, para bater o olho e saber o que é o quê.
fn cor_do_dominio(dominio: &str) -> Cor {
    match dominio {
        "light" => Cor::AmareloQuente,
        "switch" => Cor::Ciano,
        "fan" => Cor::Turquesa,
        "input_boolean" => Cor::Violeta,
        "humidifier" => Cor::Azul,
        "siren" => Cor::Vermelho,
        "automation" => Cor::Lima,
        "media_player" => Cor::Magenta,
        _ => Cor::Branco,
    }
}

/// Encurta os nomes e desfaz as repetições que o corte criar.
///
/// Cortar o fim é o que faz sentido na maioria dos casos, mas o fim costuma ser
/// justamente o que distingue um dispositivo do irmão dele: "Painel Studio Luz
/// Mesa" e "Painel Studio Luz Studio" viravam o mesmo "Painel Studio Luz". Nos
/// repetidos, o nome passa a guardar a primeira palavra e o fim.
fn nomes_curtos(entidades: &[crate::rede::Entidade]) -> Vec<String> {
    let curtos: Vec<String> = entidades.iter().map(|e| encurtar(&e.nome)).collect();
    // Ambíguo é mais do que repetido. "Painel Sala Cosinha" cortado vira
    // "Painel Sala", que é único mas some com a cozinha e ainda parece o começo
    // de "Painel Sala Sala". Um nome que é começo de outro conta como ambíguo.
    let ambiguo = |i: usize| {
        curtos.iter().enumerate().any(|(j, outro)| {
            i != j && (outro.starts_with(&curtos[i]) || curtos[i].starts_with(outro))
        })
    };
    curtos
        .iter()
        .enumerate()
        .map(|(i, curto)| {
            if ambiguo(i) {
                com_o_fim(&entidades[i].nome)
            } else {
                curto.clone()
            }
        })
        .collect()
}

/// Guarda a primeira palavra e o máximo do fim que couber, com reticências no
/// meio. É o que separa "Painel… Luz Mesa" de "Painel… Luz Studio".
fn com_o_fim(nome: &str) -> String {
    const LIMITE: usize = 18;
    let palavras: Vec<&str> = nome.split_whitespace().collect();
    if palavras.len() < 2 {
        return encurtar(nome);
    }
    let primeira = palavras[0];
    // Cresce o sufixo palavra a palavra enquanto couber.
    let mut sufixo = palavras[palavras.len() - 1].to_string();
    for i in (1..palavras.len() - 1).rev() {
        let tentativa = format!("{} {sufixo}", palavras[i]);
        if primeira.chars().count() + 2 + tentativa.chars().count() > LIMITE {
            break;
        }
        sufixo = tentativa;
    }
    let montado = format!("{primeira}… {sufixo}");
    if montado.chars().count() <= LIMITE {
        montado
    } else {
        // Nem a primeira palavra mais a última cabem: fica só o fim.
        let so_o_fim: String = sufixo.chars().rev().take(LIMITE).collect::<String>().chars().rev().collect();
        so_o_fim
    }
}

/// A tela do aparelho é pequena e o nome do pad aparece nela. Nome comprido
/// vira nome cortado, e cortar no espaço fica melhor do que cortar no meio da
/// palavra.
fn encurtar(nome: &str) -> String {
    const LIMITE: usize = 18;
    let nome = nome.trim();
    if nome.chars().count() <= LIMITE {
        return nome.to_string();
    }
    let curto: String = nome.chars().take(LIMITE).collect();
    let cortado = match curto.rsplit_once(' ') {
        Some((antes, _)) if antes.chars().count() >= 8 => antes.to_string(),
        _ => curto.trim_end().to_string(),
    };
    sem_palavra_solta(&cortado)
}

/// Tira a preposição ou o artigo que ficou pendurado no fim do corte.
/// "Câmera Alarme de" fica "Câmera Alarme".
fn sem_palavra_solta(nome: &str) -> String {
    const SOLTAS: [&str; 10] = ["de", "da", "do", "das", "dos", "e", "em", "a", "o", "no"];
    let mut atual = nome.trim().to_string();
    loop {
        let Some((antes, ultima)) = atual.rsplit_once(' ') else {
            return atual;
        };
        if !SOLTAS.contains(&ultima.to_lowercase().as_str()) || antes.trim().is_empty() {
            return atual;
        }
        atual = antes.trim_end().to_string();
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
    fn todo_atalho_das_prontas_tem_codigo() {
        // Uma tecla sem codigo cancela o atalho inteiro, e em silencio.
        use crate::acoes::teclado::codigo_da_tecla;
        for p in catalogo() {
            for (pad, controle) in montar(&p.id).unwrap().pads {
                if let Acao::Atalho { teclas } = &controle.acao {
                    for parte in teclas.split('+') {
                        assert!(
                            codigo_da_tecla(parte).is_some(),
                            "tecla {parte:?} sem codigo, no pad {pad} de {}",
                            p.id
                        );
                    }
                }
            }
        }
    }

    fn ent(id: &str, nome: &str) -> crate::rede::Entidade {
        crate::rede::Entidade {
            dominio: id.split_once('.').unwrap().0.to_string(),
            id: id.to_string(),
            nome: nome.to_string(),
            ligada: false,
        }
    }

    #[test]
    fn a_casa_vira_pagina_com_um_pad_por_dispositivo() {
        let e = vec![ent("light.sala", "Sala"), ent("switch.cafeteira", "Cafeteira")];
        let p = paginas_da_casa(&e);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].nome, "Casa");
        // Preenche na ordem de leitura: o primeiro cai no canto de cima.
        assert_eq!(p[0].pads[&13].nome, "Sala");
        assert_eq!(p[0].pads[&14].nome, "Cafeteira");
        assert_eq!(
            p[0].pads[&13].acao,
            Acao::HomeAssistant {
                servico: "light.toggle".into(),
                entidade: "light.sala".into()
            }
        );
    }

    #[test]
    fn mais_de_dezesseis_dispositivos_viram_mais_de_uma_pagina() {
        let e: Vec<_> = (0..20)
            .map(|i| ent(&format!("light.l{i}"), &format!("Luz {i}")))
            .collect();
        let p = paginas_da_casa(&e);
        assert_eq!(p.len(), 2, "cortou dispositivo em vez de abrir pagina");
        assert_eq!(p[0].nome, "Casa 1");
        assert_eq!(p[1].nome, "Casa 2");
        assert_eq!(
            p[0].pads.len() + p[1].pads.len(),
            20,
            "perdeu dispositivo no caminho"
        );
    }

    #[test]
    fn as_paginas_saem_equilibradas_em_vez_de_deixar_sobra() {
        // Dezoito dispositivos: 9 e 9 fica melhor que 16 e 2.
        let e: Vec<_> = (0..18)
            .map(|i| ent(&format!("light.l{i}"), &format!("Luz {i}")))
            .collect();
        let p = paginas_da_casa(&e);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].pads.len(), 9);
        assert_eq!(p[1].pads.len(), 9);
    }

    #[test]
    fn nenhuma_pagina_passa_de_dezesseis_pads() {
        for quantos in 1..=100usize {
            let e: Vec<_> = (0..quantos)
                .map(|i| ent(&format!("light.l{i}"), &format!("Luz {i}")))
                .collect();
            let paginas = paginas_da_casa(&e);
            let soma: usize = paginas.iter().map(|p| p.pads.len()).sum();
            assert_eq!(soma, quantos, "perdeu dispositivo com {quantos}");
            for p in &paginas {
                assert!(
                    p.pads.len() <= 16,
                    "{} pads numa pagina com {quantos} dispositivos",
                    p.pads.len()
                );
                assert!(p.pads.len() > 0, "pagina vazia com {quantos}");
            }
        }
    }

    #[test]
    fn sem_dispositivo_nenhum_nao_cria_pagina_vazia() {
        assert!(paginas_da_casa(&[]).is_empty());
    }

    #[test]
    fn o_corte_nao_deixa_preposicao_pendurada() {
        assert_eq!(encurtar("Câmera Alarme de movimento"), "Câmera Alarme");
        assert_eq!(encurtar("Câmera Modo de privacidade"), "Câmera Modo");
        // Palavra solta no meio do nome nao e mexida.
        assert_eq!(encurtar("Luz da sala"), "Luz da sala");
    }

    #[test]
    fn nomes_que_se_repetem_depois_do_corte_ganham_o_fim() {
        // Sem isto, os dois viravam "Painel Studio Luz" e o pad ficava adivinha.
        let e = vec![
            ent("switch.a", "Painel Studio Luz Mesa"),
            ent("switch.b", "Painel Studio Luz Studio"),
            ent("switch.c", "Banheiro"),
        ];
        let nomes = nomes_curtos(&e);
        assert_ne!(nomes[0], nomes[1], "dois pads com o mesmo nome: {nomes:?}");
        assert!(nomes[0].contains("Mesa"), "{:?}", nomes[0]);
        assert!(nomes[1].contains("Studio"), "{:?}", nomes[1]);
        // Quem nao repetia continua como estava.
        assert_eq!(nomes[2], "Banheiro");
        for n in &nomes {
            assert!(n.chars().count() <= 18, "{n} passou do limite");
        }
    }

    #[test]
    fn nome_cortado_que_vira_comeco_de_outro_tambem_e_desfeito() {
        // "Painel Sala Cosinha" cortado virava "Painel Sala", que e unico mas
        // engole a cozinha e parece o comeco de "Painel Sala Sala".
        let e = vec![
            ent("switch.a", "Painel Sala Cosinha"),
            ent("switch.b", "Painel Sala Sala"),
        ];
        let nomes = nomes_curtos(&e);
        assert!(nomes[0].contains("Cosinha"), "{:?}", nomes[0]);
        assert_ne!(nomes[0], nomes[1]);
        assert!(!nomes[1].starts_with(&nomes[0]), "{nomes:?}");
        for n in &nomes {
            assert!(n.chars().count() <= 18, "{n} passou do limite");
        }
    }

    #[test]
    fn o_roteador_tambem_fica_distinguivel() {
        let e = vec![
            ent("switch.a", "Primary router Guest network"),
            ent("switch.b", "Primary router WiFi 6 TWT"),
        ];
        let nomes = nomes_curtos(&e);
        assert_ne!(nomes[0], nomes[1], "{nomes:?}");
        for n in &nomes {
            assert!(n.chars().count() <= 18, "{n} passou do limite");
        }
    }

    #[test]
    fn nome_comprido_e_cortado_no_espaco() {
        assert_eq!(encurtar("Sala"), "Sala");
        assert_eq!(
            encurtar("Lampada da mesa do escritorio"),
            "Lampada da mesa"
        );
        // Sem espaco util, corta seco em vez de estourar a tela.
        assert_eq!(encurtar("Abcdefghijklmnopqrstuvwxyz").chars().count(), 18);
    }

    #[test]
    fn a_ordem_de_leitura_cobre_os_dezesseis_pads_uma_vez_so() {
        let mut vistos: Vec<u8> = ORDEM_DE_LEITURA.to_vec();
        vistos.sort();
        assert_eq!(vistos, (1..=16).collect::<Vec<u8>>());
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
