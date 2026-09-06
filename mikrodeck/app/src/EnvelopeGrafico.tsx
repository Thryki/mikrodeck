/**
 * O envelope desenhado, como num plugin: a curva com alças que se arrastam.
 *
 * A curva é a mesma conta do motor (`som/envelope.rs`), então o que se vê é o
 * que se ouve. Arrastar um nó na horizontal muda a duração do estágio; o nó da
 * sustentação também sobe e desce.
 */

import { useEffect, useRef, useState } from "react";
import { curvar, type Envelope } from "./tipos";

interface Props {
  envelope: Envelope;
  onMudar: (e: Envelope) => void;
}

/** Tamanho do desenho, em unidades do SVG. */
const L = 460;
const A = 150;
/** Margem para as alças não saírem cortadas na borda. */
const M = 12;

/** Largura que a sustentação ocupa no desenho. Ela não tem duração própria. */
const LARGURA_SUSTENTACAO = 0.18;

/** Largura mínima de um estágio no desenho, em unidades do SVG. */
const LARGURA_MINIMA = 14;

/** Teto de cada estágio, em milissegundos. */
const MAXIMO_MS = 8000;

/** Quantos milissegundos o desenho mostra, no mínimo. Sem isso, um envelope
 * todo zerado viraria uma parede vertical. */
const ESCALA_MINIMA_MS = 600;

type Alca = "atraso" | "ataque" | "retencao" | "decaimento" | "sustentacao" | "liberacao";

