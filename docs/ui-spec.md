# Pedidos do Davi na primeira rodada de uso real (2026-09-02)

Lista completa, em ordem de prioridade decidida junto. Nada aqui foi inventado.

## Fazer agora

1. **Janela responsiva.** A interface fica parada no meio quando a janela cresce.
   Tem que acompanhar o tamanho.
2. **Tela do aparelho vazia.** Precisa mostrar: nome da página ao trocar, e o nome do
   controle ao apertar. Ver a seção "Tela do aparelho" abaixo.
3. **Brilho individual por pad**, além do brilho geral. O controle de cima é o geral e
   precisa estar rotulado como tal.
4. **Ícones no lugar de emoji** nos botões do desenho.
5. **Funções padrão nos três botões da coluna esquerda**, todos remapeáveis:
   - Círculo dentro de círculo (`maschine`): liga e desliga o MikroDeck.
   - Estrela (`estrela`): abre a página de favoritos.
   - Busca (`busca`): tecla Windows, a busca do sistema.
6. **Touch strip configurável**: volume, brilho dos pads, brilho da tela, e o que mais
   fizer sentido para um controle deslizante.
7. **Garantir que o aparelho não está em modo MIDI ao abrir o MikroDeck.** Em modo MIDI
   o comportamento fica estranho.

## Fazer depois

8. **Cor para "aplicativo aberto"**, o terceiro estado que já estava previsto. Depende
   dos watchers.
9. **Comportamento de toque por pressão**: pressão leve mostra na tela o que o pad faz,
   sem executar; pressão firme executa. A leitura de pressão já existe (0 a 4095).
10. **Aperta, aperta de novo, segura**: abrir o app, minimizar, e fechar segurando.
11. **Sons**: abertura, volume subindo e descendo, mudo.

## Páginas de exemplo para o Davi (pedido em 2026-09-02)

Para depois que a ferramenta estiver redonda. Ele pediu explicitamente:

- **Spotify**: uma página para controlar o Spotify inteiro, usando o PLAY e o STOP físicos.
- **Claude**: página com botões que abrem chat novo e o Claude design.
- Configurar com critério, aproveitando o que ele usa no dia a dia.

## Servidor MCP embutido (pedido em 2026-09-02)

O app expõe um servidor MCP para qualquer assistente configurar o MikroDeck por
linguagem natural. O caso de uso é o usuário final: baixou, instalou, está com preguiça
de configurar na mão, conecta o assistente e pede "coloca o Spotify no pad 5, verde".

Não é para o desenvolvedor mexer no repositório. É para quem só usa o produto.

Ferramentas que o servidor precisa expor:

| Ferramenta | O que faz |
|---|---|
| `listar_paginas` | nomes, números e quantos controles cada uma tem |
| `ler_pagina` | todos os pads e botões de uma página, com ação e cor |
| `configurar_pad` | nome, ação, cor e brilho de um pad |
| `configurar_botao` | o mesmo para um botão físico |
| `limpar_controle` | apaga a configuração de um pad ou botão |
| `criar_pagina`, `renomear_pagina`, `apagar_pagina` | gestão de páginas |
| `listar_acoes` | os tipos de ação e os parâmetros de cada um |
| `listar_cores` | as 18 cores válidas |
| `estado_do_aparelho` | conectado ou não, página atual |

Regras: toda escrita valida antes de aplicar, salva no mesmo `config.json`, e o motor
aplica na hora sem reiniciar. Erro precisa ser em texto claro, porque quem lê é um
assistente que vai tentar de novo.

## Ideias para o futuro

12. **Cores animadas**: alternar entre cores, transição suave, faixa de cor escolhida,
    no estilo dos aplicativos de RGB.
13. **Efeitos de LED**: visualizador de frequência reagindo ao som, animação de
    carregamento em espiral ou ziguezague, barrinha enchendo.

# Especificação da UI

Referência de sensação: Logitech Options / G Hub. Qualquer leigo configura; quem quer tudo, encontra tudo.

## Tela principal

Barra superior
- Status: "Mikro MK3 conectado" com ícone verde, ou "desconectado" em cinza.
- Seletor de perfil (Trabalho, Música, Stream...).
- Engrenagem: configurações gerais.

