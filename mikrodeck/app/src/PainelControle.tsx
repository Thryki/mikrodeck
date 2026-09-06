/**
 * Painel do controle selecionado: nome, ação, parâmetros e cores.
 *
 * Regra que veio do primeiro teste no aparelho (ver docs/ui-spec.md):
 * "abrir programa" não é campo de texto solto. Tem botão de procurar arquivo,
 * porque digitar nome de executável na mão falha de formas que o usuário não entende.
 */

import { useEffect, useState } from "react";
import { escolherPrograma as escolher } from "./ponte";
import PainelSample from "./PainelSample";
import SeletorApp from "./SeletorApp";
import { CORES, NOMES_CORES, rotuloDaCor, type NomeCor } from "./cores";
import type { Selecao } from "./Aparelho";
import {
  rotuloDoBotao,
  ROTULOS_ACAO,
  type MetodoHttp,
  ROTULOS_MIDIA,
  acaoVazia,
  type Acao,
  type Controle,
  type TeclaMidia,
} from "./tipos";

interface Props {
  selecao: Selecao;
  nomePagina: string;
  numeroPagina: number;
  totalPaginas: number;
  /** Brilho geral, usado quando o pad não tem brilho próprio. */
  brilhoGeral: number;
  /** Botão reservado para trocar de página: não aceita programação. */
  reservado?: boolean;
  controle?: Controle;
  onMudar: (controle: Controle | null) => void;
}

const CONTROLE_VAZIO: Controle = {
  nome: "",
  acao: { tipo: "nenhuma" },
  cor: "azul",
  cor_pressionado: null,
  cor_aberto: null,
  brilho: null,
  gerenciar_janela: false,
};

