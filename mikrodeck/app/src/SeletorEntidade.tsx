/**
 * Escolher uma entidade do Home Assistant pelo nome.
 *
 * Além de achar a entidade, ele já traz o serviço certo para o tipo dela: cena
 * e script disparam com `turn_on`, botão com `press`, o resto alterna. Digitar
 * isso na mão era onde a pessoa errava sem saber por quê.
 */

import { useEffect, useState } from "react";
import { entidadesDaCasa, type EntidadeCasa } from "./ponte";
import SeletorLista, { type ItemDaLista } from "./SeletorLista";

interface Props {
  onEscolher: (entidade: EntidadeCasa) => void;
  onFechar: () => void;
}

export default function SeletorEntidade({ onEscolher, onFechar }: Props) {
  const [entidades, setEntidades] = useState<EntidadeCasa[] | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    entidadesDaCasa().then(setEntidades).catch((e) => setErro(String(e)));
  }, []);

  const itens: ItemDaLista[] | null =
    entidades?.map((e) => ({
      id: e.id,
      titulo: e.nome,
      // O identificador vale como detalhe: é por ele que se busca quando dois
      // cômodos têm o mesmo nome amigável.
      detalhe: e.id,
      etiqueta: e.estado_conhecido ? (e.ligada ? "ligado" : "desligado") : undefined,
    })) ?? null;

  return (
    <SeletorLista
      titulo="Digite o nome da entidade"
      itens={itens}
      erro={erro}
      vazio="Nada com esse nome. Confira o endereço e o token em Configurações."
      onFechar={onFechar}
      onEscolher={(item) => {
        const achada = entidades?.find((e) => e.id === item.id);
        if (achada) onEscolher(achada);
      }}
    />
  );
}