export default function EnvelopeGrafico({ envelope, onMudar }: Props) {
  const svg = useRef<SVGSVGElement>(null);
  const [arrastando, setArrastando] = useState<Alca | null>(null);
  // A escala é travada no começo do arrasto: se ela mudasse junto, o nó fugiria
  // do cursor enquanto a pessoa arrasta.
  const inicio = useRef<{ x: number; y: number; env: Envelope; escala: number } | null>(null);

  const somaDosTempos =
    envelope.atraso_ms +
    envelope.ataque_ms +
    envelope.retencao_ms +
    envelope.decaimento_ms +
    envelope.liberacao_ms;
  const escala = Math.max(ESCALA_MINIMA_MS, somaDosTempos);

  // Largura útil, tirando o pedaço reservado para a sustentação.
  const util = (L - 2 * M) * (1 - LARGURA_SUSTENTACAO);
  // Todo estágio ocupa uma largura mínima no desenho, mesmo valendo zero. Sem
  // isso as cinco alças caem no mesmo ponto num envelope zerado e não há como
  // pegar nenhuma.
  const px = (ms: number) => Math.max(LARGURA_MINIMA, (ms / escala) * util);
  const y = (nivel: number) => M + (1 - nivel) * (A - 2 * M);

  const x0 = M;
  const xAtaque = x0 + px(envelope.atraso_ms);
  const xTopo = xAtaque + px(envelope.ataque_ms);
  const xFimRetencao = xTopo + px(envelope.retencao_ms);
  const xSustentacao = xFimRetencao + px(envelope.decaimento_ms);
  const xFimSustentacao = xSustentacao + (L - 2 * M) * LARGURA_SUSTENTACAO;
  const xFim = xFimSustentacao + px(envelope.liberacao_ms);

  /** Uma curva em pedacinhos, com a mesma conta do motor. */
  function trecho(
    xa: number,
    xb: number,
    de: number,
    para: number,
    tensao: number,
  ): string {
    if (xb - xa < 0.5) return `L ${xb} ${y(para)}`;
    const passos = 24;
    let d = "";
    for (let i = 1; i <= passos; i++) {
      const f = i / passos;
      const nivel = de + (para - de) * curvar(f, tensao);
      d += ` L ${(xa + (xb - xa) * f).toFixed(2)} ${y(nivel).toFixed(2)}`;
    }
    return d;
  }

  const caminho =
    `M ${x0} ${y(0)}` +
    ` L ${xAtaque} ${y(0)}` +
    trecho(xAtaque, xTopo, 0, 1, envelope.tensao_ataque) +
    ` L ${xFimRetencao} ${y(1)}` +
    trecho(xFimRetencao, xSustentacao, 1, envelope.sustentacao, envelope.tensao_queda) +
    ` L ${xFimSustentacao} ${y(envelope.sustentacao)}` +
    trecho(xFimSustentacao, xFim, envelope.sustentacao, 0, envelope.tensao_queda);

  function aoPegar(alca: Alca, e: React.PointerEvent) {
    e.preventDefault();
    inicio.current = { x: e.clientX, y: e.clientY, env: { ...envelope }, escala };
    setArrastando(alca);
  }

  // O arrasto escuta a janela, não o SVG. Preso ao SVG, sair do desenho com o
  // botão apertado largava o nó no meio do caminho.
  useEffect(() => {
    if (!arrastando) return;
    const mover = (e: PointerEvent) => aoMover(e);
    const soltar = () => aoSoltar();
    window.addEventListener("pointermove", mover);
    window.addEventListener("pointerup", soltar);
    window.addEventListener("pointercancel", soltar);
    return () => {
      window.removeEventListener("pointermove", mover);
      window.removeEventListener("pointerup", soltar);
      window.removeEventListener("pointercancel", soltar);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [arrastando, envelope]);

  function aoMover(e: { clientX: number; clientY: number }) {
    if (!arrastando || !inicio.current) return;
    const { x, y: yInicial, env, escala: escalaTravada } = inicio.current;
    const caixa = svg.current?.getBoundingClientRect();
    // O SVG é desenhado em unidades próprias e esticado na tela; sem esta
    // conversão o arrasto anda mais rápido ou mais devagar que o cursor.
    const porPixel = caixa ? L / caixa.width : 1;
    const dx = (e.clientX - x) * porPixel;
    const dy = (e.clientY - yInicial) * (caixa ? A / caixa.height : 1);
    // O arrasto converte pela escala de verdade, não pela largura mínima do
    // desenho: sair de zero tem que responder ao primeiro pixel arrastado.
    const dms = Math.round((dx / util) * escalaTravada);

    const limitar = (v: number) => Math.min(MAXIMO_MS, Math.max(0, v));
    switch (arrastando) {
      case "atraso":
        onMudar({ ...env, atraso_ms: limitar(env.atraso_ms + dms) });
        break;
      case "ataque":
        onMudar({ ...env, ataque_ms: limitar(env.ataque_ms + dms) });
        break;
      case "retencao":
        onMudar({ ...env, retencao_ms: limitar(env.retencao_ms + dms) });
        break;
      case "decaimento":
        onMudar({ ...env, decaimento_ms: limitar(env.decaimento_ms + dms) });
        break;
      case "liberacao":
        onMudar({ ...env, liberacao_ms: limitar(env.liberacao_ms + dms) });
        break;
      case "sustentacao": {
        const nivel = env.sustentacao - dy / (A - 2 * M);
        onMudar({
          ...env,
          sustentacao: Math.min(1, Math.max(0, Number(nivel.toFixed(3)))),
          decaimento_ms: limitar(env.decaimento_ms + dms),
        });
        break;
      }
    }
  }

  function aoSoltar() {
    setArrastando(null);
    inicio.current = null;
  }

  const nos: { alca: Alca; x: number; y: number; titulo: string }[] = [
    { alca: "atraso", x: xAtaque, y: y(0), titulo: "Atraso" },
    { alca: "ataque", x: xTopo, y: y(1), titulo: "Ataque" },
    { alca: "retencao", x: xFimRetencao, y: y(1), titulo: "Retenção" },
    {
      alca: "sustentacao",
      x: xSustentacao,
      y: y(envelope.sustentacao),
      titulo: "Decaimento e sustentação",
    },
    { alca: "liberacao", x: xFim, y: y(0), titulo: "Liberação" },
  ];

  /**
   * Até onde a alça `i` responde ao clique: metade do caminho até o vizinho
   * mais próximo. Com um alvo fixo e grande, a alça da direita cobria a da
   * esquerda quando os dois estágios eram curtos, e uma delas ficava
   * inalcançável.
   */
  function alcance(i: number): number {
    const anterior = i > 0 ? nos[i - 1].x : nos[i].x - 999;
    const proximo = i < nos.length - 1 ? nos[i + 1].x : nos[i].x + 999;
    const folga = Math.min(nos[i].x - anterior, proximo - nos[i].x) / 2;
    return Math.min(14, Math.max(4, folga));
  }

  return (
    <div>
      <svg
        ref={svg}
        viewBox={`0 0 ${L} ${A}`}
        className="w-full touch-none select-none rounded-lg bg-neutral-100 dark:bg-neutral-900"
      >
        {/* Linhas de referência: o cheio e o silêncio. */}
        <line x1={M} y1={y(1)} x2={L - M} y2={y(1)} className="stroke-neutral-300 dark:stroke-neutral-700" strokeWidth={1} strokeDasharray="3 4" />
        <line x1={M} y1={y(0)} x2={L - M} y2={y(0)} className="stroke-neutral-300 dark:stroke-neutral-700" strokeWidth={1} />

        {/* A faixa da sustentação, que não tem duração e por isso é marcada. */}
        <rect
          x={xSustentacao}
          y={M}
          width={Math.max(0, xFimSustentacao - xSustentacao)}
          height={A - 2 * M}
          className="fill-neutral-200/60 dark:fill-neutral-800/60"
        />

        <path
          d={`${caminho} L ${xFim} ${y(0)} L ${x0} ${y(0)} Z`}
          className="fill-emerald-500/15"
          stroke="none"
        />
        <path d={caminho} className="stroke-emerald-500" strokeWidth={2} fill="none" />

        {nos.map((n, i) => (
          <No
            key={n.alca}
            x={n.x}
            y={n.y}
            raio={alcance(i)}
            ativo={arrastando === n.alca}
            titulo={n.titulo}
            aoPegar={(e) => aoPegar(n.alca, e)}
          />
        ))}
      </svg>

      <div className="mt-1 flex justify-between px-1 text-[10px] uppercase tracking-wide text-neutral-500">
        <span>Atraso</span>
        <span>Ataque</span>
        <span>Reten.</span>
        <span>Decai.</span>
        <span>Sustent.</span>
        <span>Liber.</span>
      </div>
    </div>
  );
}

function No({
  x,
  y,
  raio,
  ativo,
  titulo,
  aoPegar,
}: {
  x: number;
  y: number;
  /** Até onde ele responde ao clique. */
  raio: number;
  ativo: boolean;
  titulo: string;
  aoPegar: (e: React.PointerEvent) => void;
}) {
  return (
    <g onPointerDown={aoPegar} className="cursor-ew-resize">
      <title>{titulo}</title>
      {/* Alvo invisível maior que a bolinha: pegar um círculo de 5 px é difícil. */}
      <circle cx={x} cy={y} r={raio} fill="transparent" />
      <circle
        cx={x}
        cy={y}
        r={ativo ? 7 : 5}
        className={
          ativo
            ? "fill-emerald-400 stroke-emerald-600"
            : "fill-white stroke-emerald-500 dark:fill-neutral-800"
        }
        strokeWidth={2}
      />
    </g>
  );
}