Abas de páginas
- Cada página = 16 pads. Aba ativa em destaque. Botão "Nova página".
- Arrastar para reordenar.

Desenho do aparelho (componente central)
- Réplica fiel do Mikro MK3 (`design/mikro-mk3-v2.svg` como base).
- Todo controle é clicável: pads, botões, knob, strip, tela.
- As cores dos pads no desenho = cores reais de repouso da página atual.
- Ao apertar um pad físico, o pad no desenho pisca (feedback ao vivo via IPC).

Painel lateral (controle selecionado)
- Cabeçalho: "Pad 13 · Página 1 · Apps".
- Nome.
- Ação ao apertar (select): Abrir programa, Atalho de teclado, Rodar script, Comando, Abrir link, Mídia, Trocar página.
- Parâmetros da ação (caminho com botão de pasta, campo de atalho que captura teclas, etc.).
- Cores por estado: Repouso, Pressionado, Ativo (programa aberto). Seletor de cor com paleta + hex.
- Texto na tela ao apertar.
- Botões: "Testar no aparelho" (aplica sem salvar) e "Salvar".

## Knob
- Girar: volume, scroll, brilho dos pads, trocar página.
- Apertar: mute, confirmar, ação livre.
- Pode mudar de função por página.

## Touch strip
- Modos: slider de volume, brilho, barra de progresso (mídia), indicador da página.

## Tela
- Linha 1: nome da página + N/total.
- Linha 2: nome do pad sendo segurado, ou status (volume, relógio, gravando).

## Lição do primeiro teste no aparelho (2026-09-02)

Dois problemas apareceram assim que o motor foi usado de verdade. Os dois viram regra de UI:

**Abrir programa não pode ser campo de texto solto.** Digitar "chrome" falhou porque o Windows
só resolve nomes curtos por uma chave de registro que a chamada direta não consulta (corrigido
no motor com fallback pelo `start`). Mas mesmo corrigido, o usuário não deve digitar nome de
executável. O campo precisa de: botão de procurar arquivo, lista dos programas instalados, e
validação na hora ("esse programa não existe neste computador"). O caso da calculadora, que o
Davi não tem instalada, mostra que validar na hora importa.

**Botão de encerrar não pode ser um botão só.** A primeira versão encerrava no SHIFT, que é
modificador e todo mundo aperta sem querer, e depois não havia como reabrir pelo aparelho.
Virou SHIFT + STOP, com os dois acesos fracos para o combo ficar visível. Regra geral: ação
destrutiva ou irreversível no aparelho pede combo, e o combo tem que estar visível nos LEDs.

## Tela do aparelho (128x32, 1 bit por pixel)

Quatro estados, nesta prioridade:

1. **Segurando um controle**: mostra o nome do que está sendo apertado. Some ao soltar.
2. **Normal**: nome da página e a posição, por exemplo "Apps 1/3".
3. **Feedback de ação**: quando uma ação tem estado que dá para mostrar, a tela mostra por
   uns segundos e volta ao normal. O caso principal é volume: aparece o número e uma barra
   cheia até a proporção certa. Vale também para mudo (ícone), faixa de mídia (nome, se der
   para ler) e troca de perfil. Exige um watcher lendo o volume real do sistema, senão a
   barra mente quando o volume muda por fora.
4. **Descanso**: depois de um tempo parado (configurável, padrão 2 minutos), roda um texto
   escrito pelo usuário na interface, deslizando da direita para a esquerda. Qualquer
   evento no aparelho tira do descanso na hora.

A prioridade importa: segurar um controle ganha do feedback, que ganha do normal, que ganha
do descanso.

Na tela de configurações entram: o texto do descanso, o tempo até entrar em descanso, e um
liga e desliga. O texto é livre, o usuário escreve o que quiser.

## Configurações gerais
- Brilho global dos LEDs.
- Animação de boot: ligada/desligada, tipo, duração.
- Iniciar com o sistema.
- Botões físicos que fazem "página +" e "página -".
- Importar/exportar perfil (JSON).

## Regras de design
- Sentence case. Sem Title Case.
- Cores dos pads são as cores reais; o resto da UI é neutro.
- Modo claro e escuro.
- Nada de gradiente ou sombra pesada.


## Home Assistant (pedido em 2026-09-02)

