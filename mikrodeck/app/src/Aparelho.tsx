/**
 * Desenho do Maschine Mikro MK3.
 *
 * Não é uma imagem: é SVG gerado, para os pads e os botões serem clicáveis,
 * mostrarem a configuração real, e reagirem quando o controle físico é apertado.
 *
 * As proporções seguem `design/mikro-mk3.svg`, que passou pelo loop de revisão
 * contra a foto do aparelho.
 */

import { useRef, useState } from "react";
import { CORES, type NomeCor } from "./cores";

/** A letra impressa no canto de cada pad, como no aparelho: A a P de 13 a 4. */
const LETRAS_PADS: Record<number, string> = {
  13: "A", 14: "B", 15: "C", 16: "D",
  9: "E", 10: "F", 11: "G", 12: "H",
  5: "I", 6: "J", 7: "K", 8: "L",
  1: "M", 2: "N", 3: "O", 4: "P",
};

/** Números dos pads como aparecem no aparelho, de cima para baixo. */
const LINHAS_PADS = [
  [13, 14, 15, 16],
  [9, 10, 11, 12],
  [5, 6, 7, 8],
  [1, 2, 3, 4],
];

/** Onde cada botão fica no desenho. Bate com o SVG revisado contra a foto. */
const BOTOES: {
  id: string;
  rotulo: string;
  x: number;
  y: number;
  w: number;
  h: number;
  fonte?: number;
  /** Segunda linha, o que o botão faz no Maschine. Só decoração. */
  sub?: string;
  /** Desenha um ícone no lugar do texto. */
  icone?: NomeIcone;
}[] = [
  // Coluna da esquerda. Estes três usam ícone desenhado, não texto.
  { id: "maschine", rotulo: "", icone: "ligar", x: 27, y: 30, w: 30, h: 29 },
  { id: "estrela", rotulo: "", icone: "estrela", x: 27, y: 68, w: 30, h: 29 },
  { id: "busca", rotulo: "", icone: "busca", x: 27, y: 106, w: 30, h: 29 },
  // Coluna VOLUME / SWING / TEMPO
  { id: "volume", rotulo: "VOLUME", x: 197, y: 34, w: 74, h: 32, sub: "[Velocity]" },
  { id: "swing", rotulo: "SWING", x: 197, y: 74, w: 74, h: 32, sub: "[Position]" },
  { id: "tempo", rotulo: "TEMPO", x: 197, y: 114, w: 74, h: 32, sub: "[Tune]" },
  // Coluna PLUG-IN / SAMPLING / setas
  { id: "plug_in", rotulo: "PLUG-IN", x: 279, y: 34, w: 76, h: 32, sub: "Macro" },
  { id: "sampling", rotulo: "SAMPLING", x: 279, y: 74, w: 76, h: 32 },
  { id: "seta_esquerda", rotulo: "", icone: "seta_esq", x: 279, y: 114, w: 35, h: 32 },
  { id: "seta_direita", rotulo: "", icone: "seta_dir", x: 320, y: 114, w: 35, h: 32 },
  // Linha PITCH
  { id: "pitch", rotulo: "PITCH", x: 23, y: 204, w: 77, h: 28 },
  { id: "mod", rotulo: "MOD", x: 107, y: 204, w: 77, h: 28 },
  { id: "perform", rotulo: "PERFORM", x: 191, y: 204, w: 77, h: 28, sub: "FX Select" },
  { id: "notes", rotulo: "NOTES", x: 275, y: 204, w: 77, h: 28 },
  // Linha GROUP: na foto ela tem quase o dobro da altura da fileira PITCH
  { id: "group", rotulo: "GROUP", x: 23, y: 334, w: 77, h: 50 },
  { id: "auto", rotulo: "AUTO", x: 107, y: 334, w: 77, h: 50 },
  { id: "lock", rotulo: "LOCK", x: 191, y: 334, w: 77, h: 50 },
  { id: "note_repeat", rotulo: "NOTE REPEAT", x: 275, y: 334, w: 77, h: 50, fonte: 9, sub: "Arp" },
  // Linha RESTART, baixa, colada na linha PLAY
  { id: "restart", rotulo: "RESTART", x: 23, y: 427, w: 77, h: 28, sub: "Loop" },
  { id: "erase", rotulo: "ERASE", x: 107, y: 427, w: 77, h: 28, sub: "Replace" },
  { id: "tap", rotulo: "TAP", x: 191, y: 427, w: 77, h: 28, sub: "Metro" },
  { id: "follow", rotulo: "FOLLOW", x: 275, y: 427, w: 77, h: 28, sub: "Grid" },
  // Linha PLAY, alta como a GROUP, fechando na mesma base da grade de pads
  { id: "play", rotulo: "PLAY", x: 23, y: 459, w: 77, h: 48 },
  { id: "rec", rotulo: "REC", x: 107, y: 459, w: 77, h: 48, sub: "Count-In" },
  { id: "stop", rotulo: "STOP", x: 191, y: 459, w: 77, h: 48 },
  { id: "shift", rotulo: "SHIFT", x: 275, y: 459, w: 77, h: 48 },
  // Coluna central: dois botões por fileira de pads, do topo ao pé da grade
  { id: "fixed_vel", rotulo: "FIXED VEL", x: 405, y: 30, w: 79, h: 33, fonte: 9, sub: "16 Vel" },
  { id: "scene", rotulo: "SCENE", x: 405, y: 82, w: 79, h: 47, sub: "Section" },
  { id: "pattern", rotulo: "PATTERN", x: 405, y: 136, w: 79, h: 47 },
  { id: "events", rotulo: "EVENTS", x: 405, y: 190, w: 79, h: 47 },
  { id: "variation", rotulo: "VARIATION", x: 405, y: 244, w: 79, h: 47, fonte: 9, sub: "Navigate" },
  { id: "duplicate", rotulo: "DUPLICATE", x: 405, y: 298, w: 79, h: 47, fonte: 9, sub: "Double" },
  { id: "select", rotulo: "SELECT", x: 405, y: 352, w: 79, h: 47 },
  { id: "solo", rotulo: "SOLO", x: 405, y: 406, w: 79, h: 47 },
  { id: "mute", rotulo: "MUTE", x: 405, y: 460, w: 79, h: 47, sub: "Choke" },
  // Botões de modo, em cima dos pads e alinhados com as colunas deles
  { id: "pad_mode", rotulo: "PAD MODE", x: 501, y: 30, w: 105, h: 33 },
  { id: "keyboard", rotulo: "KEYBOARD", x: 613, y: 30, w: 105, h: 33 },
  { id: "chords", rotulo: "CHORDS", x: 725, y: 30, w: 105, h: 33 },
  { id: "step", rotulo: "STEP", x: 837, y: 30, w: 105, h: 33 },
];

