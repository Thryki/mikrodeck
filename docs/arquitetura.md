# Arquitetura do motor

## Visão geral

Dois processos lógicos dentro de um app Tauri 2:

- Motor (Rust, background, tray): sempre ligado. Fala HID, decide, executa, pinta.
- UI (React + Tailwind, janela Tauri): abre sob demanda, conecta ao motor por IPC, fecha sem parar nada.

## Módulos

### hid
- Crate `hidapi`.
- `open()` localiza o device por VID/PID da NI (ver pymikro para os IDs).
- Thread de leitura: `read()` bloqueante, converte bytes em `InputEvent` (PadDown{n, pressure}, PadUp, Button, Encoder{delta}, EncoderPush, Strip{x}).
- Thread de escrita: tick fixo (30 a 60 Hz). Mantém um `LedFrame` completo; só escreve se `dirty`.
- Windows: prefixar report ID 0 no buffer de saída.
- Nada fora deste módulo toca no HID.

### state
- `Profile` -> `Page[]` -> `Control[]`.
- Cada `Control` tem `states: {idle, pressed, active, disabled}` com cor e ação.
- `current_page`, `current_profile`.
- Recebe `InputEvent` e `WatcherEvent`, devolve `LedFrame` + `ActionRequest`.

### actions
- Enum `Action`: OpenApp(path), Hotkey(keys), Script(path, args), Shell(cmd), Url(url), Media(Play|Pause|Next|Prev|VolUp|VolDown|Mute), PageNext, PagePrev, PageGoto(n), Profile(name).
- Executa em thread própria; nunca bloqueia o loop de entrada.

### watchers
- `ProcessWatcher`: lista de processos a cada ~500 ms; emite Started/Stopped por nome.
- `FocusWatcher`: janela em foco.
- `AudioWatcher`: volume e mute.
- Extensível: cada watcher é um trait com `poll()` ou callback.

### render
- Framebuffer da tela (dimensões a confirmar no spike).
- Texto simples com fonte bitmap; barra de volume; nome da página.
- Envia só quando o buffer muda.

### ipc
- Comandos Tauri: get_profiles, save_profile, set_page, test_control (envia config para o aparelho sem salvar), get_device_status.
- Eventos Tauri: device_connected, device_disconnected, input_event (para a UI mostrar o pad piscando ao vivo).

## Latência

Caminho crítico: hid.read -> state -> actions.spawn. Tudo em Rust, sem UI, sem disco. Meta: < 5 ms do evento à ação.

LEDs: state marca dirty -> próximo tick escreve o frame inteiro. Nunca escrever LED por LED.

## Persistência

- `~/.mikrodeck/config.json` (ou pasta de app data do SO).
- Formato versionado (`"version": 1`) para migrações futuras.

## Boot

- Motor sobe com o sistema (autostart).
- Animação de boot configurável (varredura de cor nos pads, 1 a 2 s).
- Se o device não estiver conectado, o motor fica em espera e reconecta sozinho.
