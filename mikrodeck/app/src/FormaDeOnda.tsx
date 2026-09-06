/**
 * O desenho do som escolhido ou gravado.
 *
 * Serve para duas coisas: ver que gravou alguma coisa, e ver o quê. Depois de
 * gravar, um contorno diferente é a prova de que o som novo entrou.
 */

import { useEffect, useState } from "react";
import { formaDeOnda } from "./ponte";

interface Props {
  caminho: string;
  /** Muda quando o arquivo é regravado, para o desenho ser refeito. */
  versao?: number;
}

/** Quantas barras o desenho tem. */
const COLUNAS = 110;

export default function FormaDeOnda({ caminho, versao = 0 }: Props) {
  const [picos, setPicos] = useState<number[] | null>(null);
  const [segundos, setSegundos] = useState(0);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    if (!caminho) {
      setPicos(null);
      return;
    }
    let cancelado = false;
    setErro(null);
    formaDeOnda(caminho, COLUNAS)
      .then((f) => {
        if (cancelado) return;
        setPicos(f.picos);
        setSegundos(f.segundos);
      })
      .catch((e) => !cancelado && setErro(String(e)));
    return () => {
      cancelado = true;
    };
  }, [caminho, versao]);

  if (erro) {
    return <p className="text-xs text-red-500">{erro}</p>;
  }
  if (!picos) {
    return null;
  }

  const largura = COLUNAS * 4;
  const altura = 56;
  const meio = altura / 2;

  return (
    <div>
      <svg
        viewBox={`0 0 ${largura} ${altura}`}
        preserveAspectRatio="none"
        className="h-14 w-full rounded-lg bg-neutral-100 dark:bg-neutral-900"
      >
        <line
          x1={0}
          y1={meio}
          x2={largura}
          y2={meio}
          className="stroke-neutral-300 dark:stroke-neutral-700"
          strokeWidth={1}
        />
        {picos.map((p, i) => {
          // Barra mínima de 1: assim o silêncio aparece como linha, não como
          // buraco, e dá para ver onde o som começa.
          const h = Math.max(1, p * (altura - 6));
          return (
            <rect
              key={i}
              x={i * 4 + 1}
              y={meio - h / 2}
              width={2}
              height={h}
              rx={1}
              className="fill-emerald-500"
            />
          );
        })}
      </svg>
      <p className="mt-1 text-right text-xs tabular-nums text-neutral-500">
        {segundos.toFixed(2)} s
      </p>
    </div>
  );
}
