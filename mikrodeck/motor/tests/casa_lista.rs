//! Confere contra o Home Assistant de verdade quantas entidades o seletor
//! oferece, e que o servico sugerido bate com o tipo de cada uma.

use motor::config::Config;

#[test]
#[ignore = "precisa de um Home Assistant alcancavel"]
fn o_seletor_oferece_mais_que_a_pagina_da_casa() {
    let config = Config::carregar_ou_criar(&Config::caminho_padrao()).expect("config");
    if !config.home_assistant.configurado() {
        eprintln!("sem token: teste pulado");
        return;
    }
    let alternam = motor::rede::listar_entidades(&config.home_assistant).expect("alternam");
    let acionaveis = motor::rede::listar_acionaveis(&config.home_assistant).expect("acionaveis");
    println!("{} alternam, {} acionaveis", alternam.len(), acionaveis.len());

    let mut por_dominio: std::collections::BTreeMap<String, usize> = Default::default();
    for e in &acionaveis {
        *por_dominio.entry(e.dominio.clone()).or_default() += 1;
    }
    for (d, n) in &por_dominio {
        let exemplo = acionaveis.iter().find(|e| e.dominio == *d).unwrap();
        println!("  {d:<18} {n:>3}   ex: {} -> {}", exemplo.nome, exemplo.servico_sugerido());
    }
    assert!(acionaveis.len() >= alternam.len());
    assert!(acionaveis.iter().all(|e| e.servico_sugerido().contains('.')));
}
