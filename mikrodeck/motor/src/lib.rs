//! Motor do MikroDeck.
//!
//! Toda a lógica de falar com o Maschine Mikro MK3 e transformar ele num Stream Deck.
//! Existe como biblioteca para o app Tauri e o binário headless usarem o mesmo código.
//!
//! Ver `docs/arquitetura.md` e `docs/spike-hid.md`.

pub mod acoes;
pub mod audio;
pub mod config;
pub mod estado;
pub mod hid;
pub mod janelas;
pub mod luz;
pub mod prontas;
pub mod rede;
pub mod render;
pub mod servico;
pub mod vigias;

pub use acoes::{Acao, TeclaMidia};
pub use config::{Config, Controle, Pagina};
pub use estado::{Estado, Reacao};
pub use hid::{Aparelho, BrilhoBotao, Cor, Evento};
