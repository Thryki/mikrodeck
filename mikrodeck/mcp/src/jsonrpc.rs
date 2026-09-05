//! O mínimo de JSON-RPC 2.0 sobre stdio que o MCP precisa.
//!
//! O protocolo vive no stdout, então nada mais pode escrever lá. Log é stderr.

use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// Um pedido que chegou. `id` ausente quer dizer notificação: não se responde.
pub struct Pedido {
    pub metodo: String,
    pub id: Option<Value>,
    pub parametros: Value,
}

/// Lê pedidos do stdin, uma linha por mensagem.
pub fn ler_pedidos<R: BufRead>(entrada: R) -> impl Iterator<Item = Pedido> {
    entrada.lines().filter_map(|linha| {
        let linha = linha.ok()?;
        if linha.trim().is_empty() {
            return None;
        }
        let v: Value = match serde_json::from_str(&linha) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("linha inválida: {e}");
                return None;
            }
        };
        Some(Pedido {
            metodo: v.get("method")?.as_str()?.to_string(),
            id: v.get("id").cloned(),
            parametros: v.get("params").cloned().unwrap_or(json!({})),
        })
    })
}

pub fn responder<W: Write>(saida: &mut W, id: Value, resultado: Value) {
    escrever(saida, json!({"jsonrpc": "2.0", "id": id, "result": resultado}));
}

pub fn responder_erro<W: Write>(saida: &mut W, id: Value, codigo: i32, mensagem: &str) {
    escrever(
        saida,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": codigo, "message": mensagem}
        }),
    );
}

fn escrever<W: Write>(saida: &mut W, valor: Value) {
    let _ = writeln!(saida, "{valor}");
    let _ = saida.flush();
}
