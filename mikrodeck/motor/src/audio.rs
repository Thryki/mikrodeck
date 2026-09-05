//! Volume do sistema, via Core Audio do Windows.
//!
//! Serve para duas coisas que o Davi pediu: a touch strip funcionar como controle
//! de volume de verdade, e a tela do aparelho mostrar a barra com o valor real.
//! Sem ler o volume do sistema, a barra mentiria assim que alguém mexesse por fora.

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};

/// Acesso ao volume principal do sistema.
pub struct Volume {
    endpoint: IAudioEndpointVolume,
}

impl Volume {
    /// Abre o dispositivo de saída padrão. Devolve `None` se não der,
    /// para o motor seguir funcionando sem volume em vez de morrer.
    pub fn abrir() -> Option<Self> {
        unsafe {
            // O COM precisa estar iniciado nesta thread. Se já estiver, o erro é
            // esperado e pode ser ignorado.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerador: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let dispositivo = enumerador.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            let endpoint: IAudioEndpointVolume = dispositivo.Activate(CLSCTX_ALL, None).ok()?;
            Some(Self { endpoint })
        }
    }

    /// Volume atual, de 0.0 a 1.0.
    pub fn ler(&self) -> Option<f32> {
        unsafe { self.endpoint.GetMasterVolumeLevelScalar().ok() }
    }

    /// Define o volume, de 0.0 a 1.0.
    pub fn definir(&self, valor: f32) -> bool {
        unsafe {
            self.endpoint
                .SetMasterVolumeLevelScalar(valor.clamp(0.0, 1.0), std::ptr::null())
                .is_ok()
        }
    }

    pub fn esta_mudo(&self) -> Option<bool> {
        unsafe { self.endpoint.GetMute().ok().map(|m| m.as_bool()) }
    }

    pub fn definir_mudo(&self, mudo: bool) -> bool {
        unsafe { self.endpoint.SetMute(mudo, std::ptr::null()).is_ok() }
    }
}

// O ponteiro COM é usado só pela thread do motor, mas o `Servico` guarda o objeto
// dentro de um `Arc`. Como toda chamada passa pelo mesmo laço, é seguro mover.
unsafe impl Send for Volume {}
unsafe impl Sync for Volume {}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn abrir_e_ler_nao_entram_em_panico() {
        // Em máquina sem placa de som, `abrir` devolve None e o teste passa mesmo assim.
        if let Some(v) = Volume::abrir() {
            let atual = v.ler();
            assert!(atual.is_none() || (0.0..=1.0).contains(&atual.unwrap()));
            let _ = v.esta_mudo();
        }
    }
}
