//! Fala com o Home Assistant de verdade, usando o endereço e o token que estão
//! na config. Marcado como `ignore`: sem servidor na rede ele não tem o que
//! testar, e o teste normal não pode depender da casa de ninguém estar de pé.
//!
//! Rodar com:
//! `cargo test --test casa_de_verdade -- --ignored --nocapture`

use motor::config::Config;

#[test]
#[ignore = "precisa de um Home Assistant alcancavel e de token na config"]
fn lista_o_que_a_casa_tem() {
    let config = Config::carregar_ou_criar(&Config::caminho_padrao()).expect("config");
    if !config.home_assistant.configurado() {
        eprintln!("sem endereco ou token na config: nada a fazer");
        return;
    }
    let entidades = motor::rede::listar_entidades(&config.home_assistant).expect("listar");
    println!("{} dispositivos que ligam e desligam:", entidades.len());
    for e in &entidades {
        println!(
            "  {:<34} {:<8} {}",
            e.nome,
            if e.ligada { "ligado" } else { "desligado" },
            e.id
        );
    }
    let paginas = motor::prontas::paginas_da_casa(&entidades);
    println!("\n{} pagina(s):", paginas.len());
    for p in &paginas {
        println!("  {} com {} pads", p.nome, p.pads.len());
    }
    assert!(!entidades.is_empty(), "a casa respondeu vazia");
}

#[test]
#[ignore = "escreve na config de verdade"]
fn adiciona_as_paginas_da_casa_na_config() {
    let caminho = Config::caminho_padrao();
    let mut config = Config::carregar_ou_criar(&caminho).expect("config");
    let entidades = motor::rede::listar_entidades(&config.home_assistant).expect("listar");
    let novas = motor::prontas::paginas_da_casa(&entidades);
    for mut pagina in novas {
        // Não sobrescreve: uma página com o nome já usado ganha um número.
        let mut nome = pagina.nome.clone();
        let mut n = 2;
        while config.paginas.iter().any(|p| p.nome == nome) {
            nome = format!("{} {n}", pagina.nome);
            n += 1;
        }
        pagina.nome = nome;
        println!("adicionando {} com {} pads", pagina.nome, pagina.pads.len());
        config.paginas.push(pagina);
    }
    config.salvar(&caminho).expect("salvar");
    println!("paginas agora: {}", config.paginas.len());
}
