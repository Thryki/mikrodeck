/**
 * Ponte com o motor.
 *
 * Dentro do app Tauri, chama o backend de verdade. Aberto num navegador comum
 * (`npm run dev` em http://localhost:1420), cai num modo de demonstração com
 * dados falsos. Isso permite trabalhar no visual e testar a interface sem o
 * aparelho, inclusive de forma automatizada.
 */

import type { Config, Microfone, Situacao } from "./tipos";

export const DENTRO_DO_TAURI = "__TAURI_INTERNALS__" in window;

/** Config de mentira, só para o modo de demonstração no navegador. */
function configDemo(): Config {
  return {
    versao: 1,
    paginas: [
      {
        nome: "Apps",
        pads: {
          "1": { nome: "Copiar", acao: { tipo: "atalho", teclas: "ctrl+c" }, cor: "violeta", cor_pressionado: null },
          "2": { nome: "Colar", acao: { tipo: "atalho", teclas: "ctrl+v" }, cor: "violeta", cor_pressionado: null },
          "9": { nome: "Claude", acao: { tipo: "abrir_url", url: "https://claude.ai" }, cor: "laranja", cor_pressionado: null },
          "13": { nome: "Chrome", acao: { tipo: "abrir_programa", caminho: "chrome", argumentos: [] }, cor: "azul", cor_pressionado: null },
          "14": { nome: "Explorador", acao: { tipo: "abrir_programa", caminho: "explorer", argumentos: [] }, cor: "amarelo_quente", cor_pressionado: null },
          "15": { nome: "Bloco de notas", acao: { tipo: "abrir_programa", caminho: "notepad", argumentos: [] }, cor: "verde", cor_pressionado: null },
          "16": { nome: "Terminal", acao: { tipo: "abrir_programa", caminho: "wt", argumentos: [] }, cor: "turquesa", cor_pressionado: null },
        },
        botoes: {
          mute: { nome: "Mudo", acao: { tipo: "midia", tecla: "mudo" }, cor: "azul", cor_pressionado: null },
        },
      },
      { nome: "Mídia", pads: {}, botoes: {} },
    ],
    botao_proxima_pagina: "seta_direita",
    botao_pagina_anterior: "seta_esquerda",
    brilho: 2,
    strip: "volume",
    home_assistant: { endereco: "", token: "" },
    calibracao_strip: { minimo: 0, maximo: 255 },
    knob: "paginas",
    descanso: {
      ativo: true,
      texto: "MikroDeck",
      segundos: 90,
      luz: { modo: "respiracao", ritmo: "medio", cor: "auto" },
    },
    ao_apertar: "eco",
  };
}

let demo: Config | null = null;

