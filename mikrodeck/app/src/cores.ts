/**
 * As 18 cores que o aparelho aceita, e como cada uma aparece na tela.
 *
 * O nome é o mesmo usado no config.json do motor. O valor em hexadecimal é uma
 * aproximação de como o LED aparece de verdade, para o desenho na interface bater
 * com o aparelho na mesa.
 */
export const CORES = {
  apagado: "#3d3d3b",
  vermelho: "#e24b4a",
  laranja: "#d85a30",
  laranja_claro: "#ef9f27",
  amarelo_quente: "#f0b429",
  amarelo: "#e8d84a",
  lima: "#b5d33d",
  verde: "#1d9e75",
  menta: "#5dcaa5",
  ciano: "#4fd1d9",
  turquesa: "#37a9dd",
  azul: "#378add",
  ameixa: "#7f77dd",
  violeta: "#9b6fdd",
  roxo: "#a855c7",
  magenta: "#d4537e",
  fucsia: "#e0518f",
  branco: "#f1efe8",
} as const;

export type NomeCor = keyof typeof CORES;

export const NOMES_CORES = Object.keys(CORES) as NomeCor[];

/** Rótulo legível para mostrar na interface. */
export function rotuloDaCor(nome: NomeCor): string {
  return nome.replace(/_/g, " ");
}
