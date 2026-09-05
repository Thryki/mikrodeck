//! Eventos de entrada do aparelho, já decodificados.

/// O que o aparelho manda para o motor.
#[derive(Debug, Clone, PartialEq)]
pub enum Evento {
    /// Dedo encostou no pad, ainda sem força suficiente para contar como toque.
    PadTocado { pad: u8, pressao: u16 },
    /// Pad apertado de verdade. `pad` é o número impresso no aparelho, de 1 a 16.
    PadApertado { pad: u8, pressao: u16 },
    /// Pad solto.
    PadSolto { pad: u8 },
    /// Botão apertado ou solto. `nome` vem da tabela `protocolo::BOTOES`.
    Botao { nome: &'static str, apertado: bool },
    /// Knob apertado ou solto.
    KnobApertado { apertado: bool },
    /// Knob girado. `delta` é +1 ou -1 por passo.
    KnobGirado { delta: i8 },
    /// Dedo encostou ou saiu do knob.
    KnobTocado { tocado: bool },
    /// Posição do dedo na touch strip, de 0 a 255. `None` quando o dedo sai.
    Strip { posicao: Option<u8> },
    /// O aparelho foi desconectado.
    Desconectado,
}
