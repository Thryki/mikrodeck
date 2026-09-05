//! A tela do aparelho: 128 x 32 pixels, 1 bit por pixel.
//!
//! O motor mantém um framebuffer aqui e só manda para o aparelho quando muda,
//! igual ao frame de LEDs. O empacotamento segue o report `0xe0`: dois pacotes
//! de 265 bytes, um por metade, com o bitmap invertido (bit 1 apaga o pixel).

pub mod composicao;
pub mod fonte;

use crate::hid::protocolo::REPORT_TELA;

pub const LARGURA: usize = 128;
pub const ALTURA: usize = 32;

/// Framebuffer da tela. Um byte por pixel na memória, para o desenho ficar simples;
/// o empacotamento em bits acontece só na hora de mandar.
pub struct Tela {
    pixels: [bool; LARGURA * ALTURA],
    sujo: bool,
}

impl Default for Tela {
    fn default() -> Self {
        Self::nova()
    }
}

impl Tela {
    pub fn nova() -> Self {
        Self {
            pixels: [false; LARGURA * ALTURA],
            sujo: true,
        }
    }

    pub fn limpar(&mut self) {
        if self.pixels.iter().any(|&p| p) {
            self.pixels = [false; LARGURA * ALTURA];
            self.sujo = true;
        }
    }

    pub fn esta_suja(&self) -> bool {
        self.sujo
    }

    pub fn marcar_limpa(&mut self) {
        self.sujo = false;
    }

    pub fn pixel(&mut self, x: usize, y: usize, aceso: bool) {
        if x >= LARGURA || y >= ALTURA {
            return;
        }
        let i = y * LARGURA + x;
        if self.pixels[i] != aceso {
            self.pixels[i] = aceso;
            self.sujo = true;
        }
    }

    /// Quantos pixels estão acesos. Serve para teste comparar dois desenhos.
    pub fn pixels_acesos(&self) -> usize {
        self.pixels.iter().filter(|p| **p).count()
    }

    /// Escreve texto começando em (x, y). Corta o que passar da borda.
    /// Devolve onde o texto terminou.
    pub fn texto(&mut self, x: usize, y: usize, texto: &str) -> usize {
        let mut cursor = x;
        for c in texto.chars() {
            let Some(glifo) = fonte::glifo(c) else {
                cursor += fonte::AVANCO;
                continue;
            };
            for (coluna, bits) in glifo.iter().enumerate() {
                for linha in 0..fonte::ALTURA {
                    if bits & (1 << linha) != 0 {
                        self.pixel(cursor + coluna, y + linha, true);
                    }
                }
            }
            cursor += fonte::AVANCO;
            if cursor >= LARGURA {
                break;
            }
        }
        cursor
    }

    /// Escreve texto centralizado na horizontal.
    pub fn texto_centrado(&mut self, y: usize, texto: &str) {
        let largura = fonte::largura_do_texto(texto);
        let x = LARGURA.saturating_sub(largura) / 2;
        self.texto(x, y, texto);
    }

    /// Escreve texto com cada pixel virando um quadrado de `escala` por `escala`.
    /// A fonte tem 5 por 7; na tela de 128 por 32, escala 1 fica pequena demais
    /// para ler de relance.
    pub fn texto_escalado(&mut self, x: usize, y: usize, texto: &str, escala: usize) {
        let escala = escala.max(1);
        let mut cursor = x;
        for c in texto.chars() {
            let Some(glifo) = fonte::glifo(c) else {
                cursor += fonte::AVANCO * escala;
                continue;
            };
            for (coluna, bits) in glifo.iter().enumerate() {
                for linha in 0..fonte::ALTURA {
                    if bits & (1 << linha) != 0 {
                        self.retangulo(
                            cursor + coluna * escala,
                            y + linha * escala,
                            escala,
                            escala,
                            true,
                        );
                    }
                }
            }
            cursor += fonte::AVANCO * escala;
            if cursor >= LARGURA {
                break;
            }
        }
    }

    /// Escreve texto a partir de um x que pode ser negativo, para o texto poder
    /// entrar e sair pela borda. É o que faz o descanso correr.
    pub fn texto_deslocado(&mut self, x: isize, y: usize, texto: &str, escala: usize) {
        let escala = escala.max(1);
        let avanco = (fonte::AVANCO * escala) as isize;
        let mut cursor = x;
        for c in texto.chars() {
            if cursor >= LARGURA as isize {
                break;
            }
            // Só desenha o que já entrou pela esquerda; o resto é puro custo.
            if cursor + avanco > 0 {
                if let Some(glifo) = fonte::glifo(c) {
                    for (coluna, bits) in glifo.iter().enumerate() {
                        let px = cursor + (coluna * escala) as isize;
                        if px < 0 {
                            continue;
                        }
                        for linha in 0..fonte::ALTURA {
                            if bits & (1 << linha) != 0 {
                                self.retangulo(
                                    px as usize,
                                    y + linha * escala,
                                    escala,
                                    escala,
                                    true,
                                );
                            }
                        }
                    }
                }
            }
            cursor += avanco;
        }
    }

    /// Escreve centralizado, na maior escala que couber, até `maxima`.
    /// Devolve a escala usada.
    pub fn texto_ajustado(&mut self, y: usize, texto: &str, maxima: usize) -> usize {
        let largura = fonte::largura_do_texto(texto).max(1);
        let mut escala = maxima.max(1);
        while escala > 1 && largura * escala > LARGURA {
            escala -= 1;
        }
        let x = LARGURA.saturating_sub(largura * escala) / 2;
        self.texto_escalado(x, y, texto, escala);
        escala
    }

