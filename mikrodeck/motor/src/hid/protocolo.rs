//! Constantes e tabelas do protocolo HID do Maschine Mikro MK3.
//!
//! Tudo aqui foi confirmado no aparelho real durante o spike. Ver `docs/spike-hid.md`.

/// Native Instruments.
pub const VID: u16 = 0x17cc;
/// Maschine Mikro MK3.
pub const PID: u16 = 0x1700;

/// Report de saída dos LEDs. 81 bytes no total: 1 de ID e 80 de dados.
pub const REPORT_LEDS: u8 = 0x80;
/// Tamanho total do frame de LEDs, com o byte de ID.
pub const TAM_FRAME_LEDS: usize = 81;

/// Report de saída da tela. Dois pacotes de 265 bytes, um por metade.
pub const REPORT_TELA: u8 = 0xE0;

/// Report de entrada dos botões, knob e touch strip.
pub const REPORT_BOTOES: u8 = 0x01;
/// Report de entrada dos pads, com pressão.
pub const REPORT_PADS: u8 = 0x02;

/// Onde cada bloco começa dentro do frame de LEDs (índices já contando o byte de ID).
pub const OFFSET_BOTOES: usize = 1;
pub const OFFSET_PADS: usize = 40;
pub const OFFSET_STRIP: usize = 56;

pub const NUM_BOTOES: usize = 39;
pub const NUM_PADS: usize = 16;
pub const NUM_STRIP: usize = 25;

/// Mapeia o índice bruto que o aparelho manda (0..15) para o número lógico do pad (0..15).
/// O número impresso no aparelho é o lógico mais um: lógico 12 é o pad "13", o canto
/// superior esquerdo. Índice bruto 0 é justamente esse canto.
pub const ORDEM_PADS: [u8; NUM_PADS] = [12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3];

/// Converte o número impresso no aparelho (1..16) no índice bruto usado no frame de LEDs.
pub fn pad_impresso_para_bruto(impresso: u8) -> Option<usize> {
    if !(1..=16).contains(&impresso) {
        return None;
    }
    let logico = impresso - 1;
    ORDEM_PADS.iter().position(|&v| v == logico)
}

/// Converte o índice bruto vindo do aparelho no número impresso (1..16).
pub fn pad_bruto_para_impresso(bruto: u8) -> Option<u8> {
    ORDEM_PADS.get(bruto as usize).map(|logico| logico + 1)
}

/// Botões na ordem exata em que aparecem no frame de LEDs e no bitmask de entrada.
/// São 39 botões com LED. O `enter` (apertar o knob) é o bit 39 na entrada e não tem LED.
pub const BOTOES: [&str; NUM_BOTOES] = [
    "maschine",
    "estrela",
    "busca",
    "volume",
    "swing",
    "tempo",
    "plug_in",
    "sampling",
    "seta_esquerda",
    "seta_direita",
    "pitch",
    "mod",
    "perform",
    "notes",
    "group",
    "auto",
    "lock",
    "note_repeat",
    "restart",
    "erase",
    "tap",
    "follow",
    "play",
    "rec",
    "stop",
    "shift",
    "fixed_vel",
    "pad_mode",
    "keyboard",
    "chords",
    "step",
    "scene",
    "pattern",
    "events",
    "variation",
    "duplicate",
    "select",
    "solo",
    "mute",
];

/// Índice do bit de "apertar o knob" no bitmask de entrada. Não tem LED correspondente.
pub const BIT_KNOB_APERTADO: usize = 39;

/// Cores que o aparelho aceita nos pads e na touch strip.
/// O byte final é `(cor << 2) | brilho`, com brilho de 0 a 3.
/// Brilho 0 não apaga, é o nível mais fraco. O que apaga é o byte inteiro em zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cor {
    Apagado = 0,
    Vermelho = 1,
    Laranja = 2,
    LaranjaClaro = 3,
    AmareloQuente = 4,
    Amarelo = 5,
    Lima = 6,
    Verde = 7,
    Menta = 8,
    Ciano = 9,
    Turquesa = 10,
    Azul = 11,
    Ameixa = 12,
    Violeta = 13,
    Roxo = 14,
    Magenta = 15,
    Fucsia = 16,
    Branco = 17,
}

impl Cor {
    /// Monta o byte de LED para pad ou strip. Brilho vai de 0 (fraco) a 3 (forte).
    /// `Cor::Apagado` sempre vira zero, que é o que realmente apaga o LED.
    pub fn byte(self, brilho: u8) -> u8 {
        if self == Cor::Apagado {
            0
        } else {
            ((self as u8) << 2) | (brilho & 0b11)
        }
    }
}

/// Brilho dos LEDs de botão. São monocromáticos, então só o nível importa.
/// Valores confirmados na captura USB do próprio software da Native Instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BrilhoBotao {
    Apagado = 0x00,
    Fraco = 0x7C,
    Normal = 0x7E,
    Forte = 0x7F,
}
