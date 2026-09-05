//! As ferramentas que o MCP oferece, e o que cada uma faz na config.
//!
//! Tudo aqui mexe em `~/.mikrodeck/config.json`. O MikroDeck fica de olho nesse
//! arquivo e aplica sozinho em até um segundo, então não é preciso reiniciar nada.

use motor::acoes::Acao;
use motor::config::{cor_do_nome, nome_da_cor, Config, Controle, Pagina};
use motor::hid::protocolo::BOTOES;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Texto que aparece no `initialize`, para o modelo saber o que é isto.
pub const INSTRUCOES: &str = "\
Configura o MikroDeck, que transforma o Maschine Mikro MK3 num Stream Deck.
Os pads vao de 1 a 16: 1 e o canto inferior esquerdo, 13 o superior esquerdo.
As paginas contam a partir de 1. Toda mudanca entra em vigor sozinha em ate um
segundo, sem reiniciar nada. Use `ajuda_acoes` antes de escrever uma acao pela
primeira vez, e `listar_cores` antes de escolher uma cor.";

/// Descrição das ferramentas, no formato que o MCP espera.
pub fn catalogo() -> Value {
    json!([
        ferramenta(
            "ler_configuracao",
            "Mostra a configuracao inteira: paginas, pads, botoes e ajustes gerais.",
            json!({"type": "object", "properties": {}})
        ),
        ferramenta(
            "ajuda_acoes",
            "Lista os tipos de acao que um pad ou botao pode ter, com um exemplo de cada.",
            json!({"type": "object", "properties": {}})
        ),
        ferramenta(
            "listar_cores",
            "Lista os nomes de cor aceitos pelos pads.",
            json!({"type": "object", "properties": {}})
        ),
        ferramenta(
            "listar_botoes",
            "Lista os nomes dos botoes fisicos que podem ser programados.",
            json!({"type": "object", "properties": {}})
        ),
        ferramenta(
            "listar_paginas_prontas",
            "Lista as paginas prontas que dao para adicionar de uma vez.",
            json!({"type": "object", "properties": {}})
        ),
        ferramenta(
            "adicionar_pagina_pronta",
            "Adiciona uma pagina pronta no fim da lista. Nao sobrescreve nada.",
            json!({
                "type": "object",
                "properties": {"id": {"type": "string"}},
                "required": ["id"]
            })
        ),
        ferramenta(
            "criar_pagina",
            "Cria uma pagina nova no fim da lista.",
            json!({
                "type": "object",
                "properties": {"nome": {"type": "string"}},
                "required": ["nome"]
            })
        ),
        ferramenta(
            "renomear_pagina",
            "Troca o nome de uma pagina.",
            json!({
                "type": "object",
                "properties": {
                    "pagina": {"type": "integer", "description": "contando de 1"},
                    "nome": {"type": "string"}
                },
                "required": ["pagina", "nome"]
            })
        ),
        ferramenta(
            "apagar_pagina",
            "Apaga uma pagina inteira. Nao da para apagar a ultima que sobrou.",
            json!({
                "type": "object",
                "properties": {"pagina": {"type": "integer"}},
                "required": ["pagina"]
            })
        ),
        ferramenta(
            "definir_pad",
            "Programa um pad. Veja `ajuda_acoes` para o formato de `acao`.",
            json!({
                "type": "object",
                "properties": {
                    "pagina": {"type": "integer", "description": "contando de 1"},
                    "pad": {"type": "integer", "description": "de 1 a 16"},
                    "nome": {"type": "string", "description": "aparece na tela do aparelho"},
                    "acao": {"type": "object"},
                    "cor": {"type": "string"},
                    "cor_pressionado": {"type": "string"},
                    "cor_aberto": {
                        "type": "string",
                        "description": "cor enquanto o programa estiver aberto; so vale para abrir_programa"
                    },
                    "brilho": {"type": "integer", "description": "0 a 3; ausente usa o geral"},
                    "gerenciar_janela": {
                        "type": "boolean",
                        "description": "para abrir_programa e abrir_url: toque alterna a janela, dois toques maximizam, segurar fecha (programa) ou abre outra (link)"
                    }
                },
                "required": ["pagina", "pad", "nome", "acao", "cor"]
            })
        ),
        ferramenta(
            "limpar_pad",
            "Tira a programacao de um pad, deixando ele apagado.",
            json!({
                "type": "object",
                "properties": {
                    "pagina": {"type": "integer"},
                    "pad": {"type": "integer"}
                },
                "required": ["pagina", "pad"]
            })
        ),
        ferramenta(
            "definir_botao",
            "Programa um botao fisico numa pagina. Veja `listar_botoes`.",
            json!({
                "type": "object",
                "properties": {
                    "pagina": {"type": "integer"},
                    "botao": {"type": "string"},
                    "nome": {"type": "string"},
                    "acao": {"type": "object"}
                },
                "required": ["pagina", "botao", "nome", "acao"]
            })
        ),
        ferramenta(
            "limpar_botao",
            "Tira a programacao de um botao fisico numa pagina.",
            json!({
                "type": "object",
                "properties": {
                    "pagina": {"type": "integer"},
                    "botao": {"type": "string"}
                },
                "required": ["pagina", "botao"]
            })
        ),
        ferramenta(
            "definir_ajustes",
            "Muda os ajustes gerais. Manda so o que quer trocar.",
            json!({
                "type": "object",
                "properties": {
                    "brilho": {"type": "integer", "description": "0 a 3"},
                    "strip": {
                        "type": "string",
                        "enum": ["nenhuma", "volume", "brilho_pads", "paginas"]
                    },
                    "descanso_ativo": {"type": "boolean"},
                    "descanso_texto": {"type": "string"},
                    "descanso_segundos": {"type": "integer"},
                    "knob": {
                        "type": "string",
                        "enum": ["nenhuma", "volume", "brilho_pads", "paginas", "rolagem"],
                        "description": "o que girar o knob faz"
                    },
                    "home_assistant_endereco": {"type": "string"},
                    "home_assistant_token": {"type": "string"}
                }
            })
        )
    ])
}

