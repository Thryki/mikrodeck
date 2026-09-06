/**
 * O painel do som no pad: escolher o arquivo, gravar do microfone, decidir se
 * toca até o fim ou enquanto está apertado, e ajustar o envelope.
 *
 * O limite de gravação é curto de propósito, como o Davi pediu: trinta a
 * quarenta segundos, um minuto no máximo. Isso é sample, não podcast.
 */

import { useEffect, useRef, useState } from "react";
import {
  escolherSample,
  gravarSample,
  microfones as lerMicrofones,
  pararGravacao,
  tempoDeGravacao,
  testarSample,
} from "./ponte";
import EnvelopeGrafico from "./EnvelopeGrafico";
import FormaDeOnda from "./FormaDeOnda";
import {
  ENVELOPE_PADRAO,
  nomeDoArquivo,
  ROTULOS_MODO_DISPARO,
  type Acao,
  type Envelope,
  type Microfone,
  type ModoDisparo,
} from "./tipos";

/** Quanto tempo a gravação pode durar, em segundos. */
const SEGUNDOS_DE_GRAVACAO = 40;

interface Props {
  acao: Extract<Acao, { tipo: "sample" }>;
  /** Nome do controle, usado para batizar o arquivo gravado. */
  nome: string;
  onMudar: (acao: Acao) => void;
}

const entrada =
  "w-full rounded-md border border-neutral-300 bg-white px-2 py-1.5 text-[13px] outline-none focus:border-sky-500 dark:border-neutral-700 dark:bg-neutral-900";
const botao =
  "rounded-md border border-neutral-300 px-2.5 py-1.5 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800";

