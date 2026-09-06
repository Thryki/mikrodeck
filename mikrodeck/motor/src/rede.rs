//! Requisições HTTP disparadas por pads.
//!
//! Existe por causa do Home Assistant, que expõe tudo por REST e por webhook,
//! mas serve para qualquer serviço da casa que aceite uma chamada HTTP.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

/// Quanto tempo esperar antes de desistir. Curto de propósito: um pad que trava
/// cinco segundos parece um pad quebrado.
const ESPERA: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metodo {
    Get,
    #[default]
    Post,
    Put,
}


/// Ligação com um Home Assistant. Fica na config geral, não em cada pad, para a
/// pessoa digitar o endereço e o token uma vez só.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistant {
    /// Endereço do servidor, por exemplo `http://homeassistant.local:8123`.
    #[serde(default)]
    pub endereco: String,
    /// Token de acesso de longa duração, criado no perfil do usuário do Home Assistant.
    #[serde(default)]
    pub token: String,
}

impl HomeAssistant {
    pub fn configurado(&self) -> bool {
        !self.endereco.trim().is_empty() && !self.token.trim().is_empty()
    }

    /// Monta a chamada de um serviço, por exemplo `light.toggle` na entidade
    /// `light.sala`. É o formato que o Home Assistant documenta em
    /// `POST /api/services/<dominio>/<servico>`.
    pub fn chamada(&self, servico: &str, entidade: &str) -> Option<(String, String)> {
        if !self.configurado() {
            return None;
        }
        let (dominio, nome) = servico.split_once('.')?;
        let base = self.endereco.trim().trim_end_matches('/');
        let url = format!("{base}/api/services/{dominio}/{nome}");
        let corpo = if entidade.trim().is_empty() {
            "{}".to_string()
        } else {
            format!("{{\"entity_id\":{}}}", aspas(entidade.trim()))
        };
        Some((url, corpo))
    }
}

/// Domínios do Home Assistant que ligam e desligam com `toggle`.
///
/// A lista é curta de propósito: entra "todo dispositivo que liga e
/// desliga". Sensor e câmera não entram porque não há o que alternar neles.
pub const DOMINIOS_QUE_ALTERNAM: [&str; 8] = [
    "light",
    "switch",
    "fan",
    "input_boolean",
    "humidifier",
    "siren",
    "automation",
    "media_player",
];

/// Domínios que um pad sabe acionar, e para os quais existe serviço claro.
pub const DOMINIOS_ACIONAVEIS: [&str; 18] = [
    "light",
    "switch",
    "fan",
    "input_boolean",
    "humidifier",
    "siren",
    "automation",
    "media_player",
    "scene",
    "script",
    "button",
    "input_button",
    "lock",
    "cover",
    "vacuum",
    "climate",
    "remote",
    "valve",
];

/// Uma entidade do Home Assistant, no que interessa para virar pad.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entidade {
    /// Identificador, por exemplo `light.sala`.
    pub id: String,
    /// Nome amigável, o que a pessoa vê no Home Assistant.
    pub nome: String,
    /// A parte antes do ponto do `id`.
    pub dominio: String,
    /// Se está ligada agora.
    pub ligada: bool,
    /// Se o Home Assistant sabe o estado dela. Cena, script e botão não têm
    /// estado, e isso não é defeito.
    pub estado_conhecido: bool,
}

impl Entidade {
    /// O serviço que alterna esta entidade, por exemplo `light.toggle`.
    pub fn servico_de_alternar(&self) -> String {
        format!("{}.toggle", self.dominio)
    }

    /// O serviço que faz sentido para esta entidade, já pronto para o pad.
    ///
    /// Nem tudo alterna. Cena e script não têm estado para inverter: eles
    /// disparam. Botão se aperta. Deixar `toggle` para esses seria dar um pad
    /// que não funciona e não diz por quê.
    pub fn servico_sugerido(&self) -> String {
        let acao = match self.dominio.as_str() {
            // Não têm estado para inverter: disparam.
            "scene" | "script" => "turn_on",
            "button" | "input_button" => "press",
            "media_player" => "media_play_pause",
            "vacuum" => "start",
            _ => "toggle",
        };
        format!("{}.{acao}", self.dominio)
    }

    /// Se dá para fazer alguma coisa com esta entidade pelo pad.
    ///
    /// É lista de quem entra, não de quem sai. Tirar só o que informa deixava
    /// passar coisa demais: `update.toggle`, `conversation.toggle`,
    /// `tts.toggle`. São entidades de verdade, mas nenhuma delas responde a um
    /// pad, e oferecer isso é oferecer pad quebrado. Numa casa de teste eram 47
    /// entidades oferecidas, das quais 24 não faziam nada.
    pub fn tem_o_que_fazer(&self) -> bool {
        DOMINIOS_ACIONAVEIS.contains(&self.dominio.as_str())
    }
}

