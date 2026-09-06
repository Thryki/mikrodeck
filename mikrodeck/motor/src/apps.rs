//! A lista de programas instalados, para escolher pelo nome em vez de caçar o
//! executável no disco.
//!
//! Procurar `Raycast.exe` na mão é difícil de propósito: apps da Microsoft
//! Store ficam em `C:\Program Files\WindowsApps`, uma pasta que o Windows
//! esconde e protege, e o executável de lá nem sempre abre direto. O jeito
//! certo é abrir pelo identificador do app, que é o que o próprio menu Iniciar
//! usa.

use serde::Serialize;

/// Um programa que o menu Iniciar conhece.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct App {
    /// O nome como aparece no menu Iniciar.
    pub nome: String,
    /// O que o MikroDeck guarda para abrir depois: sempre um
    /// `shell:appsFolder\<identificador>`, que é como o próprio menu Iniciar
    /// abre as coisas.
    pub caminho: String,
    /// Se veio da Microsoft Store. A interface mostra para a pessoa entender
    /// por que o caminho é estranho.
    pub da_loja: bool,
}

/// Prefixo que manda o Windows abrir pelo identificador do app.
pub const PREFIXO_LOJA: &str = r"shell:appsFolder\";

/// Se este caminho é um app do menu Iniciar em vez de um arquivo.
pub fn e_da_loja(caminho: &str) -> bool {
    caminho.trim_start().starts_with("shell:")
}

/// Lê a lista do menu Iniciar, do jeito que o próprio Windows a enxerga.
///
/// Usa o `Get-StartApps` do PowerShell, que devolve nome e identificador de
/// tudo: programa comum e app da Store, no mesmo formato. Varrer as pastas de
/// atalhos na mão deixaria os apps da Store de fora, que são justamente os
/// difíceis de achar.
pub fn listar() -> Result<Vec<App>, String> {
    let saida = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            // A primeira linha força UTF-8 na saída: sem ela o console
            // devolve a página de código antiga e "Configurações" chega
            // quebrado.
            //
            // O separador é uma barra vertical dupla: nome de app tem de tudo,
            // menos isso.
            "[Console]::OutputEncoding=[Text.Encoding]::UTF8;              Get-StartApps | ForEach-Object { \"$($_.Name)||$($_.AppID)\" }",
        ])
        .output()
        .map_err(|e| format!("nao consegui listar os programas: {e}"))?;
    if !saida.status.success() {
        return Err("o Windows nao devolveu a lista de programas".into());
    }
    Ok(separar(&String::from_utf8_lossy(&saida.stdout)))
}

/// Separa a saída do `Get-StartApps`. Fora da chamada para poder ser testada
/// sem depender do que está instalado na máquina.
pub fn separar(texto: &str) -> Vec<App> {
    let mut apps: Vec<App> = texto
        .lines()
        .filter_map(|linha| {
            let (nome, id) = linha.trim().split_once("||")?;
            let nome = nome.trim();
            let id = id.trim();
            if nome.is_empty() || id.is_empty() {
                return None;
            }
            // Todo item do menu Iniciar abre pelo identificador de app,
            // inclusive programa comum: o `Get-StartApps` devolve identificador,
            // nao caminho de arquivo. O Chrome, por exemplo, vem como "Chrome",
            // que nao e caminho nenhum e nao abriria de outro jeito.
            Some(App {
                nome: nome.to_string(),
                caminho: format!("{PREFIXO_LOJA}{id}"),
                // O "!" separa pacote de aplicativo: e o que identifica app da
                // Store. Serve para a interface explicar o caminho estranho.
                da_loja: id.contains('!'),
            })
        })
        .collect();
    apps.sort_by(|a, b| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()));
    apps.dedup_by(|a, b| a.caminho == b.caminho);
    apps
}

/// As pistas de nome de processo deste caminho. Reexporta o que o `vigias`
/// calcula, para quem só conhece o módulo de apps.
pub fn pistas_publicas(caminho: &str) -> Vec<String> {
    crate::vigias::pistas_de_processo(caminho)
}

#[cfg(test)]
mod testes {
    use super::*;

    const EXEMPLO: &str = "Raycast||Raycast.Raycast_qypenmj9wpt2a!Raycast\n\
                           Bloco de Notas||C:\\Windows\\system32\\notepad.exe\n\
                           \n\
                           Spotify||Spotify.exe\n\
                           ||sem nome\n\
                           Sem id||\n";

    #[test]
    fn o_app_da_loja_vira_um_caminho_que_o_windows_abre() {
        let apps = separar(EXEMPLO);
        let raycast = apps.iter().find(|a| a.nome == "Raycast").unwrap();
        assert!(raycast.da_loja);
        assert_eq!(
            raycast.caminho,
            r"shell:appsFolder\Raycast.Raycast_qypenmj9wpt2a!Raycast"
        );
        assert!(e_da_loja(&raycast.caminho));
    }

    #[test]
    fn programa_comum_tambem_abre_pelo_menu_iniciar() {
        // O Get-StartApps devolve identificador de app, nao caminho de arquivo:
        // o Chrome vem como "Chrome", que nao abriria de outro jeito.
        let apps = separar(EXEMPLO);
        let bloco = apps.iter().find(|a| a.nome == "Bloco de Notas").unwrap();
        assert!(!bloco.da_loja, "nao e da Store");
        assert!(e_da_loja(&bloco.caminho), "mas abre pelo menu Iniciar");
        assert!(bloco.caminho.ends_with("notepad.exe"));
    }

    #[test]
    fn linha_torta_e_pulada_em_vez_de_estourar() {
        let apps = separar(EXEMPLO);
        assert_eq!(apps.len(), 3, "{apps:?}");
        assert!(apps.iter().all(|a| !a.nome.is_empty() && !a.caminho.is_empty()));
        assert!(separar("").is_empty());
        assert!(separar("linha sem separador nenhum").is_empty());
    }

    #[test]
    fn a_lista_vem_em_ordem_de_nome() {
        let nomes: Vec<String> = separar(EXEMPLO).into_iter().map(|a| a.nome).collect();
        assert_eq!(nomes, vec!["Bloco de Notas", "Raycast", "Spotify"]);
    }

    #[test]
    fn a_lista_de_verdade_traz_alguma_coisa() {
        // Numa maquina Windows o menu Iniciar nunca esta vazio.
        match listar() {
            Ok(apps) => {
                assert!(!apps.is_empty(), "o menu Iniciar veio vazio");
                assert!(apps.iter().all(|a| !a.caminho.trim().is_empty()));
            }
            Err(e) => eprintln!("sem lista de programas nesta maquina: {e}"),
        }
    }
}