    /// Retângulo preenchido. Serve para barra de volume e realce.
    pub fn retangulo(&mut self, x: usize, y: usize, largura: usize, altura: usize, aceso: bool) {
        for dy in 0..altura {
            for dx in 0..largura {
                self.pixel(x + dx, y + dy, aceso);
            }
        }
    }

    /// Contorno de retângulo, sem preencher.
    pub fn moldura(&mut self, x: usize, y: usize, largura: usize, altura: usize) {
        if largura == 0 || altura == 0 {
            return;
        }
        for dx in 0..largura {
            self.pixel(x + dx, y, true);
            self.pixel(x + dx, y + altura - 1, true);
        }
        for dy in 0..altura {
            self.pixel(x, y + dy, true);
            self.pixel(x + largura - 1, y + dy, true);
        }
    }

    /// Barra de progresso, de 0.0 a 1.0. Usada para volume e brilho.
    pub fn barra(&mut self, x: usize, y: usize, largura: usize, altura: usize, fracao: f32) {
        self.moldura(x, y, largura, altura);
        let dentro = largura.saturating_sub(4);
        let cheio = (dentro as f32 * fracao.clamp(0.0, 1.0)).round() as usize;
        if cheio > 0 && altura > 4 {
            self.retangulo(x + 2, y + 2, cheio, altura - 4, true);
        }
    }

    /// Monta os dois pacotes do report `0xe0` prontos para escrever no aparelho.
    ///
    /// Cada pacote cobre metade da tela: 128 colunas por 16 linhas, com 2 bytes por
    /// coluna. O bitmap é invertido, então bit 1 significa pixel apagado.
    pub fn pacotes(&self) -> [Vec<u8>; 2] {
        let mut saida = [Vec::with_capacity(265), Vec::with_capacity(265)];
        for (metade, pacote) in saida.iter_mut().enumerate() {
            pacote.push(REPORT_TELA);
            pacote.extend_from_slice(&[0x00, 0x00]); // x = 0
            pacote.extend_from_slice(&[(metade as u8) * 2, 0x00]); // y em blocos de 8 linhas
            pacote.extend_from_slice(&[0x80, 0x00]); // largura 128
            pacote.extend_from_slice(&[0x02, 0x00]); // altura 2 blocos = 16 linhas

            // 256 bytes: 2 blocos de 8 linhas, cada um com 128 colunas.
            for bloco in 0..2 {
                for x in 0..LARGURA {
                    let mut byte = 0u8;
                    for bit in 0..8 {
                        byte <<= 1;
                        let y = metade * 16 + bloco * 8 + (7 - bit);
                        // Invertido: bit 1 apaga o pixel.
                        if !self.pixels[y * LARGURA + x] {
                            byte |= 1;
                        }
                    }
                    pacote.push(byte);
                }
            }
        }
        saida
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn pacotes_tem_o_tamanho_que_o_aparelho_exige() {
        let t = Tela::nova();
        let p = t.pacotes();
        assert_eq!(p[0].len(), 265);
        assert_eq!(p[1].len(), 265);
        assert_eq!(p[0][0], REPORT_TELA);
    }

    #[test]
    fn tela_apagada_manda_bits_em_um() {
        // O bitmap é invertido: sem pixel aceso, todos os bits de dados são 1.
        let t = Tela::nova();
        let p = t.pacotes();
        assert!(p[0][9..].iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn pixel_aceso_zera_o_bit_correspondente() {
        let mut t = Tela::nova();
        t.pixel(0, 0, true);
        let p = t.pacotes();
        // O primeiro byte de dados cobre a coluna 0, linhas 0 a 7. A linha 0 cai no
        // bit menos significativo, e o bitmap é invertido, então acender a linha 0
        // zera só esse bit: 0xFF vira 0xFE. Mesma ordem que o pymikro usa.
        assert_eq!(p[0][9], 0xFE);
    }

    #[test]
    fn linha_de_baixo_do_bloco_cai_no_bit_mais_alto() {
        let mut t = Tela::nova();
        t.pixel(0, 7, true);
        let p = t.pacotes();
        assert_eq!(p[0][9], 0x7F);
    }

    #[test]
    fn as_duas_metades_cobrem_alturas_diferentes() {
        let t = Tela::nova();
        let p = t.pacotes();
        assert_eq!(p[0][3], 0, "primeira metade começa na linha 0");
        assert_eq!(p[1][3], 2, "segunda metade começa no bloco 2, linha 16");
    }

    #[test]
    fn texto_acende_pixels() {
        let mut t = Tela::nova();
        t.limpar();
        t.marcar_limpa();
        t.texto(0, 0, "A");
        assert!(t.esta_suja());
        assert!(t.pixels.iter().any(|&p| p), "o texto não acendeu nada");
    }

    #[test]
    fn texto_nao_estoura_a_borda() {
        let mut t = Tela::nova();
        // Muito mais do que cabe: não pode entrar em pânico nem escrever fora.
        t.texto(0, 0, &"W".repeat(80));
        t.texto(120, 28, "teste que passa da borda");
    }

    #[test]
    fn so_fica_suja_quando_o_pixel_muda() {
        let mut t = Tela::nova();
        t.marcar_limpa();
        t.pixel(5, 5, false);
        assert!(!t.esta_suja(), "apagar pixel já apagado não suja");
        t.pixel(5, 5, true);
        assert!(t.esta_suja());
    }

    #[test]
    fn barra_respeita_os_limites() {
        let mut t = Tela::nova();
        t.barra(0, 0, 100, 10, 2.0);
        t.barra(0, 12, 100, 10, -1.0);
    }
}
