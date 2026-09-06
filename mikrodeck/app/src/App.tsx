/**
 * Tela principal do MikroDeck.
 *
 * Segue a direção aprovada em `design/mockup-config.html`: barra de status,
 * abas de páginas, o aparelho no centro, e o painel do controle selecionado
 * à direita. Interface neutra; as cores fortes são as dos pads.
 */

import { useCallback, useEffect, useState } from "react";
import {
  adicionarPaginaPronta,
  escutar,
  irParaPagina,
  lerConfig,
  lerSituacao,
  paginasProntas,
  salvarConfig,
  type PaginaPronta,
} from "./ponte";
import { Aparelho, type Selecao } from "./Aparelho";
import { PainelControle } from "./PainelControle";
import { Configuracoes } from "./Configuracoes";
import type { NomeCor } from "./cores";
import { rotuloDoBotao, type Config, type Controle, type Situacao } from "./tipos";
import "./App.css";

export default function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [situacao, setSituacao] = useState<Situacao>("procurando");
  const [pagina, setPagina] = useState(0);
  const [selecao, setSelecao] = useState<Selecao | null>(null);
  const [padsApertados, setPadsApertados] = useState<Set<number>>(new Set());
  const [botoesApertados, setBotoesApertados] = useState<Set<string>>(new Set());
  const [salvando, setSalvando] = useState(false);
  const [menuPagina, setMenuPagina] = useState<{ i: number; x: number; y: number } | null>(null);
  const [mostrarConfig, setMostrarConfig] = useState(false);
  const [pausado, setPausado] = useState(false);
  const [prontas, setProntas] = useState<PaginaPronta[]>([]);
  // Arrasto de um controle para cima de outro que já tem configuração.
  const [conflito, setConflito] = useState<{
    origem: Selecao;
    destino: Selecao;
  } | null>(null);
  const [mostrarProntas, setMostrarProntas] = useState(false);

  useEffect(() => {
    lerConfig().then(setConfig).catch(console.error);
    lerSituacao().then(setSituacao).catch(console.error);
    paginasProntas().then(setProntas).catch(console.error);
  }, []);

  // O menu de contexto do navegador não faz sentido num app. Some com ele.
  useEffect(() => {
    const bloquear = (e: MouseEvent) => e.preventDefault();
    document.addEventListener("contextmenu", bloquear);
    return () => document.removeEventListener("contextmenu", bloquear);
  }, []);

  // Fecha o menu de página ao clicar em qualquer lugar.
  useEffect(() => {
    if (!menuPagina) return;
    const fechar = () => setMenuPagina(null);
    document.addEventListener("click", fechar);
    return () => document.removeEventListener("click", fechar);
  }, [menuPagina]);

  // Escuta o motor: conexão, controles apertados de verdade, troca de página.
  useEffect(() => {
    const inscricoes = [
      escutar<Situacao>("situacao", setSituacao),
      escutar<boolean>("pausado", setPausado),
      // A config pode mudar por fora, pelo servidor MCP ou na mão no arquivo.
      escutar<null>("config-mudou", () => {
        lerConfig().then(setConfig).catch(console.error);
      }),
      escutar<{ pad: number; apertado: boolean }>("pad", (d) => {
        setPadsApertados((antes) => {
          const novo = new Set(antes);
          if (d.apertado) novo.add(d.pad);
          else novo.delete(d.pad);
          return novo;
        });
      }),
      escutar<{ nome: string; apertado: boolean }>("botao", (d) => {
        setBotoesApertados((antes) => {
          const novo = new Set(antes);
          if (d.apertado) novo.add(d.nome);
          else novo.delete(d.nome);
          return novo;
        });
      }),
      escutar<{ numero: number; nome: string }>("pagina", (d) => {
        setPagina(d.numero - 1);
      }),
    ];
    return () => {
      inscricoes.forEach((p) => p.then((remover) => remover()));
    };
  }, []);

  const salvar = useCallback(async (novo: Config) => {
    setConfig(novo);
    setSalvando(true);
    try {
      await salvarConfig(novo);
    } catch (e) {
      console.error(e);
    } finally {
      setSalvando(false);
    }
  }, []);

  if (!config) {
    return (
      <div className="flex h-screen items-center justify-center text-sm text-neutral-500">
        Carregando…
      </div>
    );
  }

  const paginaAtual = config.paginas[pagina];
  const pads = paginaAtual?.pads ?? {};
  const botoes = paginaAtual?.botoes ?? {};

  const cores: Record<number, NomeCor> = {};
  const nomes: Record<number, string> = {};
  for (const [chave, c] of Object.entries(pads)) {
    cores[Number(chave)] = c.cor;
    nomes[Number(chave)] = c.nome;
  }
  const botoesProgramados: Record<string, string> = {};
  for (const [id, c] of Object.entries(botoes)) {
    botoesProgramados[id] = c.nome || rotuloDoBotao(id);
  }

  const controleAtual: Controle | undefined =
    selecao?.tipo === "pad"
      ? pads[String(selecao.pad)]
      : selecao?.tipo === "botao"
        ? botoes[selecao.id]
        : undefined;

  /** Grava o controle no que está selecionado. Passar null limpa. */
  function mudarControle(controle: Controle | null) {
    if (!selecao) return;
    const paginas = config!.paginas.map((p, i) => {
      if (i !== pagina) return p;
      if (selecao.tipo === "pad") {
        const novos = { ...p.pads };
        if (controle) novos[String(selecao.pad)] = controle;
        else delete novos[String(selecao.pad)];
        return { ...p, pads: novos };
      }
      const novos = { ...p.botoes };
      if (controle) novos[selecao.id] = controle;
      else delete novos[selecao.id];
      return { ...p, botoes: novos };
    });
    salvar({ ...config!, paginas });
  }

  /** Botões que a configuração reservou para trocar de página. */
  const reservados = config
    ? [config.botao_pagina_anterior, config.botao_proxima_pagina]
    : [];

  /** Lê o controle de uma posição na página atual. */
  function controleDe(s: Selecao): Controle | undefined {
    if (!paginaAtual) return undefined;
    return s.tipo === "pad"
      ? paginaAtual.pads[String(s.pad)]
      : paginaAtual.botoes[s.id];
  }

  /** Grava vários controles de uma vez na página atual. */
  function gravarControles(mudancas: { onde: Selecao; o_que: Controle | null }[]) {
    const paginas = config!.paginas.map((p, i) => {
      if (i !== pagina) return p;
      const pads = { ...p.pads };
      const botoes = { ...p.botoes };
      for (const { onde, o_que } of mudancas) {
        if (onde.tipo === "pad") {
          if (o_que) pads[String(onde.pad)] = o_que;
          else delete pads[String(onde.pad)];
        } else {
          if (o_que) botoes[onde.id] = o_que;
          else delete botoes[onde.id];
        }
      }
      return { ...p, pads, botoes };
    });
    salvar({ ...config!, paginas });
  }

  /** Arrastou um controle para cima de outro. */
  function mover(origem: Selecao, destino: Selecao) {
    const daOrigem = controleDe(origem);
    if (!daOrigem) return;
    if (controleDe(destino)) {
      setConflito({ origem, destino });
      return;
    }
    gravarControles([
      { onde: destino, o_que: daOrigem },
      { onde: origem, o_que: null },
    ]);
    setSelecao(destino);
  }

  function substituir() {
    if (!conflito) return;
    const daOrigem = controleDe(conflito.origem);
    if (daOrigem) {
      gravarControles([
        { onde: conflito.destino, o_que: daOrigem },
        { onde: conflito.origem, o_que: null },
      ]);
      setSelecao(conflito.destino);
    }
    setConflito(null);
  }

  function trocarDeLugar() {
    if (!conflito) return;
    const daOrigem = controleDe(conflito.origem);
    const doDestino = controleDe(conflito.destino);
    if (daOrigem && doDestino) {
      gravarControles([
        { onde: conflito.destino, o_que: daOrigem },
        { onde: conflito.origem, o_que: doDestino },
      ]);
      setSelecao(conflito.destino);
    }
    setConflito(null);
  }

  /** Nome amigável de uma posição, para o texto do modal. */
  function ondeFica(s: Selecao): string {
    return s.tipo === "pad" ? `pad ${s.pad}` : rotuloDoBotao(s.id);
  }

  /**
   * O que a tela do aparelho está mostrando. Espelha a lógica do compositor
   * do motor: o controle segurado ganha do nome da página.
   */
  function telaDoAparelho(): { titulo: string; sub?: string } {
    const padSegurado = [...padsApertados][0];
    if (padSegurado !== undefined) {
      const c = pads[String(padSegurado)];
      if (c?.nome) return { titulo: c.nome };
    }
    const botaoSegurado = [...botoesApertados][0];
    if (botaoSegurado !== undefined) {
      const c = botoes[botaoSegurado];
      if (c?.nome) return { titulo: c.nome };
    }
    return {
      titulo: paginaAtual?.nome ?? "—",
      sub: `${pagina + 1}/${config!.paginas.length}`,
    };
  }

  function trocarPagina(indice: number) {
    setPagina(indice);
    setSelecao(null);
    irParaPagina(indice + 1).catch(console.error);
  }

  function novaPagina() {
    const paginas = [
      ...config!.paginas,
      { nome: `Página ${config!.paginas.length + 1}`, pads: {}, botoes: {} },
    ];
    salvar({ ...config!, paginas });
    setPagina(paginas.length - 1);
  }

  function renomearPagina(i: number) {
    const nome = prompt("Nome da página", config!.paginas[i].nome);
    if (nome === null) return;
    const paginas = config!.paginas.map((p, j) =>
      j === i ? { ...p, nome: nome.trim() || p.nome } : p,
    );
    salvar({ ...config!, paginas });
  }

  function apagarPagina(i: number) {
    if (config!.paginas.length === 1) {
      alert("Não dá para apagar a única página.");
      return;
    }
    if (!confirm(`Apagar a página "${config!.paginas[i].nome}"?`)) return;
    const paginas = config!.paginas.filter((_, j) => j !== i);
    salvar({ ...config!, paginas });
    if (pagina >= paginas.length) trocarPagina(paginas.length - 1);
  }

  function duplicarPagina(i: number) {
    const copia = {
      ...config!.paginas[i],
      nome: `${config!.paginas[i].nome} (cópia)`,
    };
    const paginas = [...config!.paginas];
    paginas.splice(i + 1, 0, copia);
    salvar({ ...config!, paginas });
  }

  return (
    <div className="flex h-full flex-col bg-neutral-100 text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100">
      <div className="flex min-h-0 flex-1 flex-col bg-white dark:bg-neutral-800">
        {/* Abas de páginas */}
        <div className="flex flex-wrap items-center gap-2 border-b border-neutral-200 px-4 py-2.5 dark:border-neutral-700">
          <span className="mr-1 text-[13px] text-neutral-600 dark:text-neutral-400">
            Páginas
          </span>
          {config.paginas.map((p, i) => (
            <button
              key={i}
              onClick={() => trocarPagina(i)}
              onContextMenu={(e) => {
                e.preventDefault();
                setMenuPagina({ i, x: e.clientX, y: e.clientY });
              }}
              className={`rounded-lg px-3 py-1 text-[13px] transition ${
                i === pagina
                  ? "bg-sky-100 font-medium text-sky-800 dark:bg-sky-900 dark:text-sky-200"
                  : "border border-neutral-300 text-neutral-600 hover:bg-neutral-50 dark:border-neutral-600 dark:text-neutral-400 dark:hover:bg-neutral-700"
              }`}
            >
              {i + 1} · {p.nome}
            </button>
          ))}
          <div className="relative">
            <button
              onClick={() => setMostrarProntas((v) => !v)}
              title="Nova página"
              aria-label="Nova página"
              className="flex h-7 w-7 items-center justify-center rounded-lg border border-neutral-300 text-neutral-600 hover:bg-neutral-50 dark:border-neutral-600 dark:text-neutral-400 dark:hover:bg-neutral-700"
            >
              <svg viewBox="0 0 16 16" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth={1.6} strokeLinecap="round">
                <path d="M8 3.5v9M3.5 8h9" />
              </svg>
            </button>
            {mostrarProntas && (
              <>
                <div
                  className="fixed inset-0 z-10"
                  onClick={() => setMostrarProntas(false)}
                />
                <div className="absolute left-0 top-full z-20 mt-1 w-72 overflow-hidden rounded-lg border border-neutral-300 bg-white shadow-lg dark:border-neutral-600 dark:bg-neutral-800">
                  <button
                    onClick={() => {
                      setMostrarProntas(false);
                      novaPagina();
                    }}
                    className="block w-full border-b border-neutral-200 px-3 py-2 text-left hover:bg-neutral-50 dark:border-neutral-700 dark:hover:bg-neutral-700"
                  >
                    <span className="block text-[13px] font-medium">Vazia</span>
                    <span className="block text-xs text-neutral-500">
                      Uma página em branco, para você montar do jeito que quiser.
                    </span>
                  </button>
                  {prontas.map((p) => (
                    <button
                      key={p.id}
                      onClick={async () => {
                        setMostrarProntas(false);
                        try {
                          const nova = await adicionarPaginaPronta(p.id);
                          setConfig(nova);
                          trocarPagina(nova.paginas.length - 1);
                        } catch (e) {
                          console.error(e);
                        }
                      }}
                      className="block w-full border-b border-neutral-200 px-3 py-2 text-left last:border-0 hover:bg-neutral-50 dark:border-neutral-700 dark:hover:bg-neutral-700"
                    >
                      <span className="block text-[13px] font-medium">{p.nome}</span>
                      <span className="block text-xs text-neutral-500">
                        {p.descricao}
                      </span>
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>
          <span className="hidden text-xs text-neutral-500 xl:inline">
            botão direito na aba para renomear ou apagar
          </span>

          <span className="flex-1" />

          {salvando && <span className="text-xs text-neutral-500">salvando…</span>}
          {pausado && (
            <span className="rounded-full bg-amber-100 px-2.5 py-0.5 text-[12px] text-amber-900 dark:bg-amber-900/40 dark:text-amber-200">
              Desligado
            </span>
          )}
          <label className="flex items-center gap-2 text-[13px] text-neutral-600 dark:text-neutral-400">
            Brilho
            <input
              type="range"
              min={0}
              max={3}
              value={config.brilho}
              onChange={(e) =>
                salvar({ ...config, brilho: Number(e.target.value) })
              }
              className="w-20"
            />
          </label>
          <button
            onClick={() => setMostrarConfig(true)}
            title="Configurações"
            aria-label="Configurações"
            className="flex h-7 w-7 items-center justify-center rounded-lg border border-neutral-300 text-neutral-600 hover:bg-neutral-50 dark:border-neutral-600 dark:text-neutral-400 dark:hover:bg-neutral-700"
          >
            <svg viewBox="0 0 24 24" className="h-4 w-4" fill="none" stroke="currentColor" strokeWidth={1.6}>
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9v0a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
        </div>

        <div className="grid min-h-0 flex-1 grid-cols-1 lg:grid-cols-[1fr_360px]">
          <div className="flex min-h-0 flex-col items-center justify-center gap-3 overflow-auto border-neutral-200 p-5 lg:border-r dark:border-neutral-700">
            {/* A caixa segue a proporção do desenho, senão sobra um vão morto
                entre o aparelho e a legenda. */}
            <div className="aspect-[960/536] max-h-full w-full max-w-[960px]">
              <Aparelho
                cores={cores}
                nomes={nomes}
                botoesProgramados={botoesProgramados}
                padsApertados={padsApertados}
                botoesApertados={botoesApertados}
                selecao={selecao}
                onSelecionar={setSelecao}
                onMover={mover}
                reservados={reservados}
                brilho={config.brilho}
                tela={telaDoAparelho()}
              />
            </div>
            <div className="flex shrink-0 flex-col items-center gap-1">
              <span
                className="flex items-center gap-2 text-xs text-neutral-500"
                title={
                  situacao === "conectado"
                    ? "Mikro MK3 conectado"
                    : "Procurando o aparelho"
                }
              >
                <span
                  className={`inline-block h-2 w-2 rounded-full ${
                    situacao === "conectado" ? "bg-green-500" : "bg-red-500"
                  }`}
                />
                {situacao === "conectado" ? "Conectado" : "Procurando…"}
              </span>
              <p className="text-center text-xs text-neutral-500">
                Clique em qualquer pad ou botão para configurar. O que você aperta
                no aparelho acende aqui também.
              </p>
            </div>
          </div>

          <div className="min-h-0 overflow-auto p-5">
            {!selecao ? (
              <div className="flex h-full min-h-40 flex-col justify-center gap-3 text-sm text-neutral-500">
                <p className="font-medium text-neutral-700 dark:text-neutral-300">
                  Página {pagina + 1}: {paginaAtual?.nome}
                </p>
                <p>
                  {Object.keys(paginaAtual?.pads ?? {}).length} de 16 pads
                  programados
                  {Object.keys(paginaAtual?.botoes ?? {}).length === 1
                    ? ", 1 botão."
                    : `, ${Object.keys(paginaAtual?.botoes ?? {}).length} botões.`}
                </p>
                <p className="text-xs leading-relaxed">
                  Clique num pad ou botão do desenho ao lado para escolher o que
                  ele faz, o nome que aparece na tela e a cor.
                </p>
              </div>
            ) : (
              <PainelControle
                reservado={
                  selecao.tipo === "botao" && reservados.includes(selecao.id)
                }
                selecao={selecao}
                nomePagina={paginaAtual?.nome ?? ""}
                numeroPagina={pagina + 1}
                controle={controleAtual}
                totalPaginas={config.paginas.length}
                brilhoGeral={config.brilho}
                onMudar={mudarControle}
              />
            )}
          </div>
        </div>
      </div>

      {/* Arrastou em cima de um controle que já tinha configuração */}
      {conflito && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-6"
          onClick={() => setConflito(null)}
        >
          <div
            className="w-full max-w-sm rounded-xl border border-neutral-300 bg-white p-5 dark:border-neutral-700 dark:bg-neutral-800"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="mb-2 text-base font-medium">
              O {ondeFica(conflito.destino)} já está ocupado
            </h2>
            <p className="mb-1 text-[13px] text-neutral-600 dark:text-neutral-400">
              Lá está <strong>{controleDe(conflito.destino)?.nome || "sem nome"}</strong>.
            </p>
            <p className="mb-4 text-[13px] text-neutral-600 dark:text-neutral-400">
              Você arrastou <strong>{controleDe(conflito.origem)?.nome || "sem nome"}</strong>{" "}
              do {ondeFica(conflito.origem)}.
            </p>
            <div className="flex flex-wrap gap-2">
              <button
                onClick={substituir}
                className="rounded-lg bg-sky-600 px-3 py-1.5 text-[13px] text-white hover:bg-sky-700"
              >
                Substituir
              </button>
              <button
                onClick={trocarDeLugar}
                className="rounded-lg border border-neutral-300 px-3 py-1.5 text-[13px] hover:bg-neutral-50 dark:border-neutral-600 dark:hover:bg-neutral-700"
              >
                Trocar de lugar
              </button>
              <button
                onClick={() => setConflito(null)}
                className="rounded-lg px-3 py-1.5 text-[13px] text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-700"
              >
                Cancelar
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Menu de contexto da página */}
      {menuPagina && (
        <div
          className="fixed z-50 min-w-40 rounded-lg border border-neutral-300 bg-white py-1 shadow-lg dark:border-neutral-600 dark:bg-neutral-800"
          style={{ left: menuPagina.x, top: menuPagina.y }}
          onClick={(e) => e.stopPropagation()}
        >
          <ItemMenu onClick={() => { renomearPagina(menuPagina.i); setMenuPagina(null); }}>
            Renomear
          </ItemMenu>
          <ItemMenu onClick={() => { duplicarPagina(menuPagina.i); setMenuPagina(null); }}>
            Duplicar
          </ItemMenu>
          <ItemMenu
            perigo
            onClick={() => { apagarPagina(menuPagina.i); setMenuPagina(null); }}
          >
            Apagar
          </ItemMenu>
        </div>
      )}

      {mostrarConfig && (
        <Configuracoes
          config={config}
          onFechar={() => setMostrarConfig(false)}
          onSalvar={salvar}
        />
      )}
    </div>
  );
}

function ItemMenu({
  children,
  onClick,
  perigo,
}: {
  children: React.ReactNode;
  onClick: () => void;
  perigo?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      className={`block w-full px-3 py-1.5 text-left text-[13px] hover:bg-neutral-100 dark:hover:bg-neutral-700 ${
        perigo ? "text-red-700 dark:text-red-400" : ""
      }`}
    >
      {children}
    </button>
  );
}