fn ferramenta(nome: &str, descricao: &str, esquema: Value) -> Value {
    json!({"name": nome, "description": descricao, "inputSchema": esquema})
}

/// Executa uma ferramenta. Devolve o texto que vai para o modelo, ou um erro.
pub fn executar(nome: &str, argumentos: &Value) -> Result<String, String> {
    match nome {
        "ler_configuracao" => {
            let c = carregar()?;
            Ok(resumo(&c))
        }
        "ajuda_acoes" => Ok(AJUDA_ACOES.to_string()),
        "listar_cores" => Ok(format!("Cores: {}", cores().join(", "))),
        "listar_botoes" => Ok(format!(
            "Botoes: {}, {} (clique do knob, sem luz)",
            BOTOES.join(", "),
            motor::estado::BOTAO_KNOB
        )),

        "listar_paginas_prontas" => {
            let linhas: Vec<String> = motor::prontas::catalogo()
                .into_iter()
                .map(|p| format!("{}: {} — {}", p.id, p.nome, p.descricao))
                .collect();
            Ok(linhas.join("
"))
        }

        "adicionar_pagina_pronta" => {
            let id = texto(argumentos, "id")?;
            let pagina = motor::prontas::montar(&id).ok_or_else(|| {
                format!("pagina pronta \"{id}\" nao existe; use `listar_paginas_prontas`")
            })?;
            let nome_pagina = pagina.nome.clone();
            let mut c = carregar()?;
            c.paginas.push(pagina);
            let total = c.paginas.len();
            salvar(&c)?;
            Ok(format!("Pagina {total} \"{nome_pagina}\" adicionada."))
        }

        "criar_pagina" => {
            let nome_pagina = texto(argumentos, "nome")?;
            let mut c = carregar()?;
            c.paginas.push(Pagina {
                nome: nome_pagina.clone(),
                pads: BTreeMap::new(),
                botoes: BTreeMap::new(),
            });
            let total = c.paginas.len();
            salvar(&c)?;
            Ok(format!("Pagina {total} criada com o nome \"{nome_pagina}\"."))
        }

        "renomear_pagina" => {
            let mut c = carregar()?;
            let i = indice_pagina(&c, argumentos)?;
            let nome_novo = texto(argumentos, "nome")?;
            c.paginas[i].nome = nome_novo.clone();
            salvar(&c)?;
            Ok(format!("Pagina {} agora se chama \"{nome_novo}\".", i + 1))
        }

        "apagar_pagina" => {
            let mut c = carregar()?;
            if c.paginas.len() <= 1 {
                return Err("nao da para apagar a unica pagina que existe".into());
            }
            let i = indice_pagina(&c, argumentos)?;
            let apagada = c.paginas.remove(i);
            salvar(&c)?;
            Ok(format!("Pagina \"{}\" apagada.", apagada.nome))
        }

        "definir_pad" => {
            let mut c = carregar()?;
            let i = indice_pagina(&c, argumentos)?;
            let pad = numero_do_pad(argumentos)?;
            let controle = montar_controle(argumentos)?;
            let nome_controle = controle.nome.clone();
            c.paginas[i].pads.insert(pad, controle);
            salvar(&c)?;
            Ok(format!(
                "Pad {pad} da pagina {} virou \"{nome_controle}\".",
                i + 1
            ))
        }

        "limpar_pad" => {
            let mut c = carregar()?;
            let i = indice_pagina(&c, argumentos)?;
            let pad = numero_do_pad(argumentos)?;
            match c.paginas[i].pads.remove(&pad) {
                Some(_) => {
                    salvar(&c)?;
                    Ok(format!("Pad {pad} da pagina {} limpo.", i + 1))
                }
                None => Ok(format!("Pad {pad} da pagina {} ja estava vazio.", i + 1)),
            }
        }

        "definir_botao" => {
            let mut c = carregar()?;
            let i = indice_pagina(&c, argumentos)?;
            let botao = nome_de_botao(argumentos)?;
            let controle = montar_controle(argumentos)?;
            c.paginas[i].botoes.insert(botao.clone(), controle);
            salvar(&c)?;
            Ok(format!("Botao {botao} programado na pagina {}.", i + 1))
        }

        "limpar_botao" => {
            let mut c = carregar()?;
            let i = indice_pagina(&c, argumentos)?;
            let botao = nome_de_botao(argumentos)?;
            c.paginas[i].botoes.remove(&botao);
            salvar(&c)?;
            Ok(format!("Botao {botao} da pagina {} limpo.", i + 1))
        }

        "definir_ajustes" => {
            let mut c = carregar()?;
            let mut mudou = Vec::new();
            if let Some(b) = argumentos.get("brilho").and_then(Value::as_u64) {
                c.brilho = (b as u8).min(3);
                mudou.push(format!("brilho {}", c.brilho));
            }
            if let Some(k) = argumentos.get("knob").and_then(Value::as_str) {
                c.knob = serde_json::from_value(json!(k))
                    .map_err(|_| format!("funcao de knob desconhecida: {k}"))?;
                mudou.push(format!("knob {k}"));
            }
            if let Some(s) = argumentos.get("strip").and_then(Value::as_str) {
                c.strip = serde_json::from_value(json!(s))
                    .map_err(|_| format!("funcao de strip desconhecida: {s}"))?;
                mudou.push(format!("strip {s}"));
            }
            if let Some(v) = argumentos.get("descanso_ativo").and_then(Value::as_bool) {
                c.descanso.ativo = v;
                mudou.push(format!("descanso {}", if v { "ligado" } else { "desligado" }));
            }
            if let Some(v) = argumentos.get("descanso_texto").and_then(Value::as_str) {
                c.descanso.texto = v.to_string();
                mudou.push("texto do descanso".into());
            }
            if let Some(v) = argumentos.get("descanso_segundos").and_then(Value::as_u64) {
                c.descanso.segundos = v.max(5);
                mudou.push(format!("descanso depois de {}s", c.descanso.segundos));
            }
            if let Some(v) = argumentos
                .get("home_assistant_endereco")
                .and_then(Value::as_str)
            {
                c.home_assistant.endereco = v.to_string();
                mudou.push("endereco do Home Assistant".into());
            }
            if let Some(v) = argumentos.get("home_assistant_token").and_then(Value::as_str) {
                c.home_assistant.token = v.to_string();
                mudou.push("token do Home Assistant".into());
            }
            if mudou.is_empty() {
                return Ok("Nada para mudar.".into());
            }
            salvar(&c)?;
            Ok(format!("Ajustado: {}.", mudou.join(", ")))
        }

        outro => Err(format!("ferramenta desconhecida: {outro}")),
    }
}

const AJUDA_ACOES: &str = r#"O campo `acao` e um objeto com a chave `tipo`. Formatos:

{"tipo":"nenhuma"}
{"tipo":"abrir_programa","caminho":"C:\\...\\app.exe","argumentos":[]}
{"tipo":"abrir_url","url":"https://claude.ai"}
{"tipo":"comando","linha":"shutdown /h"}
{"tipo":"atalho","teclas":"ctrl+shift+n"}
{"tipo":"midia","tecla":"tocar_pausar"}
{"tipo":"proxima_pagina"}
{"tipo":"pagina_anterior"}
{"tipo":"ir_para_pagina","numero":2}
{"tipo":"pausar_retomar"}
{"tipo":"home_assistant","servico":"light.toggle","entidade":"light.sala"}
{"tipo":"http","url":"http://casa:8123/api/webhook/abc","metodo":"post","cabecalhos":{},"corpo":null}

Teclas de midia: tocar_pausar, proxima, anterior, parar, aumentar_volume,
diminuir_volume, mudo.
Atalhos: separe com +. Vale ctrl, shift, alt, win, f1 a f24, letras e numeros.
O Home Assistant precisa de endereco e token em `definir_ajustes` antes de funcionar."#;

fn cores() -> Vec<&'static str> {
    use motor::hid::Cor;
    [
        Cor::Vermelho,
        Cor::Laranja,
        Cor::LaranjaClaro,
        Cor::AmareloQuente,
        Cor::Amarelo,
        Cor::Lima,
        Cor::Verde,
        Cor::Menta,
        Cor::Ciano,
        Cor::Turquesa,
        Cor::Azul,
        Cor::Ameixa,
        Cor::Violeta,
        Cor::Roxo,
        Cor::Magenta,
        Cor::Fucsia,
        Cor::Branco,
    ]
    .into_iter()
    .map(nome_da_cor)
    .collect()
}

fn carregar() -> Result<Config, String> {
    let caminho = Config::caminho_padrao();
    Config::carregar_ou_criar(&caminho).map_err(|e| format!("nao deu para ler a config: {e}"))
}

fn salvar(config: &Config) -> Result<(), String> {
    let caminho = Config::caminho_padrao();
    config
        .salvar(&caminho)
        .map_err(|e| format!("nao deu para gravar a config: {e}"))
}

fn texto(argumentos: &Value, chave: &str) -> Result<String, String> {
    argumentos
        .get(chave)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("falta o campo \"{chave}\""))
}

