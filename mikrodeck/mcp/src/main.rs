//! Servidor MCP do MikroDeck.
//!
//! Deixa qualquer modelo configurar os pads em linguagem natural. Ele fala
//! JSON-RPC pelo stdin e stdout, e a unica coisa que toca e o arquivo
//! `~/.mikrodeck/config.json`. O MikroDeck ve o arquivo mudar e aplica sozinho.
//!
//! Para ligar no Claude Code:
//!
//! ```text
//! claude mcp add mikrodeck -- <caminho>/mikrodeck-mcp.exe
//! ```

mod ferramentas;
mod jsonrpc;

use jsonrpc::{ler_pedidos, responder, responder_erro};
use serde_json::{json, Value};
use std::io::{stdin, stdout, BufReader};

/// Versao do protocolo que este servidor fala.
const VERSAO_MCP: &str = "2024-11-05";

fn main() {
    let entrada = BufReader::new(stdin());
    let mut saida = stdout();

    for pedido in ler_pedidos(entrada) {
        // Sem id e notificacao: o protocolo manda nao responder.
        let Some(id) = pedido.id.clone() else {
            continue;
        };

        match pedido.metodo.as_str() {
            "initialize" => responder(
                &mut saida,
                id,
                json!({
                    "protocolVersion": VERSAO_MCP,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "mikrodeck", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": ferramentas::INSTRUCOES,
                }),
            ),
            "tools/list" => responder(
                &mut saida,
                id,
                json!({"tools": ferramentas::catalogo()}),
            ),
            "tools/call" => {
                let nome = pedido
                    .parametros
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let argumentos = pedido
                    .parametros
                    .get("arguments")
                    .cloned()
                    .unwrap_or(json!({}));
                responder(&mut saida, id, resultado(&nome, &argumentos));
            }
            "ping" => responder(&mut saida, id, json!({})),
            outro => responder_erro(
                &mut saida,
                id,
                -32601,
                &format!("metodo nao suportado: {outro}"),
            ),
        }
    }
}

/// Roda a ferramenta e embrulha no formato de resposta do MCP. Erro de ferramenta
/// vira `isError`, e nao erro de protocolo: o modelo precisa ler e corrigir.
fn resultado(nome: &str, argumentos: &Value) -> Value {
    match ferramentas::executar(nome, argumentos) {
        Ok(texto) => json!({"content": [{"type": "text", "text": texto}]}),
        Err(erro) => json!({
            "content": [{"type": "text", "text": erro}],
            "isError": true
        }),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn erro_de_ferramenta_vira_is_error_e_nao_derruba_a_chamada() {
        let r = resultado("voar", &json!({}));
        assert_eq!(r["isError"], json!(true));
        assert!(r["content"][0]["text"].as_str().unwrap().contains("voar"));
    }

    #[test]
    fn sucesso_vem_como_texto_sem_is_error() {
        let r = resultado("listar_cores", &json!({}));
        assert!(r.get("isError").is_none());
        assert!(r["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("vermelho"));
    }
}
