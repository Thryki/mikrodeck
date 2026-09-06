/**
 * Escolher um programa pelo nome, como no menu Iniciar.
 *
 * Existe porque caçar o executável no disco não funciona: app da Microsoft
 * Store mora numa pasta protegida, e o nome do arquivo raramente é o nome do
 * programa. Aqui a pessoa digita "ray" e acha o Raycast.
 */

import { useEffect, useState } from "react";
import { listarApps, type AppInstalado } from "./ponte";
import SeletorLista, { type ItemDaLista } from "./SeletorLista";

interface Props {
  onEscolher: (app: AppInstalado) => void;
  onFechar: () => void;
}

export default function SeletorApp({ onEscolher, onFechar }: Props) {
  const [apps, setApps] = useState<AppInstalado[] | null>(null);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    listarApps().then(setApps).catch((e) => setErro(String(e)));
  }, []);

  const itens: ItemDaLista[] | null =
    apps?.map((a) => ({
      id: a.caminho,
      titulo: a.nome,
      etiqueta: a.da_loja ? "Store" : undefined,
    })) ?? null;

  return (
    <SeletorLista
      titulo="Digite o nome do programa"
      itens={itens}
      erro={erro}
      vazio="Nada com esse nome. O programa precisa aparecer no menu Iniciar do Windows para entrar nesta lista."
      onFechar={onFechar}
      onEscolher={(item) => {
        const achado = apps?.find((a) => a.caminho === item.id);
        if (achado) onEscolher(achado);
      }}
    />
  );
}