fn indice_pagina(config: &Config, argumentos: &Value) -> Result<usize, String> {
    let numero = argumentos
        .get("pagina")
        .and_then(Value::as_u64)
        .ok_or("falta o campo \"pagina\"")?;
    if numero == 0 || numero as usize > config.paginas.len() {
        return Err(format!(
            "pagina {numero} nao existe; ha {} paginas",
            config.paginas.len()
        ));
    }
    Ok(numero as usize - 1)
}

fn numero_do_pad(argumentos: &Value) -> Result<u8, String> {
    let pad = argumentos
        .get("pad")
        .and_then(Value::as_u64)
        .ok_or("falta o campo \"pad\"")?;
    if !(1..=16).contains(&pad) {
        return Err(format!("pad {pad} nao existe; os pads vao de 1 a 16"));
    }
    Ok(pad as u8)
}

fn nome_de_botao(argumentos: &Value) -> Result<String, String> {
    let botao = texto(argumentos, "botao")?;
    // O knob nao esta na tabela de botoes porque nao tem LED, mas o clique dele
    // e programavel como qualquer outro.
    if botao != motor::estado::BOTAO_KNOB && !BOTOES.contains(&botao.as_str()) {
        return Err(format!(
            "botao \"{botao}\" nao existe; use `listar_botoes` para ver os nomes"
        ));
    }
    Ok(botao)
}