type NomeIcone = "ligar" | "estrela" | "busca" | "seta_esq" | "seta_dir";

/**
 * Ícones desenhados em SVG, não emoji nem glifo de fonte.
 * Cada um é desenhado numa caixa de 16 por 16 centrada no botão.
 */
function Icone({ nome, x, y, cor }: { nome: NomeIcone; x: number; y: number; cor: string }) {
  const comum = { fill: "none", stroke: cor, strokeWidth: 1.6, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };
  return (
    <g transform={`translate(${x - 8}, ${y - 8})`} pointerEvents="none">
      {nome === "ligar" && (
        <>
          <circle cx="8" cy="8" r="6.5" {...comum} />
          <circle cx="8" cy="8" r="2.5" fill={cor} stroke="none" />
        </>
      )}
      {nome === "estrela" && (
        <path
          d="M8 1.8l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.6l-3.8 2 .7-4.3-3.1-3 4.3-.6z"
          fill={cor}
          stroke="none"
        />
      )}
      {nome === "busca" && (
        <>
          {/* Ícone "browse" da NI: lupa com três traços saindo à esquerda. */}
          <circle cx="9" cy="7" r="4.3" {...comum} />
          <line x1="12.1" y1="10.1" x2="14.6" y2="12.6" {...comum} />
          <line x1="1.4" y1="4.4" x2="4" y2="4.4" {...comum} />
          <line x1="1.4" y1="7" x2="4" y2="7" {...comum} />
          <line x1="1.4" y1="9.6" x2="4" y2="9.6" {...comum} />
        </>
      )}
      {nome === "seta_esq" && <path d="M10 3L5 8l5 5z" fill={cor} stroke="none" />}
      {nome === "seta_dir" && <path d="M6 3l5 5-5 5z" fill={cor} stroke="none" />}
    </g>
  );
}

