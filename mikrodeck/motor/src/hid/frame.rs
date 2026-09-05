//! O frame de LEDs: o estado completo de todas as luzes do aparelho.
//!
//! O aparelho não aceita acender um LED isolado. Cada escrita manda o frame inteiro,
//! os 81 bytes. Por isso o motor mantém este buffer e o marca como sujo quando muda,
//! deixando a thread de escrita mandar no próximo tick.

use super::protocolo::*;

/// Estado de todos os LEDs do aparelho: 39 botões, 16 pads e 25 LEDs da touch strip.
#[derive(Clone)]
pub struct FrameLeds {
    bytes: [u8; TAM_FRAME_LEDS],
    sujo: bool,
}

impl Default for FrameLeds {
    fn default() -> Self {
        Self::novo()
    }
}

impl FrameLeds {
    pub fn novo() -> Self {
        let mut bytes = [0u8; TAM_FRAME_LEDS];
        bytes[0] = REPORT_LEDS;
        Self { bytes, sujo: true }
    }

    /// Acende um pad pelo número impresso no aparelho (1 a 16).
    /// Devolve `false` se o número estiver fora da faixa.
    pub fn pad(&mut self, impresso: u8, cor: Cor, brilho: u8) -> bool {
        match pad_impresso_para_bruto(impresso) {
            Some(bruto) => {
                self.escrever(OFFSET_PADS + bruto, cor.byte(brilho));
                true
            }
            None => false,
        }
    }

    /// Acende um botão pelo nome da tabela `protocolo::BOTOES`.
    /// Devolve `false` se o nome não existir.
    pub fn botao(&mut self, nome: &str, brilho: BrilhoBotao) -> bool {
        match BOTOES.iter().position(|&b| b == nome) {
            Some(i) => {
                self.escrever(OFFSET_BOTOES + i, brilho as u8);
                true
            }
            None => false,
        }
    }

    /// Acende um LED da touch strip, de 0 a 24, contando da esquerda.
    pub fn strip(&mut self, indice: usize, cor: Cor, brilho: u8) -> bool {
        if indice >= NUM_STRIP {
            return false;
        }
        self.escrever(OFFSET_STRIP + indice, cor.byte(brilho));
        true
    }

    /// Apaga tudo.
    pub fn limpar(&mut self) {
        for i in 1..TAM_FRAME_LEDS {
            self.escrever(i, 0);
        }
    }

    /// Bytes prontos para mandar ao aparelho.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn esta_sujo(&self) -> bool {
        self.sujo
    }

    pub fn marcar_limpo(&mut self) {
        self.sujo = false;
    }

    /// Escreve um byte e só marca o frame como sujo se o valor mudou de verdade.
    /// É o que evita mandar frame igual ao anterior a cada tick.
    fn escrever(&mut self, pos: usize, valor: u8) {
        if self.bytes[pos] != valor {
            self.bytes[pos] = valor;
            self.sujo = true;
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn pad_13_e_o_primeiro_byte_do_bloco_de_pads() {
        // O pad impresso "13" é o canto superior esquerdo, e no aparelho ele é o índice bruto 0.
        assert_eq!(pad_impresso_para_bruto(13), Some(0));
        let mut f = FrameLeds::novo();
        f.pad(13, Cor::Vermelho, 3);
        assert_eq!(f.bytes()[OFFSET_PADS], Cor::Vermelho.byte(3));
    }

    #[test]
    fn pad_1_e_o_canto_inferior_esquerdo() {
        // Impresso 1 -> lógico 0 -> posição 12 na ordem do aparelho.
        assert_eq!(pad_impresso_para_bruto(1), Some(12));
    }

    #[test]
    fn ida_e_volta_do_mapeamento_de_pads() {
        for impresso in 1..=16u8 {
            let bruto = pad_impresso_para_bruto(impresso).unwrap();
            assert_eq!(pad_bruto_para_impresso(bruto as u8), Some(impresso));
        }
    }

    #[test]
    fn apagado_zera_o_byte_inteiro() {
        // Brilho 0 com cor é o nível fraco, não apagado. Só Cor::Apagado zera.
        assert_eq!(Cor::Apagado.byte(3), 0);
        assert_ne!(Cor::Vermelho.byte(0), 0);
    }

    #[test]
    fn frame_so_fica_sujo_quando_o_valor_muda() {
        let mut f = FrameLeds::novo();
        f.marcar_limpo();
        f.pad(5, Cor::Azul, 3);
        assert!(f.esta_sujo());
        f.marcar_limpo();
        f.pad(5, Cor::Azul, 3);
        assert!(!f.esta_sujo(), "escrever o mesmo valor não deve sujar o frame");
    }

    #[test]
    fn todos_os_botoes_cabem_no_bloco_de_botoes() {
        let mut f = FrameLeds::novo();
        for nome in BOTOES {
            assert!(f.botao(nome, BrilhoBotao::Normal), "botão {nome} não mapeou");
        }
        // O último botão fica logo antes do bloco de pads.
        assert_eq!(OFFSET_BOTOES + NUM_BOTOES, OFFSET_PADS);
    }

    #[test]
    fn os_tres_blocos_preenchem_o_frame_exatamente() {
        assert_eq!(OFFSET_STRIP + NUM_STRIP, TAM_FRAME_LEDS);
    }
}