export default function PainelSample({ acao, nome, onMudar }: Props) {
  const [micros, setMicros] = useState<Microfone[]>([]);
  const [microfone, setMicrofone] = useState<string>("");
  const [gravando, setGravando] = useState(false);
  const [segundos, setSegundos] = useState(0);
  const [recado, setRecado] = useState<string | null>(null);
  // Sobe a cada gravação. É o que manda a forma de onda ser lida de novo: o
  // caminho do arquivo continua o mesmo, só o conteúdo muda.
  const [versao, setVersao] = useState(0);
  const relogio = useRef<number | null>(null);

  useEffect(() => {
    lerMicrofones()
      .then((lista) => {
        setMicros(lista);
        const padrao = lista.find((m) => m.padrao) ?? lista[0];
        if (padrao) setMicrofone(padrao.nome);
      })
      .catch(() => setMicros([]));
  }, []);

  // O contador do tempo gravado vem do motor, não do relógio da interface: é
  // ele que sabe quantos quadros o microfone entregou de verdade.
  useEffect(() => {
    if (!gravando) {
      if (relogio.current) window.clearInterval(relogio.current);
      relogio.current = null;
      return;
    }
    relogio.current = window.setInterval(() => {
      tempoDeGravacao()
        .then((t) => {
          setSegundos(t);
          // O motor para sozinho no limite; a interface acompanha.
          if (t >= SEGUNDOS_DE_GRAVACAO) parar();
        })
        .catch(() => {});
    }, 200);
    return () => {
      if (relogio.current) window.clearInterval(relogio.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gravando]);

  const mudarEnvelope = (mudanca: Partial<Envelope>) =>
    onMudar({ ...acao, envelope: { ...acao.envelope, ...mudanca } });

  async function comecar() {
    setRecado(null);
    try {
      await gravarSample(nome || "sample", microfone || null, SEGUNDOS_DE_GRAVACAO);
      setSegundos(0);
      setGravando(true);
    } catch (e) {
      setRecado(String(e));
    }
  }

  async function parar() {
    try {
      const caminho = await pararGravacao();
      onMudar({ ...acao, caminho });
      setVersao((v) => v + 1);
      setRecado("Gravado.");
    } catch (e) {
      setRecado(String(e));
    } finally {
      setGravando(false);
    }
  }

  return (
    <div className="space-y-3">
      <div>
        <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Som
        </label>
        <div className="flex gap-2">
          <input
            type="text"
            value={acao.caminho}
            readOnly
            placeholder="nenhum som escolhido"
            title={acao.caminho}
            className={`${entrada} flex-1`}
          />
          <button
            onClick={() =>
              escolherSample().then((c) => c && onMudar({ ...acao, caminho: c }))
            }
            className={botao}
          >
            Procurar
          </button>
        </div>
        {acao.caminho && (
          <>
            <div className="mt-1 flex items-center justify-between">
              <p className="truncate text-xs text-neutral-500">
                {nomeDoArquivo(acao.caminho)}
              </p>
              <button
                onClick={() =>
                  testarSample(acao.caminho, acao.volume).catch((e) =>
                    setRecado(String(e)),
                  )
                }
                className="shrink-0 text-xs text-sky-600 hover:underline dark:text-sky-400"
              >
                Ouvir
              </button>
            </div>
            <div className="mt-1">
              <FormaDeOnda caminho={acao.caminho} versao={versao} />
            </div>
          </>
        )}
      </div>

      <div className="rounded-lg border border-neutral-200 p-3 dark:border-neutral-700">
        <p className="mb-2 text-[13px] font-medium">Gravar do microfone</p>
        {micros.length === 0 ? (
          <p className="text-xs text-neutral-500">
            Não achei microfone nenhum ligado neste computador.
          </p>
        ) : (
          <>
            <select
              value={microfone}
              onChange={(e) => setMicrofone(e.target.value)}
              disabled={gravando}
              className={`${entrada} mb-2 disabled:opacity-40`}
            >
              {micros.map((m) => (
                <option key={m.nome} value={m.nome}>
                  {m.nome}
                  {m.padrao ? " (padrão)" : ""}
                </option>
              ))}
            </select>
            <div className="flex items-center gap-2">
              <button
                onClick={() => (gravando ? parar() : comecar())}
                className={`${botao} ${
                  gravando ? "border-red-500 text-red-600 dark:text-red-400" : ""
                }`}
              >
                {gravando ? "Parar" : "Gravar"}
              </button>
              <span className="text-xs tabular-nums text-neutral-500">
                {gravando
                  ? `${segundos.toFixed(1)} s de ${SEGUNDOS_DE_GRAVACAO} s`
                  : `no máximo ${SEGUNDOS_DE_GRAVACAO} s`}
              </span>
            </div>
          </>
        )}
        {recado && <p className="mt-2 text-xs text-neutral-500">{recado}</p>}
      </div>

      <div>
        <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Como dispara
        </label>
        <select
          value={acao.modo}
          onChange={(e) => onMudar({ ...acao, modo: e.target.value as ModoDisparo })}
          className={entrada}
        >
          {Object.entries(ROTULOS_MODO_DISPARO).map(([valor, texto]) => (
            <option key={valor} value={valor}>
              {texto}
            </option>
          ))}
        </select>
      </div>

      <div>
        <label className="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400">
          Volume: {Math.round(acao.volume * 100)}%
        </label>
        <input
          type="range"
          min={0}
          max={100}
          value={Math.round(acao.volume * 100)}
          onChange={(e) => onMudar({ ...acao, volume: Number(e.target.value) / 100 })}
          className="w-full"
        />
      </div>

      <div className="rounded-lg border border-neutral-200 p-3 dark:border-neutral-700">
        <div className="mb-2 flex items-center justify-between">
          <p className="text-[13px] font-medium">Envelope</p>
          <button
            onClick={() => onMudar({ ...acao, envelope: { ...ENVELOPE_PADRAO } })}
            className="text-xs text-sky-600 hover:underline dark:text-sky-400"
          >
            Voltar ao padrão
          </button>
        </div>

        <EnvelopeGrafico
          envelope={acao.envelope}
          onMudar={(envelope) => onMudar({ ...acao, envelope })}
        />

        <p className="mt-2 text-xs text-neutral-500">
          Arraste as bolinhas. A da sustentação também sobe e desce.
        </p>

        <div className="mt-3 space-y-2">
          <Tempo
            rotulo="Atraso"
            valor={acao.envelope.atraso_ms}
            onMudar={(v) => mudarEnvelope({ atraso_ms: v })}
          />
          <Tempo
            rotulo="Ataque"
            valor={acao.envelope.ataque_ms}
            onMudar={(v) => mudarEnvelope({ ataque_ms: v })}
          />
          <Tempo
            rotulo="Retenção"
            valor={acao.envelope.retencao_ms}
            onMudar={(v) => mudarEnvelope({ retencao_ms: v })}
          />
          <Tempo
            rotulo="Decaimento"
            valor={acao.envelope.decaimento_ms}
            onMudar={(v) => mudarEnvelope({ decaimento_ms: v })}
          />
          <Deslizante
            rotulo="Sustentação"
            valor={Math.round(acao.envelope.sustentacao * 100)}
            unidade="%"
            onMudar={(v) => mudarEnvelope({ sustentacao: v / 100 })}
          />
          <Tempo
            rotulo="Liberação"
            valor={acao.envelope.liberacao_ms}
            onMudar={(v) => mudarEnvelope({ liberacao_ms: v })}
          />
          <Deslizante
            rotulo="Curva da subida"
            valor={Math.round(acao.envelope.tensao_ataque * 100)}
            minimo={-100}
            onMudar={(v) => mudarEnvelope({ tensao_ataque: v / 100 })}
          />
          <Deslizante
            rotulo="Curva da descida"
            valor={Math.round(acao.envelope.tensao_queda * 100)}
            minimo={-100}
            onMudar={(v) => mudarEnvelope({ tensao_queda: v / 100 })}
          />
        </div>

        <p className="mt-2 text-xs text-neutral-500">
          A liberação só aparece no modo que toca enquanto está apertado: é o
          tempo que o som leva para sumir depois que você solta.
        </p>
      </div>
    </div>
  );
}

/** Teto de um tempo do envelope, em milissegundos. */
const MAXIMO_MS = 8000;

/**
 * O slider vai de 0 a 100, mas o tempo cresce ao quadrado.
 *
 * Numa régua linear até 8 segundos, cada passo valeria 80 ms e os valores
 * curtos, que são os que mais importam num envelope, ficariam inalcançáveis.
 * Assim 25 dá 500 ms e 50 dá 2 s, com passo fino embaixo.
 */
function msDoSlider(v: number): number {
  const f = Math.min(100, Math.max(0, v)) / 100;
  return Math.round(f * f * MAXIMO_MS);
}

function sliderDoMs(ms: number): number {
  return Math.round(Math.sqrt(Math.min(MAXIMO_MS, Math.max(0, ms)) / MAXIMO_MS) * 100);
}

/** Um tempo do envelope: régua de 0 a 100 e o valor em milissegundos. */
function Tempo({
  rotulo,
  valor,
  onMudar,
}: {
  rotulo: string;
  valor: number;
  onMudar: (v: number) => void;
}) {
  return (
    <div className="flex items-center gap-2">
      <label className="w-24 shrink-0 text-xs text-neutral-600 dark:text-neutral-400">
        {rotulo}
      </label>
      <input
        type="range"
        min={0}
        max={100}
        value={sliderDoMs(valor)}
        onChange={(e) => onMudar(msDoSlider(Number(e.target.value)))}
        className="min-w-0 flex-1"
      />
      <div className="flex shrink-0 items-center gap-1">
        <input
          type="number"
          min={0}
          max={MAXIMO_MS}
          step={10}
          value={valor}
          onChange={(e) =>
            onMudar(Math.min(MAXIMO_MS, Math.max(0, Number(e.target.value))))
          }
          className={`${entrada} w-20 text-right`}
        />
        <span className="text-xs text-neutral-500">ms</span>
      </div>
    </div>
  );
}

/** Uma régua simples, para o que já é uma porcentagem. */
function Deslizante({
  rotulo,
  valor,
  onMudar,
  minimo = 0,
  unidade = "",
}: {
  rotulo: string;
  valor: number;
  onMudar: (v: number) => void;
  minimo?: number;
  unidade?: string;
}) {
  return (
    <div className="flex items-center gap-2">
      <label className="w-24 shrink-0 text-xs text-neutral-600 dark:text-neutral-400">
        {rotulo}
      </label>
      <input
        type="range"
        min={minimo}
        max={100}
        value={valor}
        onChange={(e) => onMudar(Number(e.target.value))}
        onDoubleClick={() => onMudar(minimo < 0 ? 0 : 100)}
        className="min-w-0 flex-1"
      />
      <div className="flex shrink-0 items-center gap-1">
        <input
          type="number"
          min={minimo}
          max={100}
          value={valor}
          onChange={(e) =>
            onMudar(Math.min(100, Math.max(minimo, Number(e.target.value))))
          }
          className={`${entrada} w-20 text-right`}
        />
        <span className="w-4 text-xs text-neutral-500">{unidade}</span>
      </div>
    </div>
  );
}