export type Selecao =
  | { tipo: "pad"; pad: number }
  | { tipo: "botao"; id: string };

interface Props {
  cores: Record<number, NomeCor>;
  nomes: Record<number, string>;
  /** Botões que têm ação nesta página. */
  botoesProgramados: Record<string, string>;
  /** Pads e botões apertados agora no aparelho físico. */
  padsApertados: Set<number>;
  botoesApertados: Set<string>;
  selecao: Selecao | null;
  onSelecionar: (s: Selecao) => void;
  brilho: number;
  /** O que está escrito na tela do aparelho agora. */
  tela: { titulo: string; sub?: string };
  onClicarTela?: () => void;
  /** Arrastar um controle em cima de outro leva a configuração junto. */
  onMover?: (origem: Selecao, destino: Selecao) => void;
  /** Botões que estão reservados para trocar de página e não aceitam programação. */
  reservados?: string[];
}

/** Identidade de um controle em texto, para marcar o alvo do arrasto no DOM. */
function chave(s: Selecao): string {
  return s.tipo === "pad" ? `pad:${s.pad}` : `botao:${s.id}`;
}

function daChave(valor: string): Selecao | null {
  const [tipo, resto] = valor.split(":");
  if (tipo === "pad") return { tipo: "pad", pad: Number(resto) };
  if (tipo === "botao") return { tipo: "botao", id: resto };
  return null;
}

