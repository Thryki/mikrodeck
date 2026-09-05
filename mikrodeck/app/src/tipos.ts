/** Espelho dos tipos do motor. Precisa bater com `motor/src/config.rs`. */

import type { NomeCor } from "./cores";

export type Acao =
  | { tipo: "nenhuma" }
  | { tipo: "abrir_programa"; caminho: string; argumentos: string[] }
  | { tipo: "abrir_url"; url: string }
  | { tipo: "comando"; linha: string }
  | { tipo: "atalho"; teclas: string }
  | { tipo: "midia"; tecla: TeclaMidia }
  | { tipo: "proxima_pagina" }
  | { tipo: "pagina_anterior" }
  | { tipo: "ir_para_pagina"; numero: number }
  | { tipo: "pausar_retomar" }
  | { tipo: "home_assistant"; servico: string; entidade: string }
  | {
      tipo: "http";
      url: string;
      metodo: MetodoHttp;
      cabecalhos: Record<string, string>;
      corpo: string | null;
    };

export type MetodoHttp = "get" | "post" | "put";

export type TeclaMidia =
  | "tocar_pausar"
  | "proxima"
  | "anterior"
  | "parar"
  | "aumentar_volume"
  | "diminuir_volume"
  | "mudo";

export interface Controle {
  nome: string;
  acao: Acao;
  cor: NomeCor;
  cor_pressionado: NomeCor | null;
  /** Cor enquanto o programa deste pad está aberto. Só vale para abrir programa. */
  cor_aberto?: NomeCor | null;
  /** Brilho só deste pad, de 0 a 3. Ausente usa o brilho geral. */
  brilho?: number | null;
  /** Cuidar da janela: apertar de novo alterna, segurar fecha. */
  gerenciar_janela?: boolean;
}

export interface Pagina {
  nome: string;
  /** Chave é o número do pad impresso no aparelho, de 1 a 16. */
  pads: Record<string, Controle>;
  /** Chave é o nome do botão físico, por exemplo "mute" ou "solo". */
  botoes: Record<string, Controle>;
}

/**
 * Botões físicos que dá para programar, na ordem em que aparecem no aparelho.
 * Tem que bater com a tabela BOTOES do motor.
 */
export const BOTOES_FISICOS = [
  { id: "knob", rotulo: "Knob (clique)" },
  { id: "maschine", rotulo: "MASCHINE" },
  { id: "estrela", rotulo: "Estrela" },
  { id: "busca", rotulo: "Busca" },
  { id: "volume", rotulo: "VOLUME" },
  { id: "swing", rotulo: "SWING" },
  { id: "tempo", rotulo: "TEMPO" },
  { id: "plug_in", rotulo: "PLUG-IN" },
  { id: "sampling", rotulo: "SAMPLING" },
  { id: "seta_esquerda", rotulo: "Seta esquerda" },
  { id: "seta_direita", rotulo: "Seta direita" },
  { id: "pitch", rotulo: "PITCH" },
  { id: "mod", rotulo: "MOD" },
  { id: "perform", rotulo: "PERFORM" },
  { id: "notes", rotulo: "NOTES" },
  { id: "group", rotulo: "GROUP" },
  { id: "auto", rotulo: "AUTO" },
  { id: "lock", rotulo: "LOCK" },
  { id: "note_repeat", rotulo: "NOTE REPEAT" },
  { id: "restart", rotulo: "RESTART" },
  { id: "erase", rotulo: "ERASE" },
  { id: "tap", rotulo: "TAP" },
  { id: "follow", rotulo: "FOLLOW" },
  { id: "play", rotulo: "PLAY" },
  { id: "rec", rotulo: "REC" },
  { id: "stop", rotulo: "STOP" },
  { id: "shift", rotulo: "SHIFT" },
  { id: "fixed_vel", rotulo: "FIXED VEL" },
  { id: "pad_mode", rotulo: "PAD MODE" },
  { id: "keyboard", rotulo: "KEYBOARD" },
  { id: "chords", rotulo: "CHORDS" },
  { id: "step", rotulo: "STEP" },
  { id: "scene", rotulo: "SCENE" },
  { id: "pattern", rotulo: "PATTERN" },
  { id: "events", rotulo: "EVENTS" },
  { id: "variation", rotulo: "VARIATION" },
  { id: "duplicate", rotulo: "DUPLICATE" },
  { id: "select", rotulo: "SELECT" },
  { id: "solo", rotulo: "SOLO" },
  { id: "mute", rotulo: "MUTE" },
] as const;

export type IdBotao = (typeof BOTOES_FISICOS)[number]["id"];