export function PainelControle({
  selecao,
  nomePagina,
  numeroPagina,
  totalPaginas,
  brilhoGeral,
  controle,
  onMudar,
  reservado = false,
}: Props) {
  const ehBotao = selecao.tipo === "botao";
  const titulo = ehBotao ? rotuloDoBotao(selecao.id) : `Pad ${selecao.pad}`;
  const atual = controle ?? CONTROLE_VAZIO;
  // Gravação de atalho: enquanto grava, o teclado inteiro vira entrada.
  const [gravando, setGravando] = useState(false);
  const [escolhendoApp, setEscolhendoApp] = useState(false);
  /** Teclas seguradas agora, na ordem em que foram apertadas. */
  const [seguradas, setSeguradas] = useState<string[]>([]);

  // Enquanto grava, o teclado inteiro vira entrada: nada mais recebe as teclas.
  //
  // O atalho é o CONJUNTO de teclas seguradas ao mesmo tempo, não a última
  // apertada. Por isso a gravação só fecha quando a pessoa solta tudo: até lá
  // ela pode ir somando teclas, e o campo mostra ao vivo o que já entrou.
  useEffect(() => {
    if (!gravando) return;

    let atuais: string[] = [];
    let maior: string[] = [];

    function aoTeclar(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setGravando(false);
        setSeguradas([]);
        return;
      }
      if (e.repeat) return;
      const nome = nomeDaTecla(e);
      if (!nome || atuais.includes(nome)) return;
      atuais = [...atuais, nome];
      // Guarda a maior combinação vista: soltar uma tecla não pode encolher o
      // atalho que a pessoa já tinha montado.
      if (atuais.length > maior.length) maior = atuais;
      setSeguradas(ordenar(atuais));
    }

    function aoSoltar(e: KeyboardEvent) {
      e.preventDefault();
      const nome = nomeDaTecla(e);
      if (nome) atuais = atuais.filter((t) => t !== nome);
      // Soltou tudo: fecha a gravação com a maior combinação vista.
      if (atuais.length === 0 && maior.length > 0) {
        onMudar({
          ...atual,
          acao: { tipo: "atalho", teclas: ordenar(maior).join("+") },
        });
        setGravando(false);
        setSeguradas([]);
      } else {
        setSeguradas(ordenar(atuais));
      }
    }

    // Sair da janela no meio da gravação deixaria teclas presas para sempre.
    function aoPerderFoco() {
      atuais = [];
      setSeguradas([]);
    }

    window.addEventListener("keydown", aoTeclar, true);
    window.addEventListener("keyup", aoSoltar, true);
    window.addEventListener("blur", aoPerderFoco);
    return () => {
      window.removeEventListener("keydown", aoTeclar, true);
      window.removeEventListener("keyup", aoSoltar, true);
      window.removeEventListener("blur", aoPerderFoco);
    };
  }, [gravando, atual, onMudar]);

  function mudar(parcial: Partial<Controle>) {
    onMudar({ ...atual, ...parcial });
  }

  async function escolherPrograma() {
    const caminho = await escolher();
    if (caminho) {
      const nomeArquivo = caminho.split(/[\\/]/).pop() ?? "";
      mudar({
        acao: { tipo: "abrir_programa", caminho, argumentos: [] },
        // Se o pad ainda não tem nome, sugere o do programa escolhido.
        nome: atual.nome || nomeBonito(nomeArquivo),
      });
    }
  }

  if (reservado) {
    return (
      <div className="flex flex-col gap-4">
        <div className="flex items-baseline justify-between">
          <span className="text-[15px] font-medium">{titulo}</span>
          <span className="text-xs text-neutral-500">reservado</span>
        </div>
        <div className="rounded-lg border border-amber-300 bg-amber-50 p-3 dark:border-amber-800 dark:bg-amber-950/40">
          <p className="text-[13px] leading-relaxed text-amber-900 dark:text-amber-200">
            Este botão está guardado para trocar de página, então não aceita
            outra função. Se ele pudesse ser reprogramado, você ficaria sem como
            sair da página.
          </p>
          <p className="mt-2 text-xs text-amber-800 dark:text-amber-300">
            Para liberar, escolha outro botão em Configurações → Navegação por
            páginas. Este volta a aceitar programação na hora.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-baseline justify-between">
        <span className="text-[15px] font-medium">{titulo}</span>
        <span className="text-xs text-neutral-500">
          Página {numeroPagina} · {nomePagina}
        </span>
      </div>

      <Campo rotulo="Nome">
        <input
          type="text"
          value={atual.nome}
          placeholder="Ex.: Chrome"
          onChange={(e) => mudar({ nome: e.target.value })}
          className={entrada}
        />
      </Campo>

      <Campo rotulo="Ação ao apertar">
        <select
          value={atual.acao.tipo}
          onChange={(e) =>
            mudar({ acao: acaoVazia(e.target.value as Acao["tipo"]) })
          }
          className={entrada}
        >
          {Object.entries(ROTULOS_ACAO).map(([tipo, rotulo]) => (
            <option key={tipo} value={tipo}>
              {rotulo}
            </option>
          ))}
        </select>
      </Campo>

      {/* Parâmetros, um por tipo de ação */}
      {atual.acao.tipo === "abrir_programa" && (
        <Campo rotulo="Programa">
          <div className="flex gap-2">
            <input
              type="text"
              value={atual.acao.caminho}
              placeholder="Escolha um programa"
              onChange={(e) =>
                mudar({
                  acao: { ...atual.acao, caminho: e.target.value } as Acao,
                })
              }
              className={`${entrada} flex-1 text-xs`}
            />
            <button
              onClick={() => setEscolhendoApp(true)}
              className={botao}
              title="Escolher da lista do menu Iniciar"
            >
              Escolher
            </button>
            <button
              onClick={escolherPrograma}
              className={botao}
              title="Procurar o arquivo no disco"
            >
              Arquivo
            </button>
          </div>
          <p className="mt-1 text-xs text-neutral-500">
            "Escolher" lista o que está no menu Iniciar, inclusive apps da
            Microsoft Store, que não abrem pelo arquivo.
          </p>
          <label className="mt-2 flex items-start gap-2 text-xs text-neutral-600 dark:text-neutral-400">
            <input
              type="checkbox"
              checked={atual.gerenciar_janela ?? false}
              onChange={(e) => mudar({ gerenciar_janela: e.target.checked })}
              className="mt-0.5"
            />
            <span>
              Cuidar da janela
              <span className="block text-neutral-500">
                Apertar traz para a frente ou minimiza, segurar fecha. Com isso
                ligado o pad age quando você solta, não quando aperta.
              </span>
            </span>
          </label>
        </Campo>
      )}

      {atual.acao.tipo === "abrir_url" && (
        <Campo rotulo="Link">
          <input
            type="text"
            value={atual.acao.url}
            placeholder="youtube.com"
            onChange={(e) =>
              mudar({ acao: { tipo: "abrir_url", url: e.target.value } })
            }
            className={entrada}
          />
          <p className="mt-1 text-xs text-neutral-500">
            Pode digitar só o endereço: o https vai sozinho.
          </p>
          <label className="mt-2 flex items-start gap-2 text-xs text-neutral-600 dark:text-neutral-400">
            <input
              type="checkbox"
              checked={atual.gerenciar_janela ?? false}
              onChange={(e) => mudar({ gerenciar_janela: e.target.checked })}
              className="mt-0.5"
            />
            <span>
              Cuidar da janela
              <span className="block text-neutral-500">
                Toque vai para a janela do site se ela já estiver aberta, em vez
                de abrir outra. Dois toques rápidos maximizam. Segurar abre uma
                janela nova mesmo já tendo uma.
              </span>
            </span>
          </label>
        </Campo>
      )}

      {atual.acao.tipo === "comando" && (
        <Campo rotulo="Comando">
          <input
            type="text"
            value={atual.acao.linha}
            placeholder="Ex.: shutdown /h"
            onChange={(e) =>
              mudar({ acao: { tipo: "comando", linha: e.target.value } })
            }
            className={entrada}
          />
        </Campo>
      )}

      {atual.acao.tipo === "atalho" && (
        <Campo rotulo="Teclas">
          <div className="flex gap-2">
            <input
              type="text"
              value={gravando ? seguradas.join(" + ") : atual.acao.teclas}
              placeholder={gravando ? "Segure as teclas…" : "Ex.: ctrl+shift+n"}
              onChange={(e) =>
                mudar({ acao: { tipo: "atalho", teclas: e.target.value } })
              }
              readOnly={gravando}
              className={`${entrada} flex-1 ${
                gravando ? "border-sky-500 text-sky-600 dark:text-sky-400" : ""
              }`}
            />
            <button
              onClick={() => {
                setSeguradas([]);
                setGravando((v) => !v);
              }}
              className={
                gravando
                  ? "rounded-lg border border-sky-500 px-3 py-1.5 text-[13px] text-sky-600 dark:text-sky-400"
                  : botao
              }
            >
              {gravando ? "Cancelar" : "Gravar"}
            </button>
          </div>
          <p className="mt-1 text-xs text-neutral-500">
            {gravando
              ? "Segure todas as teclas do atalho ao mesmo tempo. Grava quando você soltar. Esc cancela."
              : "Clique em Gravar e aperte o atalho, ou digite: ctrl, shift, alt, win, f1 a f24, letras e números, separados por +."}
          </p>
          {gravando && (
            <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
              Combinações que o Windows reserva, como win+L, ele engole antes de
              chegar aqui. Essas precisam ser digitadas na mão.
            </p>
          )}
        </Campo>
      )}

      {atual.acao.tipo === "midia" && (
        <Campo rotulo="Comando de mídia">
          <select
            value={atual.acao.tecla}
            onChange={(e) =>
              mudar({
                acao: { tipo: "midia", tecla: e.target.value as TeclaMidia },
              })
            }
            className={entrada}
          >
            {Object.entries(ROTULOS_MIDIA).map(([tecla, rotulo]) => (
              <option key={tecla} value={tecla}>
                {rotulo}
              </option>
            ))}
          </select>
        </Campo>
      )}

      {atual.acao.tipo === "ir_para_pagina" && (
        <Campo rotulo="Número da página">
          <input
            type="number"
            min={1}
            max={totalPaginas}
            value={atual.acao.numero}
            onChange={(e) =>
              mudar({
                acao: { tipo: "ir_para_pagina", numero: Number(e.target.value) },
              })
            }
            className={entrada}
          />
        </Campo>
      )}

      {atual.acao.tipo === "home_assistant" && (
        <>
          <Campo rotulo="Serviço">
            <input
              type="text"
              value={atual.acao.servico}
              placeholder="Ex.: light.toggle"
              onChange={(e) =>
                mudar({ acao: { ...atual.acao, servico: e.target.value } as Acao })
              }
              className={entrada}
            />
            <p className="mt-1 text-xs text-neutral-500">
              Domínio e serviço separados por ponto. Ex.: light.toggle,
              switch.turn_on, scene.turn_on, script.turn_on.
            </p>
          </Campo>
          <Campo rotulo="Entidade">
            <input
              type="text"
              value={atual.acao.entidade}
              placeholder="Ex.: light.sala"
              onChange={(e) =>
                mudar({ acao: { ...atual.acao, entidade: e.target.value } as Acao })
              }
              className={entrada}
            />
            <p className="mt-1 text-xs text-neutral-500">
              Deixe vazio para serviços que não precisam de entidade. O endereço
              e o token ficam em Configurações.
            </p>
          </Campo>
        </>
      )}

      {atual.acao.tipo === "http" && (
        <>
          <Campo rotulo="Endereço">
            <input
              type="text"
              value={atual.acao.url}
              placeholder="https://"
              onChange={(e) =>
                mudar({ acao: { ...atual.acao, url: e.target.value } as Acao })
              }
              className={entrada}
            />
          </Campo>
          <Campo rotulo="Método">
            <select
              value={atual.acao.metodo}
              onChange={(e) =>
                mudar({
                  acao: { ...atual.acao, metodo: e.target.value as MetodoHttp } as Acao,
                })
              }
              className={entrada}
            >
              <option value="post">POST</option>
              <option value="get">GET</option>
              <option value="put">PUT</option>
            </select>
          </Campo>
          {atual.acao.metodo !== "get" && (
            <Campo rotulo="Corpo (JSON)">
              <textarea
                value={atual.acao.corpo ?? ""}
                rows={3}
                placeholder={'{ "chave": "valor" }'}
                onChange={(e) =>
                  mudar({
                    acao: {
                      ...atual.acao,
                      corpo: e.target.value === "" ? null : e.target.value,
                    } as Acao,
                  })
                }
                className={`${entrada} font-mono text-xs`}
              />
            </Campo>
          )}
        </>
      )}

      {escolhendoApp && (
        <SeletorApp
          onFechar={() => setEscolhendoApp(false)}
          onEscolher={(app) => {
            setEscolhendoApp(false);
            mudar({
              acao: { tipo: "abrir_programa", caminho: app.caminho, argumentos: [] },
              // Nome vazio ainda: batiza com o nome do programa, que é o que a
              // pessoa ia digitar de qualquer jeito.
              ...(atual.nome.trim() === "" ? { nome: app.nome } : {}),
            });
          }}
        />
      )}

      {atual.acao.tipo === "sample" && (
        <PainelSample
          acao={atual.acao}
          nome={atual.nome}
          onMudar={(acao) => mudar({ acao })}
        />
      )}

      {ehBotao ? (
        <p className="text-xs text-neutral-500">
          Botões do aparelho são monocromáticos: eles acendem quando têm ação,
          e apagam quando não têm. Cor só existe nos pads.
        </p>
      ) : (
        <>
          <Campo rotulo="Cor em repouso">
            <SeletorCor valor={atual.cor} onEscolher={(cor) => mudar({ cor })} />
          </Campo>

          <Campo rotulo="Cor ao apertar">
            <SeletorCor
              valor={atual.cor_pressionado ?? "branco"}
              onEscolher={(cor) => mudar({ cor_pressionado: cor })}
            />
          </Campo>

          {atual.acao.tipo === "abrir_programa" && (
            <Campo rotulo="Cor quando o programa está aberto">
              <label className="mb-2 flex items-center gap-2 text-xs text-neutral-600 dark:text-neutral-400">
                <input
                  type="checkbox"
                  checked={atual.cor_aberto != null}
                  onChange={(e) =>
                    mudar({ cor_aberto: e.target.checked ? "verde" : null })
                  }
                />
                Mudar de cor sozinho quando o programa estiver rodando
              </label>
              {atual.cor_aberto != null && (
                <SeletorCor
                  valor={atual.cor_aberto}
                  onEscolher={(cor) => mudar({ cor_aberto: cor })}
                />
              )}
            </Campo>
          )}

          <Campo rotulo="Brilho deste pad">
            <div className="flex items-center gap-3">
              <input
                type="range"
                min={0}
                max={3}
                value={atual.brilho ?? brilhoGeral}
                onChange={(e) => mudar({ brilho: Number(e.target.value) })}
                className="flex-1"
                disabled={atual.brilho == null}
              />
              <label className="flex shrink-0 items-center gap-1.5 text-xs text-neutral-600 dark:text-neutral-400">
                <input
                  type="checkbox"
                  checked={atual.brilho == null}
                  onChange={(e) =>
                    mudar({ brilho: e.target.checked ? null : brilhoGeral })
                  }
                />
                usar o geral
              </label>
            </div>
          </Campo>
        </>
      )}

      {controle && (
        <button
          onClick={() => onMudar(null)}
          className="mt-2 self-start text-xs text-red-700 hover:underline dark:text-red-400"
        >
          {ehBotao ? "Limpar este botão" : "Limpar este pad"}
        </button>
      )}
    </div>
  );
}