/// Lê tudo do Home Assistant e devolve o que dá para acionar por um pad.
///
/// Diferente de `listar_entidades`, que só traz o que liga e desliga: aqui
/// entram cena, script, botão e companhia, porque o seletor da interface deixa
/// a pessoa escolher qualquer coisa e não só interruptor.
pub fn listar_acionaveis(ha: &HomeAssistant) -> Result<Vec<Entidade>, String> {
    let corpo = buscar_estados(ha)?;
    let mut lista = todas_do_json(&corpo)?;
    lista.retain(|e| e.tem_o_que_fazer());
    Ok(lista)
}

/// Lê a lista de entidades do Home Assistant e devolve só as que alternam.
///
/// A resposta do `/api/states` costuma ter centenas de entidades, a maioria
/// sensor. Filtrar aqui evita despejar isso tudo na interface.
pub fn listar_entidades(ha: &HomeAssistant) -> Result<Vec<Entidade>, String> {
    entidades_do_json(&buscar_estados(ha)?)
}

/// Baixa o `/api/states` cru.
fn buscar_estados(ha: &HomeAssistant) -> Result<String, String> {
    if !ha.configurado() {
        return Err("falta o endereço ou o token do Home Assistant".into());
    }
    let base = ha.endereco.trim().trim_end_matches('/');
    let url = format!("{base}/api/states");
    let agente = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent();
    let mut resposta = agente
        .get(&url)
        .header("Authorization", &format!("Bearer {}", ha.token.trim()))
        .call()
        .map_err(|e| format!("nao consegui falar com o Home Assistant: {e}"))?;
    resposta
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("resposta ilegivel do Home Assistant: {e}"))
}

/// Separa o JSON do `/api/states`. Fica de fora da chamada de rede para o teste
/// rodar sem servidor nenhum.
pub fn entidades_do_json(corpo: &str) -> Result<Vec<Entidade>, String> {
    let mut todas = todas_do_json(corpo)?;
    // Aqui "unknown" também sai: numa página de liga e desliga, um pad que não
    // sabe o estado do que controla é um pad morto.
    todas.retain(|e| DOMINIOS_QUE_ALTERNAM.contains(&e.dominio.as_str()) && e.estado_conhecido);
    Ok(todas)
}

/// Separa o JSON do `/api/states` sem filtrar por domínio.
pub fn todas_do_json(corpo: &str) -> Result<Vec<Entidade>, String> {
    let bruto: serde_json::Value =
        serde_json::from_str(corpo).map_err(|e| format!("JSON invalido: {e}"))?;
    let lista = bruto
        .as_array()
        .ok_or("esperava uma lista de entidades")?;
    let mut saida: Vec<Entidade> = lista
        .iter()
        .filter_map(|e| {
            let id = e.get("entity_id")?.as_str()?.to_string();
            let (dominio, _) = id.split_once('.')?;
            let estado = e.get("state").and_then(|s| s.as_str()).unwrap_or("");
            // Fora do ar não vira pad. "unknown" fica: cena, script e botão
            // vivem nesse estado por natureza, porque não há nada para saber.
            if estado == "unavailable" {
                return None;
            }
            let nome = e
                .get("attributes")
                .and_then(|a| a.get("friendly_name"))
                .and_then(|n| n.as_str())
                .unwrap_or(&id)
                .to_string();
            Some(Entidade {
                dominio: dominio.to_string(),
                ligada: estado == "on" || estado == "playing",
                estado_conhecido: estado != "unknown" && !estado.is_empty(),
                id,
                nome,
            })
        })
        .collect();
    // Agrupa por dominio e depois por nome, para a pagina sair organizada.
    saida.sort_by(|a, b| {
        a.dominio
            .cmp(&b.dominio)
            .then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
    });
    Ok(saida)
}

/// Escapa uma string para caber dentro de JSON. Os valores aqui sao nomes de
/// entidade, mas nada impede alguem digitar uma aspa por engano.
fn aspas(valor: &str) -> String {
    serde_json::Value::String(valor.to_string()).to_string()
}

