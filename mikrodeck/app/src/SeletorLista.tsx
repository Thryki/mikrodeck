/**
 * Uma lista com busca, para escolher uma coisa entre muitas.
 *
 * Serve para os programas instalados e para as entidades do Home Assistant: nos
 * dois casos são centenas de itens com nome comprido, e digitar o nome na mão
 * era o que estava difícil.
 */

import { useEffect, useMemo, useRef, useState } from "react";

export interface ItemDaLista {
  /** O que identifica o item, e o que volta ao escolher. */
  id: string;
  titulo: string;
  /** Linha de baixo, menor. */
  detalhe?: string;
  /** Selo à direita, tipo "Store" ou o estado da entidade. */
  etiqueta?: string;
}

interface Props {
  titulo: string;
  itens: ItemDaLista[] | null;
  erro?: string | null;
  /** O que dizer quando a busca não acha nada. */
  vazio: string;
  onEscolher: (item: ItemDaLista) => void;
  onFechar: () => void;
}

/** Tira acento e caixa, para "camera" achar "Câmera". */
export function simplificar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

export default function SeletorLista({
  titulo,
  itens,
  erro,
  vazio,
  onEscolher,
  onFechar,
}: Props) {
  const [busca, setBusca] = useState("");
  const [escolhido, setEscolhido] = useState(0);
  const campo = useRef<HTMLInputElement>(null);
  const lista = useRef<HTMLDivElement>(null);

  useEffect(() => campo.current?.focus(), []);
  useEffect(() => setEscolhido(0), [busca]);

  const filtrados = useMemo(() => {
    if (!itens) return [];
    const termo = simplificar(busca.trim());
    if (!termo) return itens;
    const cabe = (i: ItemDaLista) =>
      simplificar(i.titulo).includes(termo) ||
      simplificar(i.detalhe ?? "").includes(termo);
    // Quem começa com o que foi digitado vem antes: digitar "s" tem que trazer
    // "Sala" na frente de "Escritório sul".
    return itens.filter(cabe).sort((a, b) => {
      const ca = simplificar(a.titulo).startsWith(termo) ? 0 : 1;
      const cb = simplificar(b.titulo).startsWith(termo) ? 0 : 1;
      return ca - cb || a.titulo.localeCompare(b.titulo);
    });
  }, [itens, busca]);

  // Andar com as setas sem perder o item de vista.
  useEffect(() => {
    lista.current
      ?.querySelector(`[data-linha="${escolhido}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [escolhido]);

  function aoTeclar(e: React.KeyboardEvent) {
    if (e.key === "Escape") return onFechar();
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setEscolhido((i) => Math.min(filtrados.length - 1, i + 1));
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setEscolhido((i) => Math.max(0, i - 1));
    }
    if (e.key === "Enter" && filtrados[escolhido]) {
      onEscolher(filtrados[escolhido]);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-8"
      onClick={onFechar}
    >
      <div
        className="flex max-h-[72vh] w-full max-w-md flex-col overflow-hidden rounded-xl border border-neutral-300 bg-white shadow-xl dark:border-neutral-700 dark:bg-neutral-900"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          ref={campo}
          type="text"
          value={busca}
          onChange={(e) => setBusca(e.target.value)}
          onKeyDown={aoTeclar}
          placeholder={titulo}
          className="border-b border-neutral-200 bg-transparent px-4 py-3 text-sm outline-none dark:border-neutral-800"
        />
        <div ref={lista} className="min-h-0 flex-1 overflow-y-auto">
          {erro && <p className="p-4 text-xs text-red-500">{erro}</p>}
          {!itens && !erro && (
            <p className="p-4 text-xs text-neutral-500">Buscando…</p>
          )}
          {itens && filtrados.length === 0 && (
            <p className="p-4 text-xs text-neutral-500">{vazio}</p>
          )}
          {filtrados.map((item, i) => (
            <button
              key={item.id}
              data-linha={i}
              onClick={() => onEscolher(item)}
              onMouseEnter={() => setEscolhido(i)}
              className={`flex w-full items-center justify-between gap-3 px-4 py-2 text-left ${
                i === escolhido
                  ? "bg-sky-500/10"
                  : "hover:bg-neutral-100 dark:hover:bg-neutral-800"
              }`}
            >
              <span className="min-w-0">
                <span
                  className={`block truncate text-[13px] ${
                    i === escolhido ? "text-sky-700 dark:text-sky-300" : ""
                  }`}
                >
                  {item.titulo}
                </span>
                {item.detalhe && (
                  <span className="block truncate text-[11px] text-neutral-500">
                    {item.detalhe}
                  </span>
                )}
              </span>
              {item.etiqueta && (
                <span className="shrink-0 text-[10px] uppercase tracking-wide text-neutral-400">
                  {item.etiqueta}
                </span>
              )}
            </button>
          ))}
        </div>
        <div className="flex items-center justify-between border-t border-neutral-200 px-4 py-2 text-xs text-neutral-500 dark:border-neutral-800">
          <span>{itens ? `${filtrados.length} de ${itens.length}` : ""}</span>
          <span>Setas para andar, Enter para escolher</span>
        </div>
      </div>
    </div>
  );
}