export async function lerConfig(): Promise<Config> {
  if (!DENTRO_DO_TAURI) {
    demo ??= configDemo();
    return structuredClone(demo);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Config>("ler_config");
}

export async function salvarConfig(config: Config): Promise<void> {
  if (!DENTRO_DO_TAURI) {
    demo = structuredClone(config);
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("salvar_config", { config });
}

export async function lerSituacao(): Promise<Situacao> {
  if (!DENTRO_DO_TAURI) return "conectado";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Situacao>("ler_situacao");
}

export async function irParaPagina(numero: number): Promise<void> {
  if (!DENTRO_DO_TAURI) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("ir_para_pagina", { numero });
}

export interface Diagnostico {
  aparelho_conectado: boolean;
  caminho_config: string;
  paginas: number;
  pads_configurados: number;
  botoes_configurados: number;
  versao_motor: string;
}

export async function lerDiagnostico(): Promise<Diagnostico> {
  if (!DENTRO_DO_TAURI) {
    return {
      aparelho_conectado: true,
      caminho_config: "modo de demonstração",
      paginas: 2,
      pads_configurados: 7,
      botoes_configurados: 1,
      versao_motor: "demo",
    };
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Diagnostico>("diagnostico");
}

export async function restaurarPadrao(): Promise<Config> {
  if (!DENTRO_DO_TAURI) {
    demo = configDemo();
    return structuredClone(demo);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Config>("restaurar_padrao");
}

/** Força o descanso no aparelho agora, para ver a luz sem esperar. */
export async function previsualizarDescanso(): Promise<void> {
  if (!DENTRO_DO_TAURI) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("previsualizar_descanso");
}

export async function testarLeds(): Promise<void> {
  if (!DENTRO_DO_TAURI) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("testar_leds");
}

export async function lerIniciaComOSistema(): Promise<boolean> {
  if (!DENTRO_DO_TAURI) return false;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("ler_inicia_com_o_sistema");
}

export async function definirIniciaComOSistema(ligado: boolean): Promise<void> {
  if (!DENTRO_DO_TAURI) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("definir_inicia_com_o_sistema", { ligado });
}

export interface PaginaPronta {
  id: string;
  nome: string;
  descricao: string;
}

export async function paginasProntas(): Promise<PaginaPronta[]> {
  if (!DENTRO_DO_TAURI) {
    return [
      { id: "spotify", nome: "Spotify", descricao: "Tocar, pular e mexer no volume." },
      { id: "claude", nome: "Claude", descricao: "Abrir o Claude e começar um chat novo." },
      { id: "casa", nome: "Casa", descricao: "Exemplos de automação pelo Home Assistant." },
      { id: "trabalho", nome: "Trabalho", descricao: "Copiar, colar, print e áreas de trabalho." },
    ];
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PaginaPronta[]>("paginas_prontas");
}

export async function adicionarPaginaPronta(id: string): Promise<Config> {
  if (!DENTRO_DO_TAURI) {
    demo ??= configDemo();
    demo.paginas.push({ nome: id, pads: {}, botoes: {} });
    return structuredClone(demo);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Config>("adicionar_pagina_pronta", { id });
}

/** Onde está o servidor MCP, para a pessoa copiar o comando de ligação. */
export async function caminhoDoMcp(): Promise<string | null> {
  if (!DENTRO_DO_TAURI) return "C:\Program Files\MikroDeck\mikrodeck-mcp.exe";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string | null>("caminho_do_mcp");
}

export async function abrirPastaConfig(): Promise<void> {
  if (!DENTRO_DO_TAURI) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("abrir_pasta_config");
}

/** Escolhe um programa no disco. No navegador, devolve um caminho de mentira. */
export async function escolherPrograma(): Promise<string | null> {
  if (!DENTRO_DO_TAURI) {
    return "C:\\Program Files\\Exemplo\\exemplo.exe";
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const caminho = await open({
    multiple: false,
    filters: [{ name: "Programas", extensions: ["exe", "lnk", "bat", "cmd"] }],
  });
  return typeof caminho === "string" ? caminho : null;
}

/** Escolhe um arquivo de audio. No navegador, devolve um caminho de mentira. */
export async function escolherSample(): Promise<string | null> {
  if (!DENTRO_DO_TAURI) {
    return "C:\Users\exemplo\Musica\bumbo.wav";
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const caminho = await open({
    multiple: false,
    filters: [
      {
        name: "Áudio",
        extensions: ["wav", "mp3", "flac", "ogg", "m4a", "aac"],
      },
    ],
  });
  return typeof caminho === "string" ? caminho : null;
}

/** Os microfones que o Windows enxerga. */
export async function microfones(): Promise<Microfone[]> {
  if (!DENTRO_DO_TAURI) {
    return [{ nome: "Microfone de exemplo", padrao: true }];
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Microfone[]>("microfones");
}

/** Começa a gravar. Devolve o caminho do arquivo que vai ser escrito. */
export async function gravarSample(
  nome: string,
  microfone: string | null,
  segundos: number,
): Promise<string> {
  if (!DENTRO_DO_TAURI) return `C:\demo\${nome}.wav`;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("gravar_sample", { nome, microfone, segundos });
}

/** Para a gravação e devolve o caminho do arquivo pronto. */
export async function pararGravacao(): Promise<string> {
  if (!DENTRO_DO_TAURI) return "C:\demo\gravado.wav";
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("parar_gravacao");
}

/** Quanto já foi gravado, em segundos. */
export async function tempoDeGravacao(): Promise<number> {
  if (!DENTRO_DO_TAURI) return 0;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<number>("tempo_de_gravacao");
}

/** Toca o sample aqui no computador, para conferir. Devolve a duração. */
export async function testarSample(
  caminho: string,
  volume: number,
): Promise<number> {
  if (!DENTRO_DO_TAURI) return 1;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<number>("testar_sample", { caminho, volume });
}

type Remover = () => void;

/** Assina um evento do motor. No navegador, não faz nada. */
export async function escutar<T>(
  nome: string,
  aoReceber: (dado: T) => void,
): Promise<Remover> {
  if (!DENTRO_DO_TAURI) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(nome, (e) => aoReceber(e.payload));
}