function Campo({
  rotulo,
  children,
}: {
  rotulo: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <label className="mb-1 block text-xs text-neutral-600 dark:text-neutral-400">
        {rotulo}
      </label>
      {children}
    </div>
  );
}

function SeletorCor({
  valor,
  onEscolher,
}: {
  valor: NomeCor;
  onEscolher: (cor: NomeCor) => void;
}) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {NOMES_CORES.map((nome) => (
        <button
          key={nome}
          onClick={() => onEscolher(nome)}
          title={rotuloDaCor(nome)}
          aria-label={rotuloDaCor(nome)}
          className={`h-6 w-6 rounded transition ${
            valor === nome
              ? "ring-2 ring-neutral-900 ring-offset-1 dark:ring-neutral-100"
              : "hover:scale-110"
          }`}
          style={{ background: CORES[nome] }}
        />
      ))}
    </div>
  );
}

/** "Google Chrome.exe" vira "Google Chrome". */
function nomeBonito(arquivo: string): string {
  return arquivo.replace(/\.(exe|lnk|bat|cmd)$/i, "");
}

/** Modificadores primeiro e na ordem de sempre; o resto na ordem em que veio. */
const ORDEM_MODIFICADORES = ["ctrl", "shift", "alt", "win"];

function ordenar(teclas: string[]): string[] {
  const mods = ORDEM_MODIFICADORES.filter((m) => teclas.includes(m));
  const resto = teclas.filter((t) => !ORDEM_MODIFICADORES.includes(t));
  return [...mods, ...resto];
}

