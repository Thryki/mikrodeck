/**
 * Escolher um programa pelo nome, como no menu Iniciar.
 *
 * Existe porque caçar o executável no disco não funciona: app da Microsoft
 * Store mora numa pasta protegida, e o nome do arquivo raramente é o nome do
 * programa. Aqui a pessoa digita "ray" e acha o Raycast.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { listarApps, type AppInstalado } from "./ponte";

interface Props {
  onEscolher: (app: AppInstalado) => void;
  onFechar: () => void;
}

/** Tira acento e caixa, para "camera" achar "Câmera". */
function simplificar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

export default function SeletorApp({ onEscolher, onFechar }: Props) {
  const [apps, setApps] = useState<AppInstalado[] | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [busca, setBusca] = useState("");
  const [escolhido, setEscolhido] = useState(0);
  const campo = useRef<HTMLInputElement>(null);

  useEffect(() => {
    listarApps().then(setApps).catch((e) => setErro(String(e)));
    campo.current?.focus();
  }, []);

  const filtrados = useMemo(() => {
    if (!apps) return [];
    const termo = simplificar(busca.trim());
    if (!termo) return apps;
    // Quem começa com o que foi digitado vem antes: digitar "s" tem que trazer
    // "Spotify" na frente de "Microsoft Store".
    const contem = apps.filter((a) => simplificar(a.nome).includes(termo));
    return contem.sort((a, b) => {
      const ca = simplificar(a.nome).startsWith(termo) ? 0 : 1;
      const cb = simplificar(b.nome).startsWith(termo) ? 0 : 1;
      return ca - cb || a.nome.localeCompare(b.nome);
    });
  }, [apps, busca]);

  useEffect(() => setEscolhido(0), [busca]);

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
        className="flex max-h-[70vh] w-full max-w-md flex-col overflow-hidden rounded-xl border border-neutral-300 bg-white shadow-xl dark:border-neutral-700 dark:bg-neutral-900"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          ref={campo}
          type="text"
          value={busca}
          onChange={(e) => setBusca(e.target.value)}
          onKeyDown={aoTeclar}
          placeholder="Digite o nome do programa"
          className="border-b border-neutral-200 bg-transparent px-4 py-3 text-sm outline-none dark:border-neutral-800"
        />
        <div className="min-h-0 flex-1 overflow-y-auto">
          {erro && <p className="p-4 text-xs text-red-500">{erro}</p>}
          {!apps && !erro && (
            <p className="p-4 text-xs text-neutral-500">Lendo o menu Iniciar…</p>
          )}
          {apps && filtrados.length === 0 && (
            <p className="p-4 text-xs text-neutral-500">
              Nada com esse nome. O programa precisa aparecer no menu Iniciar do
              Windows para entrar nesta lista.
            </p>
          )}
          {filtrados.map((a, i) => (
            <button
              key={a.caminho}
              onClick={() => onEscolher(a)}
              onMouseEnter={() => setEscolhido(i)}
              className={`flex w-full items-center justify-between gap-3 px-4 py-2 text-left text-[13px] ${
                i === escolhido
                  ? "bg-sky-500/10 text-sky-700 dark:text-sky-300"
                  : "hover:bg-neutral-100 dark:hover:bg-neutral-800"
              }`}
            >
              <span className="truncate">{a.nome}</span>
              {a.da_loja && (
                <span className="shrink-0 text-[10px] uppercase tracking-wide text-neutral-400">
                  Store
                </span>
              )}
            </button>
          ))}
        </div>
        <div className="flex items-center justify-between border-t border-neutral-200 px-4 py-2 text-xs text-neutral-500 dark:border-neutral-800">
          <span>
            {apps ? `${filtrados.length} de ${apps.length}` : ""}
          </span>
          <span>Setas para andar, Enter para escolher</span>
        </div>
      </div>
    </div>
  );
}
