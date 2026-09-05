//! A configuração do MikroDeck: páginas, pads, cores e ações.
//!
//! Fica em `~/.mikrodeck/config.json`. O campo `versao` existe para permitir
//! migração de formato mais adiante sem quebrar a config de quem já usa.

use crate::acoes::Acao;
use crate::hid::Cor;
use crate::rede::HomeAssistant;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const VERSAO_ATUAL: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub versao: u32,
    pub paginas: Vec<Pagina>,
    /// Botão físico que avança de página. Nome da tabela `hid::protocolo::BOTOES`.
    #[serde(default = "botao_proxima_padrao")]
    pub botao_proxima_pagina: String,
    /// Botão físico que volta de página.
    #[serde(default = "botao_anterior_padrao")]
    pub botao_pagina_anterior: String,
    /// Brilho dos pads em repouso, de 0 a 3.
    #[serde(default = "brilho_padrao")]
    pub brilho: u8,
    /// O que a touch strip controla.
    #[serde(default)]
    pub strip: FuncaoStrip,
    /// Onde o dedo chega de verdade na touch strip. O sensor vai de 0 a 255, mas
    /// nem toda ponta é alcançável, então sem calibrar não dá para chegar a 100%.
    #[serde(default)]
    pub calibracao_strip: CalibracaoStrip,
    /// O que girar o knob faz.
    #[serde(default)]
    pub knob: FuncaoKnob,
    /// Ligação com o Home Assistant, usada pelas ações de automação da casa.
    #[serde(default)]
    pub home_assistant: HomeAssistant,
    /// Texto que corre na tela quando o aparelho fica parado.
    #[serde(default)]
    pub descanso: Descanso,
    /// O que a luz faz ao soltar um pad.
    #[serde(default)]
    pub ao_apertar: AoApertar,
}

/// Descanso de tela: depois de um tempo sem ninguém tocar no aparelho, a tela
/// passa a rodar um texto em vez de ficar com o nome da página parado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Descanso {
    pub ativo: bool,
    /// Texto que corre na tela. Vazio deixa a tela na página; só a luz anima.
    pub texto: String,
    /// Quanto tempo parado até começar.
    pub segundos: u64,
    /// A luz dos pads enquanto dorme.
    #[serde(default)]
    pub luz: LuzDescanso,
}

impl Default for Descanso {
    fn default() -> Self {
        Self {
            ativo: true,
            texto: "MikroDeck".into(),
            segundos: 90,
            luz: LuzDescanso::default(),
        }
    }
}

/// A luz dos pads no descanso. O teto de brilho é sempre o brilho geral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LuzDescanso {
    #[serde(default)]
    pub modo: ModoLuz,
    #[serde(default)]
    pub ritmo: Ritmo,
    #[serde(default)]
    pub cor: CorLuz,
}

/// O que os pads fazem no descanso. Desenho em `docs/animacoes.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoLuz {
    /// Nada muda nos pads.
    Nenhuma,
    /// A página inteira sobe e desce de brilho. É o padrão.
    #[default]
    Respiracao,
    /// Um cometa percorre a borda e fecha no centro.
    Contorno,
    /// Uma coluna varre da esquerda para a direita e volta.
    Colunas,
    /// Uma gota no centro se espalha para a borda e some.
    Pulso,
    /// Medidor estéreo da saída de áudio.
    Som,
}

/// Velocidade da luz. Ignorado no modo Som.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ritmo {
    Lento,
    #[default]
    Medio,
    Rapido,
}

/// Cor da luz: automática (anda na roda, ou a cor de cada pad na Respiração)
/// ou uma cor fixa da tabela. No JSON é `"auto"` ou o nome da cor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CorLuz {
    #[default]
    Auto,
    Fixa(Cor),
}

impl Serialize for CorLuz {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            CorLuz::Auto => s.serialize_str("auto"),
            CorLuz::Fixa(c) => s.serialize_str(nome_da_cor(*c)),
        }
    }
}

impl<'de> Deserialize<'de> for CorLuz {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let nome = String::deserialize(d)?;
        if nome == "auto" {
            return Ok(CorLuz::Auto);
        }
        cor_do_nome(&nome)
            .filter(|c| *c != Cor::Apagado)
            .map(CorLuz::Fixa)
            .ok_or_else(|| serde::de::Error::custom(format!("cor de luz desconhecida: {nome}")))
    }
}