fn montar_controle(argumentos: &Value) -> Result<Controle, String> {
    let acao_bruta = argumentos
        .get("acao")
        .cloned()
        .ok_or("falta o campo \"acao\"")?;
    let acao: Acao = serde_json::from_value(acao_bruta)
        .map_err(|e| format!("acao invalida ({e}); chame `ajuda_acoes` para ver os formatos"))?;

    // Botao e monocromatico, entao a cor so importa no pad. Um padrao evita
    // obrigar quem esta programando um botao a escolher cor a toa.
    let cor = match argumentos.get("cor").and_then(Value::as_str) {
        Some(nome) => {
            cor_do_nome(nome).ok_or_else(|| format!("cor \"{nome}\" nao existe"))?
        }
        None => motor::hid::Cor::Azul,
    };
    let cor_pressionado = match argumentos.get("cor_pressionado").and_then(Value::as_str) {
        Some(nome) => {
            Some(cor_do_nome(nome).ok_or_else(|| format!("cor \"{nome}\" nao existe"))?)
        }
        None => None,
    };
    let cor_aberto = match argumentos.get("cor_aberto").and_then(Value::as_str) {
        Some(nome) => {
            Some(cor_do_nome(nome).ok_or_else(|| format!("cor \"{nome}\" nao existe"))?)
        }
        None => None,
    };
    let brilho = argumentos
        .get("brilho")
        .and_then(Value::as_u64)
        .map(|b| (b as u8).min(3));

    Ok(Controle {
        gerenciar_janela: argumentos
            .get("gerenciar_janela")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        nome: texto(argumentos, "nome")?,
        acao,
        cor,
        cor_pressionado,
        cor_aberto,
        brilho,
    })
}

