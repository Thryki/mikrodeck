//! Decide o que aparece na tela do aparelho a cada momento.
//!
//! A prioridade está em `docs/ui-spec.md`: segurar um controle ganha do feedback
//! de ação, que ganha do normal, que ganha do descanso.

use super::Tela;
use std::time::{Duration, Instant};

/// Quanto tempo um aviso de ação fica na tela antes de sumir.
const DURACAO_AVISO: Duration = Duration::from_millis(1400);

/// Intervalo entre dois quadros do texto que corre no descanso. Cada quadro custa
/// duas escritas no aparelho, que aguenta cerca de 31 por segundo, então o passo
/// não pode ser mais rápido que isto.
const PASSO_DESCANSO: Duration = Duration::from_millis(125);

/// Quantos pixels o texto anda por quadro.
const PIXELS_POR_PASSO: usize = 3;

/// Espaço em branco entre o fim do texto e o começo da repetição.
const VAO_DESCANSO: usize = 40;

/// O que a tela está mostrando agora.
#[derive(Debug, Clone, PartialEq)]
pub enum Cena {
    /// Nome da página e a posição. É o estado normal.
    Pagina { numero: usize, total: usize, nome: String },
    /// Nome do controle sendo segurado.
    Segurando { nome: String },
    /// Aviso passageiro, com uma barra opcional de 0 a 1.
    Aviso { titulo: String, barra: Option<f32> },
    /// Descanso: um texto correndo da direita para a esquerda.
    Descanso { texto: String, passo: usize },
}

pub struct Compositor {
    cena_base: Cena,
    segurando: Option<String>,
    aviso: Option<(Cena, Instant)>,
    /// Última cena desenhada, para não redesenhar à toa.
    desenhada: Option<Cena>,
    /// Texto do descanso e quanto tempo parado até ele começar. `None` desliga.
    descanso: Option<(String, Duration)>,
    /// Quando alguém mexeu no aparelho pela última vez.
    ultimo_toque: Instant,
}

impl Compositor {
    pub fn novo(numero: usize, total: usize, nome: String) -> Self {
        Self {
            cena_base: Cena::Pagina { numero, total, nome },
            segurando: None,
            aviso: None,
            desenhada: None,
            descanso: None,
            ultimo_toque: Instant::now(),
        }
    }

    /// Liga o descanso com um texto e uma espera. `None` desliga.
    pub fn definir_descanso(&mut self, descanso: Option<(String, Duration)>) {
        self.descanso = descanso.filter(|(texto, _)| !texto.trim().is_empty());
    }

    /// Marca que alguém mexeu no aparelho agora, adiando o descanso.
    pub fn tocou(&mut self) {
        self.ultimo_toque = Instant::now();
    }

    /// Troca a página mostrada e avisa na tela por um instante.
    pub fn pagina(&mut self, numero: usize, total: usize, nome: String) {
        self.cena_base = Cena::Pagina {
            numero,
            total,
            nome: nome.clone(),
        };
        self.avisar(Cena::Aviso {
            titulo: nome,
            barra: None,
        });
    }

    /// Um controle passou a ser segurado. Passar `None` solta.
    pub fn segurando(&mut self, nome: Option<String>) {
        self.segurando = nome;
    }

    /// Mostra um aviso passageiro, por exemplo o volume mudando.
    pub fn avisar(&mut self, cena: Cena) {
        self.aviso = Some((cena, Instant::now()));
        self.tocou();
    }

    /// A cena que deve estar na tela agora.
    fn cena_atual(&self) -> Cena {
        if let Some(nome) = &self.segurando {
            return Cena::Segurando { nome: nome.clone() };
        }
        if let Some((cena, quando)) = &self.aviso {
            if quando.elapsed() < DURACAO_AVISO {
                return cena.clone();
            }
        }
        if let Some((texto, espera)) = &self.descanso {
            let parado = self.ultimo_toque.elapsed();
            if parado >= *espera {
                let passo = (parado - *espera).as_millis() / PASSO_DESCANSO.as_millis();
                return Cena::Descanso {
                    texto: texto.clone(),
                    passo: passo as usize,
                };
            }
        }
        self.cena_base.clone()
    }