/// O que a luz faz quando um pad é solto, fora do descanso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AoApertar {
    Nenhuma,
    /// O pad volta em brilho 3 e pousa no de repouso em 200 ms.
    #[default]
    Eco,
}

/// O que girar o knob faz. Ele não dá posição, dá passos para um lado ou para o
/// outro, então serve para o que é incremental.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuncaoKnob {
    /// Girar não faz nada.
    Nenhuma,
    /// Volume principal do Windows, em passos de 2%.
    Volume,
    /// Brilho geral dos pads, em passos de 1.
    BrilhoPads,
    /// Passa de página, como as setas. É o padrão: o knob fica bem à mão e a
    /// troca de página é o que mais se usa.
    #[default]
    Paginas,
    /// Rola a janela que estiver na frente, como a roda do mouse.
    Rolagem,
}

/// A faixa de posições que o dedo realmente alcança na touch strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibracaoStrip {
    pub minimo: u8,
    pub maximo: u8,
}

impl Default for CalibracaoStrip {
    fn default() -> Self {
        Self {
            minimo: 0,
            maximo: 255,
        }
    }
}

impl CalibracaoStrip {
    /// Converte a posição crua em fração de 0 a 1, esticando a faixa alcançável
    /// para o intervalo inteiro. Faixa invertida ou degenerada cai no cru, para
    /// uma calibração ruim não travar a strip.
    pub fn fracao(&self, posicao: u8) -> f32 {
        if self.maximo <= self.minimo {
            return posicao as f32 / 255.0;
        }
        let baixo = self.minimo as f32;
        let alto = self.maximo as f32;
        ((posicao as f32 - baixo) / (alto - baixo)).clamp(0.0, 1.0)
    }
}

/// O que o dedo na touch strip faz. Ela dá uma posição de 0 a 255, então serve
/// bem para qualquer coisa contínua.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuncaoStrip {
    /// Só acende acompanhando o dedo, sem controlar nada.
    Nenhuma,
    /// Volume principal do Windows. É o padrão: uma faixa que não faz nada é um
    /// controle desperdiçado, e volume é o uso óbvio.
    #[default]
    Volume,
    /// Brilho geral dos pads.
    BrilhoPads,
    /// Escolhe a página pela posição do dedo.
    Paginas,
}