/// Config em texto, do jeito que um modelo lê melhor que JSON cru.
fn resumo(config: &Config) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "Brilho geral: {}. Touch strip: {}. Descanso: {}.\n",
        config.brilho,
        serde_json::to_string(&config.strip).unwrap_or_default(),
        if config.descanso.ativo {
            format!(
                "\"{}\" depois de {}s",
                config.descanso.texto, config.descanso.segundos
            )
        } else {
            "desligado".into()
        }
    ));
    s.push_str(&format!(
        "Home Assistant: {}.\n",
        if config.home_assistant.endereco.is_empty() {
            "nao configurado".to_string()
        } else {
            format!("{} (token guardado)", config.home_assistant.endereco)
        }
    ));
    s.push_str(&format!(
        "Troca de pagina: {} volta, {} avanca.\n",
        config.botao_pagina_anterior, config.botao_proxima_pagina
    ));
    for (i, pagina) in config.paginas.iter().enumerate() {
        s.push_str(&format!("\nPagina {} \"{}\"\n", i + 1, pagina.nome));
        if pagina.pads.is_empty() && pagina.botoes.is_empty() {
            s.push_str("  (vazia)\n");
        }
        for (pad, c) in &pagina.pads {
            s.push_str(&format!(
                "  pad {pad}: \"{}\" [{}] {}\n",
                c.nome,
                nome_da_cor(c.cor),
                serde_json::to_string(&c.acao).unwrap_or_default()
            ));
        }
        for (botao, c) in &pagina.botoes {
            s.push_str(&format!(
                "  botao {botao}: \"{}\" {}\n",
                c.nome,
                serde_json::to_string(&c.acao).unwrap_or_default()
            ));
        }
    }
    s
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_catalogo_traz_todas_as_ferramentas_com_esquema() {
        let lista = catalogo();
        let itens = lista.as_array().unwrap();
        assert!(itens.len() >= 14);
        for f in itens {
            assert!(f["name"].is_string(), "ferramenta sem nome");
            assert!(f["description"].is_string(), "ferramenta sem descricao");
            assert!(f["inputSchema"].is_object(), "ferramenta sem esquema");
        }
    }

    #[test]
    fn pagina_pronta_que_nao_existe_aponta_a_lista() {
        let e = executar("adicionar_pagina_pronta", &json!({"id": "voar"})).unwrap_err();
        assert!(e.contains("listar_paginas_prontas"), "erro sem dica: {e}");
    }

    #[test]
    fn a_lista_de_prontas_traz_id_e_descricao() {
        let texto = executar("listar_paginas_prontas", &json!({})).unwrap();
        assert!(texto.contains("spotify"));
        assert!(texto.contains("—"));
    }

    #[test]
    fn ferramenta_desconhecida_devolve_erro() {
        assert!(executar("voar", &json!({})).is_err());
    }

    #[test]
    fn pad_fora_da_faixa_e_recusado() {
        assert!(numero_do_pad(&json!({"pad": 0})).is_err());
        assert!(numero_do_pad(&json!({"pad": 17})).is_err());
        assert_eq!(numero_do_pad(&json!({"pad": 16})).unwrap(), 16);
    }

    #[test]
    fn botao_que_nao_existe_e_recusado_com_dica() {
        let e = nome_de_botao(&json!({"botao": "turbo"})).unwrap_err();
        assert!(e.contains("listar_botoes"), "erro sem dica: {e}");
        assert!(nome_de_botao(&json!({"botao": "mute"})).is_ok());
        assert!(
            nome_de_botao(&json!({"botao": "knob"})).is_ok(),
            "o clique do knob tem que ser programavel"
        );
    }

    #[test]
    fn acao_invalida_aponta_para_a_ajuda() {
        let e = montar_controle(&json!({"nome": "x", "acao": {"tipo": "voar"}})).unwrap_err();
        assert!(e.contains("ajuda_acoes"), "erro sem dica: {e}");
    }

    #[test]
    fn monta_o_controle_com_cor_e_brilho() {
        let c = montar_controle(&json!({
            "nome": "Chrome",
            "acao": {"tipo": "abrir_url", "url": "https://exemplo.com"},
            "cor": "azul",
            "cor_pressionado": "branco",
            "brilho": 9
        }))
        .unwrap();
        assert_eq!(c.nome, "Chrome");
        assert_eq!(nome_da_cor(c.cor), "azul");
        assert_eq!(c.brilho, Some(3), "brilho tem que ser limitado a 3");
    }

    #[test]
    fn aceita_a_cor_de_programa_aberto() {
        let c = montar_controle(&json!({
            "nome": "Chrome",
            "acao": {"tipo": "abrir_programa", "caminho": "chrome.exe"},
            "cor": "azul",
            "cor_aberto": "verde"
        }))
        .unwrap();
        assert_eq!(c.cor_aberto.map(nome_da_cor), Some("verde"));
    }

    #[test]
    fn botao_sem_cor_ganha_um_padrao() {
        let c = montar_controle(&json!({
            "nome": "Mudo",
            "acao": {"tipo": "midia", "tecla": "mudo"}
        }))
        .unwrap();
        assert_eq!(nome_da_cor(c.cor), "azul");
    }

    #[test]
    fn pagina_fora_da_faixa_diz_quantas_existem() {
        let c = Config::exemplo();
        let e = indice_pagina(&c, &json!({"pagina": 99})).unwrap_err();
        assert!(e.contains("99"));
        assert_eq!(indice_pagina(&c, &json!({"pagina": 1})).unwrap(), 0);
    }

    #[test]
    fn o_resumo_mostra_pads_e_paginas() {
        let texto = resumo(&Config::exemplo());
        assert!(texto.contains("Pagina 1"));
        assert!(texto.contains("pad "));
        assert!(texto.contains("Brilho geral"));
    }

    #[test]
    fn a_ajuda_cobre_todos_os_tipos_de_acao() {
        // Se alguem criar uma acao nova e esquecer da ajuda, o modelo nao vai saber.
        for tipo in [
            "nenhuma",
            "abrir_programa",
            "abrir_url",
            "comando",
            "atalho",
            "midia",
            "proxima_pagina",
            "pagina_anterior",
            "ir_para_pagina",
            "pausar_retomar",
            "home_assistant",
            "http",
        ] {
            assert!(
                AJUDA_ACOES.contains(tipo),
                "a ajuda nao fala do tipo {tipo}"
            );
        }
    }
}