O Davi quer uma pagina dedicada as automacoes da casa. O Home Assistant expoe
tudo por HTTP, entao o motor ganhou duas acoes:

- `home_assistant`: chama um servico, no formato `dominio.servico` mais a
  entidade. Ex.: `light.toggle` em `light.sala`. O motor monta
  `POST <endereco>/api/services/light/toggle` com corpo
  `{"entity_id":"light.sala"}` e cabecalho `Authorization: Bearer <token>`.
- `http`: requisicao crua, com metodo, endereco e corpo. Cobre webhook do
  Home Assistant (`/api/webhook/<id>`) e qualquer outro servico da casa.

Endereco e token ficam na config geral (`home_assistant`), nao em cada pad,
para a pessoa digitar uma vez so. A tela de Configuracoes tem a secao.

Ponto aberto: o token fica em texto puro em `~/.mikrodeck/config.json`. A
interface avisa. Guardar no Cofre de Credenciais do Windows fica para depois.


## Servidor MCP: como esta feito (2026-09-02)

Binario separado, `mikrodeck/mcp`, que fala JSON-RPC pelo stdin e stdout.
Ele nao conversa com o motor: so le e grava `~/.mikrodeck/config.json`.
O MikroDeck vigia esse arquivo e aplica sozinho em ate um segundo, entao a
mudanca feita por linguagem natural aparece no aparelho sem reiniciar nada.
A interface tambem recarrega, pelo evento `config-mudou`.

Ferramentas: ler_configuracao, ajuda_acoes, listar_cores, listar_botoes,
criar_pagina, renomear_pagina, apagar_pagina, definir_pad, limpar_pad,
definir_botao, limpar_botao, definir_ajustes.

Provado de ponta a ponta em 2026-09-02: tres chamadas `definir_pad` pelo MCP,
com o app ja rodando, acenderam os pads 1, 4 e 16 no aparelho, confirmado por
foto da webcam, sem tocar na interface.

Para ligar no Claude Code:

    claude mcp add mikrodeck -- <caminho>/mikrodeck-mcp.exe


## Cuidar da janela (2026-09-02)

Opcao por pad, so para a acao de abrir programa, desligada por padrao:
`gerenciar_janela`. Ligada, o pad passa a agir **quando e solto**, porque so
assim da para separar um toque de um "segurar para fechar".

- Toque curto e programa fechado: abre.
- Toque curto e programa aberto: traz para a frente; se ja estava na frente,
  minimiza.
- Segurar por 700 ms ou mais: manda `WM_CLOSE` em todas as janelas visiveis do
  programa. Fechamento educado, o programa ainda pode pedir para salvar.

A janela e achada por `EnumWindows`, filtrando janela invisivel e sem titulo, e
comparando o executavel do processo dono com o caminho configurado.

Motivo de ficar desligada por padrao: ela muda o momento em que o pad responde,
e nem todo programa merece esse comportamento.

## Toque leve na tela (2026-09-02)

O aparelho manda um evento separado quando o dedo encosta no pad sem apertar
(nibble alto `0x40`, `Evento::PadTocado`). Ele so mexe na tela: mostra o nome do
pad. Executar continua sendo coisa do aperto de verdade.


## A touch strip como medidor (2026-09-02)

Alem de controlar, a strip mostra. Ela acende do primeiro LED ate a fracao do
que estiver configurado: volume do sistema, brilho dos pads ou posicao da
pagina. Volume zero deixa ela apagada, o que e a leitura certa.

Custa zero escrita extra: a strip vive no mesmo frame de LED dos pads, e o frame
so vai para o aparelho quando algum byte muda.

O nivel e recalculado a cada tique do laco, porque o volume tambem muda por
fora, pelas teclas de midia do teclado.

Provado por foto em 2026-09-02: volume em 50%, treze dos vinte e cinco LEDs
acesos.


## Arrastar e soltar (2026-09-04)

Arrastar um pad em cima de outro leva a configuracao junto. Vale para botao
tambem; pad so troca com pad e botao so com botao, porque a cor de um nao cabe
no outro.

Destino vazio move na hora. Destino ocupado abre um modal com tres saidas:
substituir, trocar de lugar, cancelar.

Detalhe que custou tempo: o "esta arrastando" precisa morar no `useRef`, nao no
estado do React. Dentro de uma mesma sequencia de eventos o estado ainda nao
atualizou, e o soltar era tratado como clique.

