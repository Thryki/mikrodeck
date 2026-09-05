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
