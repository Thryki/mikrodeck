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
/// A lista é curta de propósito: o Davi pediu "todos os dispositivos que liga e
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
}

impl Entidade {
    /// O serviço que alterna esta entidade, por exemplo `light.toggle`.
    pub fn servico_de_alternar(&self) -> String {
        format!("{}.toggle", self.dominio)
    }
}

/// Lê a lista de entidades do Home Assistant e devolve só as que alternam.
///
/// A resposta do `/api/states` costuma ter centenas de entidades, a maioria
/// sensor. Filtrar aqui evita despejar isso tudo na interface.
pub fn listar_entidades(ha: &HomeAssistant) -> Result<Vec<Entidade>, String> {
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
    let corpo = resposta
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("resposta ilegivel do Home Assistant: {e}"))?;
    entidades_do_json(&corpo)
}

/// Separa o JSON do `/api/states`. Fica de fora da chamada de rede para o teste
/// rodar sem servidor nenhum.
pub fn entidades_do_json(corpo: &str) -> Result<Vec<Entidade>, String> {
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
            if !DOMINIOS_QUE_ALTERNAM.contains(&dominio) {
                return None;
            }
            let estado = e.get("state").and_then(|s| s.as_str()).unwrap_or("");
            // "unavailable" e "unknown" viram pad morto: melhor nao oferecer.
            if estado == "unavailable" || estado == "unknown" {
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