## Desenho do aparelho: medido contra a foto (2026-09-04)

Com a foto frontal em alta resolucao deu para medir de verdade. O que estava
errado e foi corrigido:

- A tela estava 36 de altura por 102 de largura; o certo e 27 por 84. Era ela
  que empurrava o knob.
- O knob estava colado na tela e fora de centro. Agora fica centrado sob a tela,
  com o vao que a foto mostra, e desenhado escuro e canelado como o de verdade.
- O logo virou MIKRODECK, com o anel alinhado a coluna de botoes da esquerda.
- Os 25 LEDs da strip ficam ACIMA da canaleta, nao dentro dela.
- Botoes ganharam a segunda linha impressa (Velocity, Position, Tune, Macro,
  FX Select, Arp, Loop, Replace, Metro, Grid, Count-In, 16 Vel, Section,
  Navigate, Double, Choke), que some quando o botao tem funcao no MikroDeck.
- PLAY verde, REC vermelho e STOP com o quadrado, como impresso.
- A letra de A a P no canto de cada pad.


## Rodada de 2026-09-04: gestos, knob, gravador e o desenho

### Desenho do aparelho: ciclo com avaliador

Quatro rodadas de um agente comparando a captura do app com a foto de cima em
alta (`design/referencia/mikro-mk3-foto-hd.png`). Notas: 7,0 -> 8,7 -> 9,0 -> 9,4.
Veredito da ultima: pronto para entregar. Relatorio em `design/revisao-interface.md`.

O que mais pesou: o desenho estava arejado demais. Coluna central com metade
da altura, GROUP e PLAY baixos, margens grandes. Tudo medido em porcentagem do
corpo do aparelho e corrigido em `app/src/Aparelho.tsx`.

Licao da rodada 3: tres ajustes finos passaram do ponto. Diferenca menor que
1 ponto percentual nao se mexe mais.

Captura para avaliacao: `ffmpeg -f gdigrab -draw_mouse 0 -i title=MikroDeck`
e recorte com `scripts/recorta-simples.js`.

### Gestos de janela (opcao "cuidar da janela", por pad)

Vale para abrir programa e para abrir link. Toque duplo e um segundo toque em
menos de 350 ms; segurar e a partir de 700 ms.

| Gesto | Programa | Link |
|---|---|---|
| Toque, nada aberto | abre | abre |
| Toque, ja aberto | frente; se ja na frente, minimiza | vai para a janela, nunca minimiza |
| Toque duplo | maximiza / restaura | maximiza / restaura |
| Segurar | fecha | abre outra |

Janela de programa: nome do executavel, e se nao achar, qualquer janela de
processo na mesma pasta de instalacao (git-bash.exe abre mintty.exe).
Janela de link: titulo contendo o nome do site (`open.spotify.com` -> `spotify`).

A ordem importa: tentar a janela primeiro e so abrir se nao houver nenhuma.
Perguntar "esta aberto?" antes falhava com lancadores.

### Knob

Clique e um botao chamado `knob` na config, programavel por pagina, sem LED.
Girar: nada, volume (2% por passo), brilho, pagina, ou rolagem (roda do mouse,
dois cliques por passo).

### Gravador de atalho

Como o reWASD: grava o conjunto de teclas seguradas ao mesmo tempo, fecha
quando solta tudo, modificador sozinho conta, guarda a maior combinacao vista.
Atalhos que o Windows reserva (win+L) nao chegam ao app.

### Botoes de pagina reservados

Os dois escolhidos em Configuracoes nao aceitam programacao: aparencia propria,
nao arrastam, painel explica. No motor eles ganham prioridade sobre a config da
pagina. Trocar a escolha libera o antigo na hora.

### Outros

- Link sem esquema ganha https:// no motor (vale para UI, MCP e arquivo).
- Config gravada por temporario + rename, para nao ficar pela metade.
- Cadeado do estado a prova de veneno no servico.
- Desligado, o motor apaga a tela uma vez e nao toca mais no aparelho.
- Calibracao da touch strip em Configuracoes (minimo e maximo alcancaveis).
- Painel do MCP generico: caminho, comando de terminal e bloco JSON.
- Arrastar e soltar entre pads e entre botoes, com modal de substituir/trocar.