fn botao_proxima_padrao() -> String {
    "seta_direita".into()
}
fn botao_anterior_padrao() -> String {
    "seta_esquerda".into()
}
fn brilho_padrao() -> u8 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagina {
    pub nome: String,
    /// Chave é o número do pad impresso no aparelho, de 1 a 16.
    /// Pads que não aparecem aqui ficam apagados e sem ação.
    #[serde(default)]
    pub pads: BTreeMap<u8, Controle>,
    /// Botões físicos programados nesta página. A chave é o nome da tabela
    /// `hid::protocolo::BOTOES`, por exemplo "mute" ou "solo".
    /// Os botões são monocromáticos: a cor do controle é ignorada, só o brilho conta.
    #[serde(default)]
    pub botoes: BTreeMap<String, Controle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Controle {
    pub nome: String,
    #[serde(default = "acao_padrao")]
    pub acao: Acao,
    /// Cor do pad em repouso.
    #[serde(default = "cor_padrao", with = "cor_serde")]
    pub cor: Cor,
    /// Cor enquanto o pad está apertado. Se ausente, usa branco.
    #[serde(default, with = "cor_opcional_serde")]
    pub cor_pressionado: Option<Cor>,
    /// Cor enquanto o programa deste pad está aberto. Só vale para a ação de
    /// abrir programa; nas outras não há como saber se está "ligado".
    #[serde(default, with = "cor_opcional_serde")]
    pub cor_aberto: Option<Cor>,
    /// Brilho só deste pad, de 0 a 3. Ausente significa usar o brilho geral.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brilho: Option<u8>,
    /// Cuidar da janela do programa: apertar de novo traz para a frente ou
    /// minimiza, segurar fecha. Só faz sentido com a ação de abrir programa, e
    /// muda o momento em que o pad age: passa a agir quando você solta.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gerenciar_janela: bool,
}

fn acao_padrao() -> Acao {
    Acao::Nenhuma
}
fn cor_padrao() -> Cor {
    Cor::Azul
}

impl Config {
    /// Caminho padrão do arquivo de configuração.
    pub fn caminho_padrao() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mikrodeck")
            .join("config.json")
    }

    /// Carrega a config do disco. Se o arquivo não existir, cria um de exemplo
    /// e devolve ele, para o primeiro uso já ter algo funcionando.
    pub fn carregar_ou_criar(caminho: &PathBuf) -> std::io::Result<Self> {
        if caminho.exists() {
            let texto = std::fs::read_to_string(caminho)?;
            let config: Config = serde_json::from_str(&texto)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(config)
        } else {
            let config = Config::exemplo();
            config.salvar(caminho)?;
            println!("Config de exemplo criada em {}", caminho.display());
            Ok(config)
        }
    }

    /// Grava a config sem risco de deixar o arquivo pela metade.
    ///
    /// Escreve num arquivo temporário e só então troca pelo definitivo. Sem isso,
    /// um `write` interrompido no meio, ou a interface e o servidor MCP gravando
    /// ao mesmo tempo, deixam um JSON quebrado que ninguém mais consegue ler.
    pub fn salvar(&self, caminho: &PathBuf) -> std::io::Result<()> {
        if let Some(pasta) = caminho.parent() {
            std::fs::create_dir_all(pasta)?;
        }
        let texto = serde_json::to_string_pretty(self)?;
        let temporario = caminho.with_extension("json.tmp");
        std::fs::write(&temporario, texto)?;
        // No Windows o rename por cima de arquivo existente falha, então tira o
        // antigo antes. A janela entre remover e renomear é de microssegundos, e
        // quem lê nesse instante só encontra "arquivo não existe", que é tratado.
        match std::fs::rename(&temporario, caminho) {
            Ok(()) => Ok(()),
            Err(_) => {
                let _ = std::fs::remove_file(caminho);
                std::fs::rename(&temporario, caminho)
            }
        }
    }

    /// Quantas páginas existem. Sempre pelo menos uma.
    pub fn total_paginas(&self) -> usize {
        self.paginas.len().max(1)
    }

    /// Config de exemplo, com duas páginas, para o primeiro uso.
    pub fn exemplo() -> Self {
        use crate::acoes::TeclaMidia;

        let mut apps = BTreeMap::new();
        apps.insert(
            13,
            Controle {
                nome: "Chrome".into(),
                acao: Acao::AbrirPrograma {
                    caminho: "chrome".into(),
                    argumentos: vec![],
                },
                cor: Cor::Azul,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        apps.insert(
            14,
            Controle {
                nome: "Explorador".into(),
                acao: Acao::AbrirPrograma {
                    caminho: "explorer".into(),
                    argumentos: vec![],
                },
                cor: Cor::AmareloQuente,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        apps.insert(
            15,
            Controle {
                nome: "Bloco de notas".into(),
                acao: Acao::AbrirPrograma {
                    caminho: "notepad".into(),
                    argumentos: vec![],
                },
                cor: Cor::Verde,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        apps.insert(
            16,
            Controle {
                nome: "Terminal".into(),
                acao: Acao::AbrirPrograma {
                    caminho: "wt".into(),
                    argumentos: vec![],
                },
                cor: Cor::Turquesa,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        apps.insert(
            9,
            Controle {
                nome: "Claude".into(),
                acao: Acao::AbrirUrl {
                    url: "https://claude.ai".into(),
                },
                cor: Cor::Laranja,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        apps.insert(
            1,
            Controle {
                nome: "Copiar".into(),
                acao: Acao::Atalho {
                    teclas: "ctrl+c".into(),
                },
                cor: Cor::Violeta,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        apps.insert(
            2,
            Controle {
                nome: "Colar".into(),
                acao: Acao::Atalho {
                    teclas: "ctrl+v".into(),
                },
                cor: Cor::Violeta,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );

        let mut midia = BTreeMap::new();
        midia.insert(
            13,
            Controle {
                nome: "Tocar e pausar".into(),
                acao: Acao::Midia {
                    tecla: TeclaMidia::TocarPausar,
                },
                cor: Cor::Verde,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        midia.insert(
            14,
            Controle {
                nome: "Anterior".into(),
                acao: Acao::Midia {
                    tecla: TeclaMidia::Anterior,
                },
                cor: Cor::Ciano,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        midia.insert(
            15,
            Controle {
                nome: "Próxima".into(),
                acao: Acao::Midia {
                    tecla: TeclaMidia::Proxima,
                },
                cor: Cor::Ciano,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        midia.insert(
            9,
            Controle {
                nome: "Aumentar volume".into(),
                acao: Acao::Midia {
                    tecla: TeclaMidia::AumentarVolume,
                },
                cor: Cor::Lima,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        midia.insert(
            10,
            Controle {
                nome: "Diminuir volume".into(),
                acao: Acao::Midia {
                    tecla: TeclaMidia::DiminuirVolume,
                },
                cor: Cor::Lima,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );
        midia.insert(
            11,
            Controle {
                nome: "Mudo".into(),
                acao: Acao::Midia {
                    tecla: TeclaMidia::Mudo,
                },
                cor: Cor::Vermelho,
                cor_pressionado: None,
                cor_aberto: None,
                brilho: None,
                gerenciar_janela: false,
            },
        );

        Config {
            versao: VERSAO_ATUAL,
            paginas: vec![
                Pagina {
                    nome: "Apps".into(),
                    pads: apps,
                    botoes: BTreeMap::new(),
                },
                Pagina {
                    nome: "Mídia".into(),
                    pads: midia,
                    botoes: BTreeMap::new(),
                },
            ],
            botao_proxima_pagina: botao_proxima_padrao(),
            botao_pagina_anterior: botao_anterior_padrao(),
            brilho: brilho_padrao(),
            strip: FuncaoStrip::Volume,
            calibracao_strip: Default::default(),
            knob: Default::default(),
            home_assistant: HomeAssistant::default(),
            descanso: Default::default(),
            ao_apertar: Default::default(),
        }
    }
}

/// Cores no JSON são o nome em minúsculas, para o arquivo ficar legível à mão.
mod cor_serde {
    use crate::hid::Cor;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(cor: &Cor, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(super::nome_da_cor(*cor))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Cor, D::Error> {
        let texto = String::deserialize(d)?;
        super::cor_do_nome(&texto).ok_or_else(|| serde::de::Error::custom(format!(
            "cor desconhecida: {texto}"
        )))
    }
}

mod cor_opcional_serde {
    use crate::hid::Cor;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(cor: &Option<Cor>, s: S) -> Result<S::Ok, S::Error> {
        match cor {
            Some(c) => s.serialize_str(super::nome_da_cor(*c)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Cor>, D::Error> {
        let texto = Option::<String>::deserialize(d)?;
        match texto {
            None => Ok(None),
            Some(t) => super::cor_do_nome(&t).map(Some).ok_or_else(|| {
                serde::de::Error::custom(format!("cor desconhecida: {t}"))
            }),
        }
    }
}

pub fn nome_da_cor(cor: Cor) -> &'static str {
    match cor {
        Cor::Apagado => "apagado",
        Cor::Vermelho => "vermelho",
        Cor::Laranja => "laranja",
        Cor::LaranjaClaro => "laranja_claro",
        Cor::AmareloQuente => "amarelo_quente",
        Cor::Amarelo => "amarelo",
        Cor::Lima => "lima",
        Cor::Verde => "verde",
        Cor::Menta => "menta",
        Cor::Ciano => "ciano",
        Cor::Turquesa => "turquesa",
        Cor::Azul => "azul",
        Cor::Ameixa => "ameixa",
        Cor::Violeta => "violeta",
        Cor::Roxo => "roxo",
        Cor::Magenta => "magenta",
        Cor::Fucsia => "fucsia",
        Cor::Branco => "branco",
    }
}

pub fn cor_do_nome(nome: &str) -> Option<Cor> {
    Some(match nome {
        "apagado" => Cor::Apagado,
        "vermelho" => Cor::Vermelho,
        "laranja" => Cor::Laranja,
        "laranja_claro" => Cor::LaranjaClaro,
        "amarelo_quente" => Cor::AmareloQuente,
        "amarelo" => Cor::Amarelo,
        "lima" => Cor::Lima,
        "verde" => Cor::Verde,
        "menta" => Cor::Menta,
        "ciano" => Cor::Ciano,
        "turquesa" => Cor::Turquesa,
        "azul" => Cor::Azul,
        "ameixa" => Cor::Ameixa,
        "violeta" => Cor::Violeta,
        "roxo" => Cor::Roxo,
        "magenta" => Cor::Magenta,
        "fucsia" => Cor::Fucsia,
        "branco" => Cor::Branco,
        _ => return None,
    })
}

#[cfg(test)]
mod testes_gravacao {
    use super::Config;

    #[test]
    fn salvar_nao_deixa_arquivo_temporario_para_tras() {
        let pasta = std::env::temp_dir().join("mikrodeck-teste-gravacao");
        let _ = std::fs::remove_dir_all(&pasta);
        let caminho = pasta.join("config.json");

        let config = Config::exemplo();
        config.salvar(&caminho).unwrap();
        assert!(caminho.exists(), "config não foi gravada");
        assert!(
            !caminho.with_extension("json.tmp").exists(),
            "sobrou arquivo temporário"
        );

        // Gravar por cima também tem que funcionar, que é o caso do dia a dia.
        config.salvar(&caminho).unwrap();
        let lida = Config::carregar_ou_criar(&caminho).unwrap();
        assert_eq!(lida.paginas.len(), config.paginas.len());

        let _ = std::fs::remove_dir_all(&pasta);
    }
}

#[cfg(test)]
mod testes_calibracao {
    use super::CalibracaoStrip;

    #[test]
    fn sem_calibrar_a_fracao_e_a_posicao_crua() {
        let c = CalibracaoStrip::default();
        assert_eq!(c.fracao(0), 0.0);
        assert_eq!(c.fracao(255), 1.0);
    }

    #[test]
    fn calibrado_o_maximo_alcancavel_vira_cem_por_cento() {
        // O dedo do Davi só chega a 240; isso tem que valer 100%.
        let c = CalibracaoStrip {
            minimo: 10,
            maximo: 240,
        };
        assert_eq!(c.fracao(240), 1.0);
        assert_eq!(c.fracao(10), 0.0);
        assert!((c.fracao(125) - 0.5).abs() < 0.01, "meio da faixa");
    }

    #[test]
    fn fora_da_faixa_nao_estoura() {
        let c = CalibracaoStrip {
            minimo: 10,
            maximo: 240,
        };
        assert_eq!(c.fracao(255), 1.0);
        assert_eq!(c.fracao(0), 0.0);
    }

    #[test]
    fn calibracao_invertida_cai_no_cru_em_vez_de_travar() {
        let c = CalibracaoStrip {
            minimo: 200,
            maximo: 50,
        };
        assert_eq!(c.fracao(255), 1.0);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn exemplo_vai_e_volta_do_json() {
        let c = Config::exemplo();
        let json = serde_json::to_string_pretty(&c).unwrap();
        let de_volta: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(de_volta.paginas.len(), c.paginas.len());
        assert_eq!(de_volta.paginas[0].nome, "Apps");
        assert_eq!(de_volta.brilho, c.brilho);
    }

    #[test]
    fn todas_as_cores_tem_nome_e_voltam() {
        let todas = [
            Cor::Apagado, Cor::Vermelho, Cor::Laranja, Cor::LaranjaClaro,
            Cor::AmareloQuente, Cor::Amarelo, Cor::Lima, Cor::Verde,
            Cor::Menta, Cor::Ciano, Cor::Turquesa, Cor::Azul,
            Cor::Ameixa, Cor::Violeta, Cor::Roxo, Cor::Magenta,
            Cor::Fucsia, Cor::Branco,
        ];
        for cor in todas {
            let nome = nome_da_cor(cor);
            assert_eq!(cor_do_nome(nome), Some(cor), "cor {nome} não voltou");
        }
    }

    #[test]
    fn cor_invalida_no_json_da_erro_claro() {
        let json = r#"{"nome":"X","cor":"turquesa_neon"}"#;
        let erro = serde_json::from_str::<Controle>(json).unwrap_err().to_string();
        assert!(erro.contains("cor desconhecida"), "erro foi: {erro}");
    }

    #[test]
    fn campos_opcionais_tem_padrao() {
        // Config mínima: só o essencial, o resto vem do padrão.
        let json = r#"{"versao":1,"paginas":[{"nome":"Só uma"}]}"#;
        let c: Config = serde_json::from_str(json).unwrap();
        assert_eq!(c.botao_proxima_pagina, "seta_direita");
        assert_eq!(c.brilho, 2);
        assert!(c.paginas[0].pads.is_empty());
    }

    #[test]
    fn controle_sem_acao_vira_nenhuma() {
        let json = r#"{"nome":"Vazio","cor":"verde"}"#;
        let c: Controle = serde_json::from_str(json).unwrap();
        assert_eq!(c.acao, Acao::Nenhuma);
        assert_eq!(c.cor, Cor::Verde);
    }
}