export function Aparelho({
  cores,
  nomes,
  botoesProgramados,
  padsApertados,
  botoesApertados,
  selecao,
  onSelecionar,
  brilho,
  tela,
  onClicarTela,
  onMover,
  reservados = [],
}: Props) {
  // Arrastar e soltar. Guardo o começo num ref para não redesenhar a cada pixel,
  // e só entro em modo de arrasto depois de andar o bastante para não confundir
  // com um clique.
  // `arrastou` mora no ref junto com o resto: estado do React não atualiza a
  // tempo dentro da mesma sequência de eventos, e aí o soltar virava clique.
  const inicio = useRef<{
    origem: Selecao;
    x: number;
    y: number;
    arrastou: boolean;
  } | null>(null);
  const [arrastando, setArrastando] = useState<Selecao | null>(null);
  const [alvo, setAlvo] = useState<string | null>(null);

  /** Qual controle está debaixo do ponteiro agora. */
  function alvoEm(x: number, y: number): string | null {
    const elemento = document.elementFromPoint(x, y);
    return elemento?.closest("[data-alvo]")?.getAttribute("data-alvo") ?? null;
  }

  function aoDescer(e: React.PointerEvent, origem: Selecao) {
    inicio.current = { origem, x: e.clientX, y: e.clientY, arrastou: false };
  }

  function aoMover(e: React.PointerEvent) {
    const comeco = inicio.current;
    if (!comeco || !onMover) return;
    const andou = Math.hypot(e.clientX - comeco.x, e.clientY - comeco.y);
    if (!comeco.arrastou && andou < 6) return;
    if (!comeco.arrastou) {
      comeco.arrastou = true;
      setArrastando(comeco.origem);
    }
    setAlvo(alvoEm(e.clientX, e.clientY));
  }

  function aoSubir(e: React.PointerEvent) {
    const comeco = inicio.current;
    inicio.current = null;
    if (!comeco) return;
    if (!comeco.arrastou) {
      // Não andou o bastante: foi um clique.
      onSelecionar(comeco.origem);
      return;
    }
    const destino = alvoEm(e.clientX, e.clientY);
    setArrastando(null);
    setAlvo(null);
    if (!destino || !onMover) return;
    const alvoSelecao = daChave(destino);
    if (!alvoSelecao || destino === chave(comeco.origem)) return;
    // Pad só troca com pad, e botão só com botão: a cor de um não cabe no outro.
    if (alvoSelecao.tipo !== comeco.origem.tipo) return;
    // Botão reservado para trocar de página não é origem nem destino.
    if (
      (alvoSelecao.tipo === "botao" && reservados.includes(alvoSelecao.id)) ||
      (comeco.origem.tipo === "botao" && reservados.includes(comeco.origem.id))
    ) {
      return;
    }
    onMover(comeco.origem, alvoSelecao);
  }

  /** Realce de quem está sendo arrastado e de quem vai receber. */
  function estiloArrasto(s: Selecao) {
    const eu = chave(s);
    if (arrastando && chave(arrastando) === eu) return { opacity: 0.35 };
    if (arrastando && alvo === eu && arrastando.tipo === s.tipo) {
      return { opacity: 1 };
    }
    return undefined;
  }

  function realce(s: Selecao): boolean {
    return (
      arrastando !== null &&
      alvo === chave(s) &&
      arrastando.tipo === s.tipo &&
      chave(arrastando) !== chave(s)
    );
  }
  // O brilho do aparelho vai de 0 a 3; na tela vira opacidade, para o desenho
  // acompanhar o que a pessoa está vendo na mesa.
  const opacidade = 0.45 + 0.55 * (Math.min(brilho, 3) / 3);

  return (
    // Com viewBox e preserveAspectRatio padrão, o desenho cresce até caber na
    // caixa inteira sem distorcer, seja a janela larga ou alta.
    <svg
      viewBox="0 0 960 536"
      className="h-full max-h-full w-full max-w-full select-none touch-none"
      onPointerMove={aoMover}
      onPointerUp={aoSubir}
      onPointerLeave={() => {
        inicio.current = null;
        setArrastando(null);
        setAlvo(null);
      }}
      role="img"
      aria-label="Maschine Mikro MK3 com o MikroDeck"
    >
      <rect x="0" y="0" width="960" height="536" rx="18" fill="#2C2C2A" />

      {/* Tela do aparelho, com o que ela mostra de verdade. Clicável para configurar. */}
      <g
        onClick={onClicarTela}
        className={onClicarTela ? "cursor-pointer" : undefined}
        role={onClicarTela ? "button" : undefined}
        aria-label="Tela do aparelho"
      >
        <rect x="83" y="34" width="84" height="27" rx="2" fill="#0b0b0b" stroke="#1a1a19" />
        <text
          x="125"
          y={tela.sub ? 47 : 51}
          textAnchor="middle"
          fontSize="11"
          fill="#8ad7f0"
          fontFamily="ui-monospace, Consolas, monospace"
        >
          {tela.titulo.length > 15 ? tela.titulo.slice(0, 14) + "…" : tela.titulo}
        </text>
        {tela.sub && (
          <text
            x="125"
            y="57"
            textAnchor="middle"
            fontSize="10"
            fill="#5fa8bf"
            fontFamily="ui-monospace, Consolas, monospace"
          >
            {tela.sub}
          </text>
        )}
      </g>

      {/* Knob. Na foto é um cilindro escuro de metal, com a borda canelada e um
          topo liso mais claro. O clique dele é programável, então é clicável aqui
          também; não tem LED, por isso só ganha contorno quando selecionado. */}
      <g
        className="cursor-pointer"
        data-alvo="botao:knob"
        onPointerDown={(e) => aoDescer(e, { tipo: "botao", id: "knob" })}
      >
        <circle cx="114" cy="112" r="31" fill="#141413" />
        <circle cx="114" cy="109" r="30" fill="#2a2a28" />
        {/* Canelado da borda: um tracinho a cada 12 graus. */}
        {Array.from({ length: 30 }, (_, i) => {
          const a = (i * Math.PI * 2) / 30;
          return (
            <line
              key={i}
              x1={114 + Math.cos(a) * 23}
              y1={109 + Math.sin(a) * 23}
              x2={114 + Math.cos(a) * 29.5}
              y2={109 + Math.sin(a) * 29.5}
              stroke="#4a4a47"
              strokeWidth="1.1"
            />
          );
        })}
        <circle
          cx="114"
          cy="109"
          r="22"
          fill="#3a3a37"
          stroke={
            selecao?.tipo === "botao" && selecao.id === "knob"
              ? "#38BDF8"
              : botoesProgramados["knob"]
                ? "#F1EFE8"
                : "none"
          }
          strokeWidth="2.5"
        />
        <circle cx="114" cy="107" r="18" fill="#4d4d49" />
      </g>

      {/* Logo, medido contra a foto: o anel alinha com a coluna de botões da
          esquerda e o texto segue na mesma linha de base. */}
      <g fill="none" stroke="#F1EFE8" strokeWidth="2.4">
        <circle cx="31" cy="170" r="8.5" />
      </g>
      <circle cx="31" cy="170" r="3.4" fill="#F1EFE8" />
      <text x="45" y="178" fontSize="19" fill="#F1EFE8" fontWeight="600" letterSpacing="1.4">
        MIKRODECK
      </text>

      {/* Touch strip. Na foto os 25 LEDs ficam numa fileira ACIMA da faixa, e a
          faixa em si é uma canaleta rebaixada, sem luz nenhuma. */}
      {Array.from({ length: 25 }, (_, i) => (
        <circle key={i} cx={41 + i * 12.4} cy={250} r="2.2" fill="#378ADD" opacity="0.4" />
      ))}
      <rect x="23" y="258" width="329" height="39" rx="5" fill="#242422" stroke="#1a1a19" />
      <rect x="27" y="262" width="321" height="31" rx="4" fill="#2f2f2d" />

      {/* Botões, todos clicáveis */}
      {BOTOES.map((b) => {
        const reservado = reservados.includes(b.id);
        const programado = reservado ? undefined : botoesProgramados[b.id];
        const apertado = botoesApertados.has(b.id);
        const escolhido = selecao?.tipo === "botao" && selecao.id === b.id;
        return (
          <g
            key={b.id}
            data-alvo={`botao:${b.id}`}
            onPointerDown={(e) => aoDescer(e, { tipo: "botao", id: b.id })}
            style={estiloArrasto({ tipo: "botao", id: b.id })}
            className="cursor-pointer"
            role="button"
            aria-label={`Botão ${b.rotulo}${programado ? `: ${programado}` : ""}`}
          >
            <rect
              x={b.x}
              y={b.y}
              width={b.w}
              height={b.h}
              rx="5"
              fill={
                apertado
                  ? "#F1EFE8"
                  : reservado
                    ? "#46464a"
                    : programado
                      ? "#5a5a56"
                      : "#3d3d3b"
              }
              stroke={
                realce({ tipo: "botao", id: b.id })
                  ? "#38BDF8"
                  : escolhido
                    ? "#FFFFFF"
                    : "#232321"
              }
              strokeWidth={
                realce({ tipo: "botao", id: b.id }) || escolhido ? 2.5 : 1.5
              }
            />
            {/* Acentos que o aparelho tem impressos e que ajudam a achar o botão. */}
            {!programado && b.id === "play" && (
              <path
                d={`M${b.x + 7} ${b.y + 8} l6 3.5 -6 3.5 z`}
                fill="#5EE39B"
              />
            )}
            {!programado && b.id === "rec" && (
              <circle cx={b.x + 10} cy={b.y + 11.5} r="3.4" fill="#F0555A" />
            )}
            {!programado && b.id === "stop" && (
              <rect x={b.x + 7} y={b.y + 8.5} width="6" height="6" fill="#c8c7c2" />
            )}
            {/* Botão com ícone impresso mantém o ícone mesmo programado: o nome
                não cabe aqui e vai para o painel e para a tela do aparelho. Ele
                só fica mais claro, como os outros botões programados. */}
            {b.icone ? (
              <Icone
                nome={b.icone}
                x={b.x + b.w / 2}
                y={b.y + b.h / 2}
                cor={apertado ? "#1f1f1e" : programado ? "#F1EFE8" : "#c8c7c2"}
              />
            ) : (
              <>
                <text
                  x={b.x + (!programado && ["play", "rec", "stop"].includes(b.id) ? 18 : 7)}
                  y={b.y + 14}
                  fontSize={b.fonte ?? 10}
                  fill={
                    apertado
                      ? "#1f1f1e"
                      : programado
                        ? "#F1EFE8"
                        : b.id === "play"
                          ? "#5EE39B"
                          : b.id === "rec"
                            ? "#F0555A"
                            : "#9a9994"
                  }
                  fontWeight={programado ? 600 : 500}
                >
                  {programado
                    ? programado.length > 13
                      ? programado.slice(0, 12) + "…"
                      : programado
                    : b.rotulo}
                </text>
                {/* A segunda linha é o que o botão faz no Maschine. Some quando
                    o botão tem função no MikroDeck, para não confundir. */}
                {b.sub && !programado && (
                  <text
                    x={b.x + 7}
                    y={b.y + 25}
                    fontSize="8.5"
                    fill={apertado ? "#4a4a48" : "#6f6e6a"}
                  >
                    {b.sub}
                  </text>
                )}
              </>
            )}
          </g>
        );
      })}

      {/* A grade de pads */}
      {LINHAS_PADS.map((linha, l) =>
        linha.map((pad, c) => {
          const x = 501 + c * 112;
          const y = 82 + l * 108;
          const cor = cores[pad];
          const apertado = padsApertados.has(pad);
          const preenchimento = apertado
            ? CORES.branco
            : cor
              ? CORES[cor]
              : "#3d3d3b";
          const nome = nomes[pad];
          const escolhido = selecao?.tipo === "pad" && selecao.pad === pad;
          return (
            <g
              key={pad}
              data-alvo={`pad:${pad}`}
              onPointerDown={(e) => aoDescer(e, { tipo: "pad", pad })}
              style={estiloArrasto({ tipo: "pad", pad })}
              className="cursor-pointer"
              role="button"
              aria-label={`Pad ${pad}${nome ? `: ${nome}` : ""}`}
            >
              <rect
                x={x}
                y={y}
                width="105"
                height="100"
                rx="7"
                fill={preenchimento}
                fillOpacity={apertado || !cor ? 1 : opacidade}
                stroke="#1f1f1e"
                strokeWidth="1.5"
              />
              {(escolhido || realce({ tipo: "pad", pad })) && (
                <rect
                  x={x + 2}
                  y={y + 2}
                  width="101"
                  height="96"
                  rx="6"
                  fill="none"
                  stroke={realce({ tipo: "pad", pad }) ? "#38BDF8" : "#FFFFFF"}
                  strokeWidth="3"
                />
              )}
              <text
                x={x + 8}
                y={y + 18}
                fontSize="12"
                fontWeight="600"
                fill={cor && cor !== "apagado" ? "#1f1f1e" : "#888780"}
              >
                {pad}
              </text>
              {/* A letra impressa no canto direito do pad, como no aparelho. */}
              <text
                x={x + 97}
                y={y + 18}
                textAnchor="end"
                fontSize="10"
                fill={cor && cor !== "apagado" ? "#1f1f1e" : "#6f6e6a"}
                opacity="0.75"
              >
                {LETRAS_PADS[pad]}
              </text>
              {nome && (
                <text
                  x={x + 8}
                  y={y + 34}
                  fontSize="11"
                  fill={cor && cor !== "apagado" ? "#1f1f1e" : "#888780"}
                >
                  {nome.length > 13 ? nome.slice(0, 12) + "…" : nome}
                </text>
              )}
            </g>
          );
        }),
      )}
    </svg>
  );
}
