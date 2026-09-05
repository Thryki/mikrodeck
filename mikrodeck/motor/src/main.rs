//! MikroDeck — motor.
//!
//! Transforma o Maschine Mikro MK3 num Stream Deck: cada pad tem cor e ação,
//! as páginas trocam pelos botões de seta, e nada disso depende do software
//! da Native Instruments rodando.
//!
//! Config em `~/.mikrodeck/config.json`. Se não existir, um exemplo é criado.

use motor::acoes::Executor;
use motor::config::Config;
use motor::estado::{Estado, Reacao};
use motor::hid::{Aparelho, Evento};

fn main() {
    println!("MikroDeck — motor\n");

    let caminho = Config::caminho_padrao();
    let config = match Config::carregar_ou_criar(&caminho) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Não consegui ler a config em {}: {e}", caminho.display());
            std::process::exit(1);
        }
    };
    println!("Config: {}", caminho.display());

    let (aparelho, eventos) = match Aparelho::abrir(30) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("\n{e}");
            std::process::exit(1);
        }
    };

    let mut estado = Estado::novo(config);
    let executor = Executor::novo();

    println!(
        "Aparelho conectado. Página {}/{}: {}",
        estado.numero_pagina(),
        estado.total_paginas(),
        estado.nome_pagina()
    );
    mostrar_pagina(&estado);
    println!("\nSetas trocam de página. SHIFT + STOP encerra (ou Ctrl+C aqui).\n");

    repintar(&aparelho, &estado);

    // SHIFT sozinho não encerra: é um modificador, e o usuário aperta sem querer.
    // O combo é SHIFT segurado mais STOP.
    let mut shift_segurado = false;

    for evento in eventos {
        if let Evento::Botao { nome: "shift", apertado } = evento {
            shift_segurado = apertado;
        }
        if shift_segurado
            && matches!(
                evento,
                Evento::Botao {
                    nome: "stop",
                    apertado: true
                }
            )
        {
            println!("\nEncerrando.");
            aparelho.pintar(|f| f.limpar());
            std::thread::sleep(std::time::Duration::from_millis(100));
            return;
        }

        if evento == Evento::Desconectado {
            println!("\nAparelho desconectado.");
            return;
        }

        match estado.processar(&evento) {
            Reacao::Executar(acao) => {
                if let Evento::PadApertado { pad, .. } = evento {
                    println!("pad {pad}: {acao:?}");
                }
                executor.disparar(acao);
            }
            Reacao::PaginaMudou { numero, nome } => {
                println!("\npágina {}/{}: {}", numero, estado.total_paginas(), nome);
                mostrar_pagina(&estado);
            }
            Reacao::Pausado(pausado) => {
                println!("
{}", if pausado { "pausado" } else { "ativo" });
            }
            Reacao::Nada => {}
        }

        // Repinta sempre: o frame só é mandado ao aparelho se algum byte mudou.
        repintar(&aparelho, &estado);
    }
}

fn repintar(aparelho: &Aparelho, estado: &Estado) {
    aparelho.pintar(|f| estado.pintar(f));
}

/// Lista os pads configurados da página atual, para o usuário saber o que tem onde.
fn mostrar_pagina(estado: &Estado) {
    let Some(pagina) = estado.config().paginas.get(estado.numero_pagina() - 1) else {
        return;
    };
    if pagina.pads.is_empty() {
        println!("  (nenhum pad configurado nesta página)");
        return;
    }
    for (pad, controle) in &pagina.pads {
        println!("  pad {pad}: {}", controle.nome);
    }
}