export function rotuloDoBotao(id: string): string {
  return BOTOES_FISICOS.find((b) => b.id === id)?.rotulo ?? id;
}

export interface Config {
  versao: number;
  paginas: Pagina[];
  botao_proxima_pagina: string;
  botao_pagina_anterior: string;
  brilho: number;
  strip: FuncaoStrip;
  home_assistant: LigacaoHomeAssistant;
  calibracao_strip: CalibracaoStrip;
  knob: FuncaoKnob;
  descanso: Descanso;
}

/** Texto que corre na tela do aparelho depois de um tempo parado. */
export interface Descanso {
  ativo: boolean;
  texto: string;
  segundos: number;
}

/** O que girar o knob faz. */
export type FuncaoKnob =
  | "nenhuma"
  | "volume"
  | "brilho_pads"
  | "paginas"
  | "rolagem";

export const ROTULOS_KNOB: Record<FuncaoKnob, string> = {
  nenhuma: "Nada",
  volume: "Volume do sistema",
  brilho_pads: "Brilho dos pads",
  paginas: "Passar de página",
  rolagem: "Rolar a página, como a roda do mouse",
};

/** Até onde o dedo chega de verdade na touch strip. */
export interface CalibracaoStrip {
  minimo: number;
  maximo: number;
}

/** Endereço e token do Home Assistant. Fica na config geral, não em cada pad. */
export interface LigacaoHomeAssistant {
  endereco: string;
  token: string;
}

/** O que a touch strip controla. */
export type FuncaoStrip = "nenhuma" | "volume" | "brilho_pads" | "paginas";

export const ROTULOS_STRIP: Record<FuncaoStrip, string> = {
  nenhuma: "Nada",
  volume: "Volume do sistema",
  brilho_pads: "Brilho dos pads",
  paginas: "Escolher página",
};

export type Situacao = "procurando" | "conectado";

/** Rótulo de cada tipo de ação, para o seletor da interface. */
export const ROTULOS_ACAO: Record<Acao["tipo"], string> = {
  nenhuma: "Nada",
  abrir_programa: "Abrir programa",
  abrir_url: "Abrir link",
  comando: "Rodar comando",
  atalho: "Atalho de teclado",
  midia: "Mídia",
  proxima_pagina: "Próxima página",
  pagina_anterior: "Página anterior",
  ir_para_pagina: "Ir para página",
  pausar_retomar: "Ligar e desligar o MikroDeck",
  home_assistant: "Home Assistant",
  http: "Requisição HTTP",
};

export const ROTULOS_MIDIA: Record<TeclaMidia, string> = {
  tocar_pausar: "Tocar e pausar",
  proxima: "Próxima faixa",
  anterior: "Faixa anterior",
  parar: "Parar",
  aumentar_volume: "Aumentar volume",
  diminuir_volume: "Diminuir volume",
  mudo: "Mudo",
};

/** Ação vazia de cada tipo, para quando o usuário troca o tipo no seletor. */
export function acaoVazia(tipo: Acao["tipo"]): Acao {
  switch (tipo) {
    case "abrir_programa":
      return { tipo, caminho: "", argumentos: [] };
    case "abrir_url":
      return { tipo, url: "" };
    case "comando":
      return { tipo, linha: "" };
    case "atalho":
      return { tipo, teclas: "" };
    case "midia":
      return { tipo, tecla: "tocar_pausar" };
    case "ir_para_pagina":
      return { tipo, numero: 1 };
    case "home_assistant":
      return { tipo, servico: "", entidade: "" };
    case "http":
      return { tipo, url: "", metodo: "post", cabecalhos: {}, corpo: null };
    default:
      return { tipo } as Acao;
  }
}

/** Resumo curto da ação, para mostrar embaixo do nome do pad. */
export function resumoDaAcao(acao: Acao): string {
  switch (acao.tipo) {
    case "abrir_programa":
      return acao.caminho || "programa não escolhido";
    case "abrir_url":
      return acao.url || "link vazio";
    case "comando":
      return acao.linha || "comando vazio";
    case "atalho":
      return acao.teclas || "atalho vazio";
    case "midia":
      return ROTULOS_MIDIA[acao.tecla];
    case "home_assistant":
      return acao.entidade
        ? `${acao.servico} em ${acao.entidade}`
        : acao.servico || "serviço não escolhido";
    case "http":
      return acao.url || "endereço vazio";
    case "ir_para_pagina":
      return `página ${acao.numero}`;
    default:
      return ROTULOS_ACAO[acao.tipo];
  }
}