/// Manda a requisicao e devolve o codigo de resposta. Roda na thread de acoes,
/// entao pode bloquear.
pub fn chamar(
    metodo: Metodo,
    url: &str,
    cabecalhos: &BTreeMap<String, String>,
    corpo: Option<&str>,
) -> Result<u16, String> {
    let agente = ureq::Agent::config_builder()
        .timeout_global(Some(ESPERA))
        .build()
        .new_agent();

    // GET e os outros metodos tem tipos diferentes de construtor no ureq, entao
    // os dois caminhos ficam separados.
    let resposta = match metodo {
        Metodo::Get => {
            let mut req = agente.get(url);
            for (chave, valor) in cabecalhos {
                req = req.header(chave, valor);
            }
            req.call()
        }
        Metodo::Post | Metodo::Put => {
            let mut req = match metodo {
                Metodo::Put => agente.put(url),
                _ => agente.post(url),
            };
            for (chave, valor) in cabecalhos {
                req = req.header(chave, valor);
            }
            match corpo {
                Some(c) => req.header("Content-Type", "application/json").send(c),
                None => req.send_empty(),
            }
        }
    }
    .map_err(|e| e.to_string())?;

    Ok(resposta.status().as_u16())
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Um pedaco do que o /api/states devolve de verdade, com o que atrapalha:
    /// sensor, entidade indisponivel e uma sem nome amigavel.
    const EXEMPLO: &str = r#"[
        {"entity_id":"light.sala","state":"on","attributes":{"friendly_name":"Luz da sala"}},
        {"entity_id":"sensor.temperatura","state":"21.5","attributes":{"friendly_name":"Temperatura"}},
        {"entity_id":"switch.cafeteira","state":"off","attributes":{"friendly_name":"Cafeteira"}},
        {"entity_id":"light.quarto","state":"unavailable","attributes":{"friendly_name":"Luz do quarto"}},
        {"entity_id":"input_boolean.modo_foco","state":"off","attributes":{}},
        {"entity_id":"camera.porta","state":"idle","attributes":{"friendly_name":"Camera"}}
    ]"#;

    #[test]
    fn so_entram_os_dispositivos_que_ligam_e_desligam() {
        let e = entidades_do_json(EXEMPLO).unwrap();
        let ids: Vec<&str> = e.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["input_boolean.modo_foco", "light.sala", "switch.cafeteira"],
            "sensor, camera ou indisponivel entrou na lista"
        );
    }

    #[test]
    fn entidade_sem_nome_amigavel_usa_o_proprio_id() {
        let e = entidades_do_json(EXEMPLO).unwrap();
        let foco = e.iter().find(|x| x.dominio == "input_boolean").unwrap();
        assert_eq!(foco.nome, "input_boolean.modo_foco");
    }

    #[test]
    fn o_estado_ligado_vem_junto() {
        let e = entidades_do_json(EXEMPLO).unwrap();
        assert!(e.iter().find(|x| x.id == "light.sala").unwrap().ligada);
        assert!(!e.iter().find(|x| x.id == "switch.cafeteira").unwrap().ligada);
    }

    #[test]
    fn cada_entidade_sabe_o_servico_que_a_alterna() {
        let e = entidades_do_json(EXEMPLO).unwrap();
        let sala = e.iter().find(|x| x.id == "light.sala").unwrap();
        assert_eq!(sala.servico_de_alternar(), "light.toggle");
    }

    const VARIADO: &str = r#"[
        {"entity_id":"light.sala","state":"on","attributes":{"friendly_name":"Luz da sala"}},
        {"entity_id":"scene.cinema","state":"unknown","attributes":{"friendly_name":"Cinema"}},
        {"entity_id":"script.boa_noite","state":"off","attributes":{"friendly_name":"Boa noite"}},
        {"entity_id":"button.reiniciar","state":"unknown","attributes":{"friendly_name":"Reiniciar"}},
        {"entity_id":"media_player.tv","state":"playing","attributes":{"friendly_name":"TV"}},
        {"entity_id":"sensor.temperatura","state":"21","attributes":{"friendly_name":"Temperatura"}},
        {"entity_id":"weather.casa","state":"sunny","attributes":{}}
    ]"#;

    #[test]
    fn o_servico_sugerido_muda_com_o_tipo_da_entidade() {
        // Cena e script nao alternam, eles disparam. Botao se aperta. Dar
        // `toggle` para esses seria entregar um pad que nao funciona.
        let e = todas_do_json(VARIADO).unwrap();
        let acha = |id: &str| e.iter().find(|x| x.id == id).unwrap().servico_sugerido();
        assert_eq!(acha("light.sala"), "light.toggle");
        assert_eq!(acha("scene.cinema"), "scene.turn_on");
        assert_eq!(acha("script.boa_noite"), "script.turn_on");
        assert_eq!(acha("button.reiniciar"), "button.press");
        assert_eq!(acha("media_player.tv"), "media_player.media_play_pause");
    }

    #[test]
    fn o_que_so_informa_fica_fora_da_lista() {
        let e = todas_do_json(VARIADO).unwrap();
        assert!(e.iter().any(|x| x.id == "sensor.temperatura"), "todas traz tudo");
        let acionaveis: Vec<&str> = e
            .iter()
            .filter(|x| x.tem_o_que_fazer())
            .map(|x| x.id.as_str())
            .collect();
        assert!(!acionaveis.contains(&"sensor.temperatura"), "{acionaveis:?}");
        assert!(!acionaveis.contains(&"weather.casa"), "{acionaveis:?}");
        assert_eq!(acionaveis.len(), 5);
    }

    #[test]
    fn a_lista_que_alterna_continua_menor_que_a_lista_toda() {
        let todas = todas_do_json(VARIADO).unwrap();
        let alternam = entidades_do_json(VARIADO).unwrap();
        assert!(alternam.len() < todas.len());
        assert!(alternam.iter().all(|e| DOMINIOS_QUE_ALTERNAM.contains(&e.dominio.as_str())));
    }

    #[test]
    fn nao_oferece_entidade_que_o_pad_nao_aciona() {
        // Numa casa de verdade a maioria do que sobrava era atualizacao de
        // add-on e servico de voz: entidade legitima, pad quebrado.
        let json = r#"[
            {"entity_id":"update.addon","state":"off","attributes":{}},
            {"entity_id":"conversation.ha","state":"unknown","attributes":{}},
            {"entity_id":"tts.google","state":"unknown","attributes":{}},
            {"entity_id":"stt.cloud","state":"unknown","attributes":{}},
            {"entity_id":"notify.iphone","state":"unknown","attributes":{}},
            {"entity_id":"camera.porta","state":"idle","attributes":{}},
            {"entity_id":"light.sala","state":"on","attributes":{}}
        ]"#;
        let acionaveis: Vec<&str> = todas_do_json(json)
            .unwrap()
            .iter()
            .filter(|e| e.tem_o_que_fazer())
            .map(|e| e.dominio.clone())
            .collect::<Vec<String>>()
            .leak()
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(acionaveis, vec!["light"], "{acionaveis:?}");
    }

    #[test]
    fn todo_dominio_acionavel_tem_um_servico_que_existe() {
        // Guarda de regressao: dominio na lista sem servico proprio cairia em
        // `toggle`, e nem todo dominio tem toggle.
        for dominio in DOMINIOS_ACIONAVEIS {
            let e = Entidade {
                id: format!("{dominio}.x"),
                nome: "x".into(),
                dominio: dominio.to_string(),
                ligada: false,
                estado_conhecido: true,
            };
            let s = e.servico_sugerido();
            assert!(s.starts_with(dominio), "{s}");
            assert!(s.split_once('.').is_some_and(|(_, a)| !a.is_empty()));
        }
    }

    #[test]
    fn json_torto_da_erro_em_vez_de_estourar() {
        assert!(entidades_do_json("isto nao e json").is_err());
        assert!(entidades_do_json(r#"{"nao":"e lista"}"#).is_err());
    }

    #[test]
    fn sem_token_nem_tenta_falar_com_a_rede() {
        let ha = HomeAssistant {
            endereco: "http://192.0.2.1:8123".into(),
            token: "".into(),
        };
        let e = listar_entidades(&ha).unwrap_err();
        assert!(e.contains("token"), "{e}");
    }

    #[test]
    fn sem_endereco_ou_token_nao_monta_chamada() {
        let vazio = HomeAssistant::default();
        assert!(!vazio.configurado());
        assert_eq!(vazio.chamada("light.toggle", "light.sala"), None);
    }

    #[test]
    fn monta_a_url_e_o_corpo_do_servico() {
        let ha = HomeAssistant {
            endereco: "http://casa.local:8123/".into(),
            token: "abc".into(),
        };
        let (url, corpo) = ha.chamada("light.toggle", "light.sala").unwrap();
        assert_eq!(url, "http://casa.local:8123/api/services/light/toggle");
        assert_eq!(corpo, "{\"entity_id\":\"light.sala\"}");
    }

    #[test]
    fn servico_sem_ponto_nao_e_valido() {
        let ha = HomeAssistant {
            endereco: "http://casa.local:8123".into(),
            token: "abc".into(),
        };
        assert_eq!(ha.chamada("toggle", "light.sala"), None);
    }

    #[test]
    fn sem_entidade_manda_corpo_vazio() {
        let ha = HomeAssistant {
            endereco: "http://casa.local:8123".into(),
            token: "abc".into(),
        };
        let (_, corpo) = ha.chamada("script.boa_noite", "  ").unwrap();
        assert_eq!(corpo, "{}");
    }

    #[test]
    fn aspas_dentro_do_nome_nao_quebram_o_json() {
        let entrada = "luz \"da\" sala";
        let saida = aspas(entrada);
        // Ler de volta tem que devolver exatamente a mesma string.
        let devolta: String = serde_json::from_str(&saida).unwrap();
        assert_eq!(devolta, entrada);
    }
}