/**
 * Nome da tecla no vocabulário do motor.
 *
 * Modificador conta como tecla de verdade: apertar só o Windows é um atalho
 * válido, e é o que a pessoa espera ver escrito quando aperta só ele.
 */
function nomeDaTecla(e: KeyboardEvent): string | null {
  const k = e.key;
  if (k === "Control") return "ctrl";
  if (k === "Shift") return "shift";
  if (k === "Alt") return "alt";
  if (k === "Meta" || k === "OS") return "win";
  if (k === "Dead") return null;

  const especiais: Record<string, string> = {
    ArrowUp: "cima",
    ArrowDown: "baixo",
    ArrowLeft: "esquerda",
    ArrowRight: "direita",
    Enter: "enter",
    Tab: "tab",
    Escape: "esc",
    " ": "espaco",
    Backspace: "backspace",
    Delete: "delete",
    Home: "home",
    End: "end",
    PageUp: "pageup",
    PageDown: "pagedown",
    PrintScreen: "printscreen",
  };
  if (especiais[k]) return especiais[k];
  if (/^F\d{1,2}$/.test(k)) return k.toLowerCase();
  if (k.length === 1) return k.toLowerCase();
  // Tecla que o motor não conhece: melhor não gravar lixo.
  return null;
}

const entrada =
  "w-full rounded-lg border border-neutral-300 bg-transparent px-2.5 py-1.5 text-sm outline-none focus:border-sky-500 dark:border-neutral-600";
const botao =
  "rounded-lg border border-neutral-300 px-3 py-1.5 text-xs hover:bg-neutral-50 dark:border-neutral-600 dark:hover:bg-neutral-700";