    /// Redesenha a tela se a cena mudou. Devolve `true` se desenhou.
    pub fn desenhar(&mut self, tela: &mut Tela) -> bool {
        let cena = self.cena_atual();
        if self.desenhada.as_ref() == Some(&cena) {
            return false;
        }
        tela.limpar();
        match &cena {
            // A tela tem 128 por 32. O nome vem grande, porque é o que se lê de
            // relance; o resto fica pequeno embaixo.
            Cena::Pagina { numero, total, nome } => {
                tela.texto_ajustado(4, nome, 2);
                let posicao = format!("{numero} de {total}");
                tela.texto_centrado(24, &posicao);
            }
            Cena::Segurando { nome } => {
                tela.texto_ajustado(9, nome, 2);
            }
            Cena::Aviso { titulo, barra } => match barra {
                Some(fracao) => {
                    tela.texto_ajustado(2, titulo, 2);
                    tela.barra(8, 22, 112, 9, *fracao);
                }
                None => {
                    tela.texto_ajustado(9, titulo, 2);
                }
            },
            Cena::Descanso { texto, passo } => {
                let largura = super::fonte::largura_do_texto(texto) * 2;
                let ciclo = largura + VAO_DESCANSO;
                // O texto entra pela direita e sai pela esquerda, sem parar.
                let andado = (passo * PIXELS_POR_PASSO) % ciclo;
                let x = super::LARGURA as isize - andado as isize;
                tela.texto_deslocado(x, 9, texto, 2);
            }
        }
        self.desenhada = Some(cena);
        true
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn segurar_um_controle_ganha_da_pagina() {
        let mut c = Compositor::novo(1, 2, "Apps".into());
        c.segurando(Some("Chrome".into()));
        assert_eq!(
            c.cena_atual(),
            Cena::Segurando {
                nome: "Chrome".into()
            }
        );
    }

    #[test]
    fn soltar_volta_para_a_pagina() {
        let mut c = Compositor::novo(1, 2, "Apps".into());
        c.segurando(Some("Chrome".into()));
        c.segurando(None);
        assert!(matches!(c.cena_atual(), Cena::Pagina { .. }));
    }

    #[test]
    fn trocar_de_pagina_mostra_o_nome_e_depois_fica_na_pagina() {
        let mut c = Compositor::novo(1, 2, "Apps".into());
        c.pagina(2, 2, "Mídia".into());
        assert_eq!(
            c.cena_atual(),
            Cena::Aviso {
                titulo: "Mídia".into(),
                barra: None
            }
        );
        // Depois que o aviso expira, sobra a página.
        c.aviso = None;
        assert_eq!(
            c.cena_atual(),
            Cena::Pagina {
                numero: 2,
                total: 2,
                nome: "Mídia".into()
            }
        );
    }

    #[test]
    fn nao_redesenha_quando_nada_muda() {
        let mut c = Compositor::novo(1, 2, "Apps".into());
        let mut t = Tela::nova();
        assert!(c.desenhar(&mut t), "primeira vez sempre desenha");
        assert!(!c.desenhar(&mut t), "sem mudança não redesenha");
        c.segurando(Some("Chrome".into()));
        assert!(c.desenhar(&mut t));
    }

    #[test]
    fn descanso_so_entra_depois_da_espera() {
        let mut c = Compositor::novo(1, 1, "Apps".into());
        c.definir_descanso(Some(("MikroDeck".into(), Duration::from_millis(30))));
        assert!(matches!(c.cena_atual(), Cena::Pagina { .. }));
        std::thread::sleep(Duration::from_millis(60));
        assert!(matches!(c.cena_atual(), Cena::Descanso { .. }));
    }

    #[test]
    fn mexer_no_aparelho_tira_o_descanso() {
        let mut c = Compositor::novo(1, 1, "Apps".into());
        c.definir_descanso(Some(("MikroDeck".into(), Duration::from_millis(30))));
        std::thread::sleep(Duration::from_millis(60));
        assert!(matches!(c.cena_atual(), Cena::Descanso { .. }));
        c.tocou();
        assert!(matches!(c.cena_atual(), Cena::Pagina { .. }));
    }

    #[test]
    fn texto_vazio_nao_liga_o_descanso() {
        let mut c = Compositor::novo(1, 1, "Apps".into());
        c.definir_descanso(Some(("   ".into(), Duration::from_millis(1))));
        std::thread::sleep(Duration::from_millis(20));
        assert!(matches!(c.cena_atual(), Cena::Pagina { .. }));
    }

    #[test]
    fn o_texto_do_descanso_anda_e_volta_ao_comeco() {
        let mut c = Compositor::novo(1, 1, "Apps".into());
        let mut t = Tela::nova();
        // Dois passos diferentes tem que dar desenhos diferentes, senao nao anda.
        c.avisar(Cena::Descanso {
            texto: "MikroDeck".into(),
            passo: 0,
        });
        c.desenhar(&mut t);
        let primeiro = t.pixels_acesos();
        c.avisar(Cena::Descanso {
            texto: "MikroDeck".into(),
            passo: 8,
        });
        c.desenhar(&mut t);
        assert_ne!(primeiro, t.pixels_acesos());
    }

    #[test]
    fn aviso_com_barra_desenha_sem_estourar() {
        let mut c = Compositor::novo(1, 1, "Apps".into());
        let mut t = Tela::nova();
        c.avisar(Cena::Aviso {
            titulo: "Volume".into(),
            barra: Some(0.75),
        });
        assert!(c.desenhar(&mut t));
    }
}
