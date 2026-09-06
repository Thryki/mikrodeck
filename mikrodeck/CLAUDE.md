# MikroDeck

App desktop que transforma o Maschine Mikro MK3 (Native Instruments) em um Stream Deck: pads com atalhos, cores por estado, páginas, knob, touch strip e texto na tela.

Dono do projeto: Thryki (art director). Este arquivo é a memória do projeto. Leia inteiro antes de qualquer tarefa.

## Como trabalhar com o Davi

- Responder em português, informal mas respeitoso.
- Um passo por vez. Fazer, mostrar, pausar, esperar o ok. Nunca despejar tudo de uma vez.
- Frases curtas, voz ativa, foco em ação e resultado. Sem travessões, sem floreio, sem adjetivos desnecessários.
- Não usar as palavras "literalmente", "talvez", "provavelmente".
- Ele pensa visualmente: checklists, mapas de processo, hierarquia visual. Reduzir carga cognitiva.
- Design: consistência visual estrita. Ele é o diretor de arte; a decisão final de design é dele.

## Hardware (confirmado com foto)

Maschine Mikro MK3. Uma tela pequena monocromática, um knob (encoder com push), 16 pads RGB, touch strip com 25 LEDs azuis, botões com LED próprio.

Layout físico (numeração real dos pads):

```
Coluna esquerda            Coluna central   Direita
[engrenagem] [tela] [VOLUME][PLUG-IN]   FIXED VEL   PAD MODE KEYBOARD CHORDS STEP
[estrela]  (knob)  [SWING] [SAMPLING]   SCENE       13  14  15  16
[lupa]             [TEMPO]  [<] [>]     PATTERN      9  10  11  12
(o) MASCHINE                            EVENTS       5   6   7   8
[PITCH][MOD][PERFORM][NOTES]            VARIATION    1   2   3   4
[==== touch strip 25 LEDs ====]         DUPLICATE
[GROUP][AUTO][LOCK][NOTE REPEAT]        SELECT
[RESTART][ERASE][TAP][FOLLOW]           SOLO
[PLAY][REC][STOP][SHIFT]                MUTE
```

Pad 13 é o canto superior esquerdo. Pad 1 é o canto inferior esquerdo. Letras A a P acompanham 13..16, 9..12, 5..8, 1..4.

Foto de referência: `design/referencia/mikro-mk3-foto.png` (640 px, baixa resolução; pedir versão maior ao Davi).

## Achado técnico que define tudo

O Mikro MK3 NÃO envia MIDI pela USB. É um dispositivo HID puro. O "MIDI mode" é emulado pelo driver da NI. Falar HID direto dá acesso total: LEDs de pads, strip e botões, escrita na tela, leitura de pads (com pressão), botões e knob, sem depender de driver.

Referências de protocolo:
- pymikro (Python, Mikro MK3 exato): https://github.com/flokapi/pymikro
- Notas de engenharia reversa do MK3 (família): https://gist.github.com/ktemkin/89253ecf10c5078f47607776564de83b

Reescrever em Rust usando pymikro como mapa dos pacotes.

## Decisões fechadas

- Stack: Tauri 2 + Rust no núcleo, UI em React + Tailwind.
- Escopo v1: pads com páginas, LEDs por estado, knob, touch strip, botões físicos, tela.
- Motor separado da UI. O motor roda em background (tray), sobe com o sistema, e a UI só conecta quando aberta.
- Páginas: dois botões físicos livres viram "página +" e "página -". Cada página tem 16 ações e 16 cores. A tela mostra "Nome da página N/total".

## Arquitetura do motor (Rust)

Ver `docs/arquitetura.md`. Resumo dos módulos:

1. `hid`: abre o device, thread de leitura bloqueante, thread de escrita com tick fixo (30 a 60 Hz) que manda o frame inteiro de LEDs. Nada fora deste módulo toca no HID.
2. `state`: máquina de estados por controle (idle, pressed, active, disabled), cada um com cor.
3. `actions`: abrir app, hotkey, script, comando shell, URL, controle de mídia, trocar página,
   pausar o MikroDeck, chamar serviço do Home Assistant e requisição HTTP crua.
4. `watchers`: processo aberto, janela em foco, volume, etc. Alimentam o `state`.
5. `render`: framebuffer da tela, envia só quando muda.
6. `ipc`: expõe estado e config para a UI Tauri.

Caminho crítico de latência: pad -> hid -> state -> actions, tudo em Rust, sem passar pela UI.

## LEDs: RESOLVIDO (2026-09-02)

Rodar o **Maschine 2 como administrador uma vez** grava um estado permanente no aparelho. Depois
disso ele aceita comandos de LED de qualquer processo, a qualquer momento, **sem sequência de
inicialização nenhuma**, e o estado sobrevive a desconectar o cabo.

Provado com o cabo religado fisicamente e nada da NI aberto: escrita crua do report `0x80`,
sem leituras, 9 ms depois de abrir o aparelho, acende tudo. A tela fica apagada, o que prova
que nenhum software da NI participou.

Cinco teorias minhas caíram no caminho (tamanho do pacote, modo MIDI, sequência de
inicialização, janela de tempo, corrida de abertura). Todas estão registradas em
`docs/spike-hid.md` para não serem reaproveitadas.

Regra para o manual: **abrir o Maschine 2 como administrador uma vez, na primeira vez.**
Depois disso o MikroDeck roda sozinho.

Persistência confirmada: sobrevive a reiniciar o Windows e a religar o cabo quantas vezes for.
O modo MIDI também fica guardado no aparelho, e volta como estava depois de religar.

## Spike HID: feito. Ver `docs/spike-hid.md`

Rodado em 2026-09-01 e 09-02. O que ficou provado no aparelho real:

- VID `0x17cc`, PID `0x1700`. Interface `MI_00`, HID vendor-defined, endpoints `0x81` entrada e `0x01` saída.
- **Leitura funciona**: reports `0x01` (botões, knob, strip) e `0x02` (pads com pressão de 0 a 4095).
- **Tela funciona**: report `0xe0`, 128x32, 1 bit por pixel, dois pacotes de 265 bytes, bitmap invertido (bit 1 apaga o pixel). Dá para desenhar em regiões, mas a transferência precisa vir no tamanho cheio.
- **LEDs funcionam**, report `0x80`, 81 bytes: byte 0 é o ID, 1 a 39 os botões, 40 a 55 os 16 pads, 56 a 80 os 25 LEDs da strip. Byte = `(cor << 2) | brilho`. Brilho 0 é o nível fraco, não apagado; o que apaga é o byte inteiro em zero. Botões acesos usam `0x7c`, `0x7e` ou `0x7f`.
- Tudo isso pela **API HID padrão do Windows**. Não precisa WinUSB, não precisa Zadig, não precisa trocar driver.

Duas teorias minhas que se provaram **erradas** e não devem ser reaproveitadas:
1. Que o Windows inflava o pacote para 265 bytes e o firmware descartava.
2. Que o modo MIDI era o que habilitava os LEDs.

Ferramentas de verificação criadas: webcam Insta360 apontada para o aparelho com foto automática (`scripts/olho.sh`) e medição numérica de brilho por região com desconto de exposição (`scripts/mede.js`). Sem a medição numérica, variação de exposição da câmera passa por LED aceso.

## UI (ver docs/ui-spec.md)

Referência de sensação: interface da Logitech (Options / G Hub). Fácil para leigo, completa para quem quer tudo.

Telas:
1. Principal: barra de status + perfis, abas de páginas, desenho fiel do Mikro, painel lateral do controle selecionado.
2. Painel do controle: nome, ação, parâmetros, cores por estado (repouso, pressionado, ativo), texto na tela ao apertar, botão "Testar no aparelho".
3. Biblioteca de ações.
4. Perfis (conjuntos de páginas por contexto).
5. Configurações: brilho, animação de boot, iniciar com o sistema.

Mockups já feitos:
- `design/mockup-config.html`: tela principal v1 (aprovada como direção).
- `design/mikro-mk3-v2.svg`: render do aparelho v2 (passou por 1 rodada de revisão; falta aprovação final do Davi).

## Loop de qualidade do render do aparelho

O Davi quer que o desenho do aparelho seja fiel à foto. Fluxo:
1. Subagente `construtor` edita `design/mikro-mk3.svg`.
2. `scripts/render-svg.sh` converte para PNG.
3. Subagente `revisor` compara PNG com a foto e escreve `design/revisao.md`.
4. Sessão principal repete até o revisor devolver "sem diferenças relevantes".
5. Davi aprova.

Pendências conhecidas do render: sub-rótulos ilegíveis na foto (PATTERN, EVENTS, SELECT, SOLO). Não inventar; pedir ao Davi.

## Motor: hid + state + actions prontos (2026-09-02)

Código em `mikrodeck/motor`, 30 testes passando. Roda e funciona no aparelho.

| Módulo | O que faz |
|---|---|
| `hid/protocolo.rs` | constantes, tabela de 18 cores, mapa dos 16 pads e dos 39 botões |
| `hid/frame.rs` | frame de 81 bytes; só marca sujo quando um valor muda de verdade |
| `hid/eventos.rs` | eventos já decodificados, com o número do pad **impresso no aparelho** |
| `hid/mod.rs` | abre o device, thread de leitura e thread de escrita a 30 Hz |
| `config.rs` | `~/.mikrodeck/config.json`, cria exemplo no primeiro uso |
| `acoes.rs` | executa em thread própria; atalhos e mídia via SendInput |
| `estado.rs` | páginas, pads apertados, cor por estado |

Decisões que valem manter:
- Por fora, pad é o número impresso no aparelho (1 a 16). A ordem bruta fica escondida no `hid`.
- O `estado` não fala com HID nem executa ação: recebe evento e devolve `Reacao`. Testável sem aparelho.
- Ação nunca roda no caminho crítico. Vai para uma fila em outra thread.
- Trocar de página dá a volta nos dois sentidos.

Ações prontas: abrir programa, abrir URL, comando, atalho de teclado, teclas de mídia, trocar página.
Falta: `watchers` (estado "ativo"), tela (report `0xe0` já mapeado), perfis, autostart e tray.

## App Tauri: de pé e funcionando (2026-09-02, madrugada)

`mikrodeck/app` é o app Tauri 2 com React e Tailwind. O motor virou biblioteca
(`mikrodeck/motor`, lib + bin) e o app depende dele por caminho.

Ciclo completo provado no aparelho: editar um pad na interface salva sozinho, aplica no
motor sem reiniciar, e acende no hardware.

| Peça | Onde |
|---|---|
| Ponte motor/UI | `app/src-tauri/src/lib.rs`: comandos e eventos |
| Serviço residente | `motor/src/servico.rs`: reconecta sozinho quando religa o cabo |
| Tela do aparelho | `motor/src/render/`: fonte 5x7, framebuffer, compositor de cenas |
| Ponte da UI | `app/src/ponte.ts`: fala com o Tauri, ou usa dados falsos no navegador |

O `ponte.ts` permite abrir `http://localhost:1420` num navegador comum e trabalhar no visual
sem aparelho. Foi assim que a interface foi revisada e corrigida.

### Regra de ouro do HID: uma escrita por tique

O aparelho aceita cerca de 31 escritas por segundo. Mandar o frame de LED e os dois pacotes
de tela no mesmo tique passa de 90 por segundo, e ele começa a descartar pacote e as cores
saem erradas. A thread de escrita manda **uma coisa por tique**, com os LEDs na frente, e
espaça as duas metades da tela em 12 ms.

### Dois bugs de "abrir programa", os dois reais

1. Nome curto como `chrome` falha: o Windows resolve pela chave de registro "App Paths",
   que o `CreateProcess` não consulta.
2. Atalho `.lnk` falha com "não é um aplicativo Win32 válido" (erro 193).

Os dois caem no mesmo conserto: abrir pelo `start` do shell. Extensões que sempre vão pelo
shell estão em `SO_PELO_SHELL`. Tem teste de regressão.

### Estado da lista de pedidos do Davi

Feito: janela responsiva, tela do aparelho com página e nome do controle, brilho geral
rotulado, brilho por pad, ícones desenhados, contraste do seletor no modo escuro, menu de
contexto próprio nas páginas, painel de configurações com diagnóstico e teste de LEDs,
botões físicos programáveis.

Falta, na ordem combinada: funções padrão nos três botões da esquerda, touch strip
configurável, sair do modo MIDI ao abrir, cor de "aplicativo aberto", pressão leve para
prever, aperta/aperta/segura, sons, cores animadas, efeitos de LED. Lista completa em
`docs/ui-spec.md`.

## Estado em 2026-09-02 (madrugada): o app funciona

O que está pronto e provado no aparelho:

- Motor em Rust com 67 testes. Leitura de pads com pressão, botões, knob e strip.
- LEDs, tela e strip escrevendo pelo HID padrão do Windows.
- App Tauri com bandeja do sistema, iniciar com o Windows, e fechar a janela
  esconde em vez de encerrar. Sair de verdade é pelo menu da bandeja.
- Interface espelhando o aparelho ao vivo, nos dois sentidos.
- Botões de fábrica: círculo liga e desliga o MikroDeck, estrela vai para a
  primeira página, lupa abre a busca do Windows. Os três acendem fraco sempre,
  para serem achados sem manual, e todos podem ser trocados.
- Touch strip configurável: volume do Windows, brilho dos pads ou escolher página.
- Tela: nome da página em corpo grande, número embaixo, nome do controle
  segurado, barra de volume, e descanso com texto correndo.
- Ações: abrir programa, URL, comando, atalho, mídia, trocar página, pausar,
  Home Assistant e requisição HTTP crua.
- Servidor MCP (`mikrodeck/mcp`) para configurar por linguagem natural. Provado
  de ponta a ponta: chamada MCP acendeu pad no aparelho sem reiniciar nada.
- A config é vigiada em disco: mudou por fora, o motor aplica em até 1 segundo.
- Vigia de processos: o pad muda de cor sozinho quando o programa está aberto.
- Páginas prontas: Spotify, Claude, Casa e Trabalho, adicionadas sem sobrescrever nada.
- Cuidar da janela (opcional por pad): apertar alterna a frente, segurar fecha.
- Toque leve no pad mostra o nome na tela sem executar.
- A touch strip acende como medidor do que ela controla (volume, brilho, página).
- README.md com o manual do usuário.

Documentação do protocolo em inglês, para o repositório público:
`docs/maschine-mikro-mk3-hid-protocol.md`.

### Regras que valem para sempre

- O aparelho aceita cerca de **31 escritas por segundo**. Passar disso corrompe
  pacote. Uma escrita por tick, LED na frente, 12 ms entre as duas metades da tela.
- Buffer de leitura de 256 bytes. Com 64 o pacote grande some sem erro.
- Botão do aparelho é monocromático: só o brilho conta.

### Falta fazer, em ordem

1. Garantir que o aparelho sai do modo MIDI quando o MikroDeck abre.
2. Sons ao apertar.
3. Cores alternando e animações de LED.
4. Repositório no GitHub com a engenharia reversa
5. Samples de audio nos pads: carregar, gravar do microfone (30 a 40 s),
   one-shot ou enquanto apertado, envelope; plugins so bem depois.
   Detalhes em `docs/ui-spec.md`, secao "Futuro: samples de audio". Pedido
   explicitamente como futuro, NAO e para agora.
   (`docs/maschine-mikro-mk3-hid-protocol.md` já está escrito, em inglês).

## Estado em 2026-09-04

Rodada grande de pedidos do Davi, tudo em `docs/ui-spec.md` na secao
"Rodada de 2026-09-04". Resumo do que entrou:

- Desenho do aparelho passou por 4 rodadas de avaliador contra a foto em alta:
  7,0 -> 9,4, veredito "pronto". Nao mexer em diferenca menor que 1 ponto
  percentual: na rodada 3 tres ajustes passaram do ponto.
- Gestos de janela por pad, para programa e para link: toque, toque duplo
  (maximiza), segurar (fecha o programa / abre outra janela do link).
- Knob: clique programavel como botao `knob`; girar faz volume, brilho,
  pagina ou rolagem.
- Gravador de atalho estilo reWASD (conjunto de teclas seguradas).
- Botoes de pagina reservados, arrastar e soltar, calibracao da strip,
  https automatico, config atomica, cadeado a prova de veneno.
- Dados pessoais limpos: autor e Thryki, caminhos sem usuario, fotos fora do
  git. Repositorio combinado: `mikrodeck`, privado por enquanto.

Regra de build que custou tempo: **`cargo build --release` direto no
src-tauri gera binario apontando para o servidor de desenvolvimento.** O
certo e sempre `npm run tauri build`. E antes de buildar, derrubar
`mikrodeck-mcp.exe`, senao o arquivo em `target/release` fica travado.

## Rodada de 2026-09-05: luz dos pads e README

- `docs/animacoes.md`: desenho fechado por painel (3 propostas, 2 juizes).
  Cinco modos mais "nenhuma": Respiracao (padrao), Contorno (a lista do Davi),
  Colunas, Pulso e Som. Som ainda cai na Respiracao; falta o medidor.
- Modulo `motor/src/luz/`: `Animador` puro, recebe `Instant` e devolve `Quadro`.
  Orcamento de escritas provado por teste com relogio falso, teto de 8 quadros
  por segundo em todo modo x ritmo x brilho.
- `hid/frame.rs` guarda `enviado`: sujo virou "bytes diferentes do ultimo
  enviado". Sem isso, pintar o repouso e por cima a animacao gastaria uma
  escrita por tique com bytes iguais.
- Cinco bugs achados na revisao e consertados, com teste cada:
  1. A strip escurecia no eco, nao so no descanso. Piscava a cada pad solto.
  2. Pulso andava no passo minimo (125 ms) em vez do passo do ritmo (250 ms).
  3. Modo `nenhuma` mantinha o laco a 25 ms sem ter o que animar.
  4. Toque leve disparava o eco: `PadSolto` chega tambem sem aperto.
  5. `pintar_com` nunca apagava botao que deixou de ser pintado. "Testar LEDs"
     deixava os 39 botoes acesos para sempre, e botao de outra pagina ficava
     aceso ao trocar de pagina. Agora apaga botoes e strip antes de pintar.
- README novo, escrito por painel (3 redatores, 2 juizes, 36,25/40): indice,
  prints da interface em `docs/imagens/`, o que o app faz de verdade, secao
  de conviver com o Maschine 2, e como ajudar. Prints tirados com Playwright
  contra o servidor de desenvolvimento (`scripts/prints-readme.js`).
- Repositorio no ar: https://github.com/Thryki/mikrodeck, privado.
  `gh` 2.100 instalado, sem login ainda; o push usa a credencial do Git.

### Verificado no aparelho (2026-09-05, tarde)

Confirmado por foto: modo Contorno anda na ordem que o Davi pediu, com rastro de
quatro pads e troca de cor a cada volta. `scripts/olho.sh` girava a imagem duas
vezes (`hflip,vflip,rotate=PI` volta ao original); corrigido para `hflip,vflip`.

Queixa do Davi sobre a respiracao: os pads subindo e descendo de brilho anulam o
slider de brilho e nao agradam. A opcao de ligar, desligar e trocar de modo ja
existia em Configuracoes > Luz dos pads. O que estragava era o **descanso em 5
segundos**, deixado de teste. Voltou para 90 s, e o modo padrao dele agora e
Contorno, que nao mexe no brilho.

### Samples de audio nos pads (2026-09-05)

Saiu do "futuro" e entrou. Modulo `motor/src/som/`, separado de `audio.rs`
(aquele e o volume do Windows, este toca som).

| Peca | O que faz |
|---|---|
| `som/mod.rs` | `Saida` (placa de som + cache de arquivos decodificados), `Voz`, `Tocador` |
| `som/envelope.rs` | ADSR como fonte que embrulha outra e multiplica o ganho |
| `som/gravador.rs` | grava do microfone em WAV, teto de 60 s |

Bibliotecas: `rodio` 0.21 (symphonia por baixo: wav, mp3, flac, ogg, m4a, aac),
`cpal` 0.16 para a entrada, `hound` para escrever o WAV.

Decisoes que valem manter:
- O arquivo e decodificado **uma vez** e fica em memoria. Apertar o pad so
  empurra um buffer pronto; nada de disco no caminho critico.
- O sample **nao passa pela thread de acoes**. Quem toca e o `servico`, porque
  soltar o pad precisa alcancar a mesma voz que o aperto comecou, e a thread de
  acoes nao sabe de qual pad veio o evento.
- Uma voz por pad: apertar de novo recomeca, nao empilha.
- Sem placa de som o MikroDeck continua funcionando; so o sample nao toca.
- Pausar corta todas as vozes: pausado, o aparelho volta a ser um Maschine.

Testes: 156 no total. Cinco deles (`tests/som_de_verdade.rs`) usam a **placa de
som real** e pulam sozinhos numa maquina sem saida de audio.

Falta: plugins nos samples (VST3/CLAP), que o proprio Davi deixou para bem
depois.

### Envelope como o de um plugin, e o silencio que morre sozinho

Pedido do Davi com um print do envelope do FL Studio.

O `Envelope` ganhou os estagios que faltavam: **atraso**, **retencao** e duas
**tensoes** (curva da subida e curva da descida). A tensao e expoente
(`x^(2^-t*2)`), escolhido porque assim a curva nunca sai da faixa de 0 a 1 e,
principalmente, **nao muda a duracao do estagio**: mexer na curva nao pode
mudar quanto tempo o som dura.

`EnvelopeGrafico.tsx` desenha a curva com as alcas arrastaveis. A conta de
`curvar` esta repetida em TypeScript de proposito: o desenho tem que ser a
mesma curva que o motor toca.

Dois detalhes que so apareceram testando com Playwright:
1. Com os estagios em zero as cinco alcas caem no mesmo ponto. Cada estagio
   agora ocupa uma **largura minima** de 14 unidades no desenho.
2. Mesmo assim o alvo de clique de uma alca cobria o da vizinha. O raio do
   alvo virou **metade da folga ate o vizinho**, entre 4 e 14.
O arrasto escuta a **janela**, nao o SVG: preso ao SVG, sair do desenho com o
botao apertado largava o no no meio do caminho.

Gravacao: `aparar_silencio` corta o silencio das duas pontas quando a gravacao
fecha, com margem de 30 ms e limiar de 0,005 (uns -46 dB). Arquivo so de
silencio fica como esta: apagar o que a pessoa acabou de gravar seria pior.

### Fluidez da luz (2026-09-05, tarde)

O Davi achou a animacao travada. Eram duas causas somadas:

1. `PASSO_MINIMO` de 125 ms: teto de 8 quadros por segundo. Agora 50 ms, 20 por
   segundo. O aparelho aceita cerca de 31 escritas por segundo e a tela no
   descanso usa umas 4, entao cabe.
2. O movimento andava de pad em pad, em degrau. Agora a posicao da cabeca e
   **fracionaria**: com ela em 4,5 os pads 4 e 5 dividem o brilho. E isso que
   da movimento continuo com so quatro niveis de brilho.

Contorno e Colunas ganharam rastro por rampa (`intensidade`, com cauda longa
atras e curta na frente, o que da a direcao). O Pulso virou onda circular de
verdade: cada pad tem distancia ate o centro, e o raio cresce em fracao.

Para a tela nao ficar sem vez com o LED escrevendo 20 vezes por segundo, a
thread de escrita conta os tiques em que a tela ficou suja e cede a vez depois
de 6 (200 ms, o passo do texto correndo).

Regra que mudou: **o ritmo nao manda mais na taxa de quadros**, so na velocidade
da cabeca. Quem manda na taxa e o passo minimo.

### Bug das teclas de pontuacao

`codigo_da_tecla` nao conhecia `-`, `=`, `.`, `,` e companhia, e `mandar_atalho`
usava `filter_map`: a tecla desconhecida sumia e o atalho virava outro
("ctrl+minus" mandava um "ctrl" solto). Agora a pontuacao tem codigo OEM e uma
parte desconhecida **cancela o atalho inteiro**, com aviso no log. Teste de
regressao cobre toda tecla de toda pagina pronta.

### Paginas prontas: Navegador e Windows

Somam seis com as quatro antigas. Navegador: abas, historico, downloads, zoom,
tela cheia. Windows: encaixar janelas, trocar de app, gravar tela, area de
transferencia, emoji, projetar.

### Nao verificado no aparelho

A luz dos pads passa nos 130 testes mas **nao foi confirmada por foto**: a
webcam saiu de posicao. Quando voltar, apontar para o aparelho e conferir
Respiracao, Contorno, Colunas, Pulso e o eco com `scripts/mede.js`.

## Próximos passos, em ordem

1. Funções padrão nos três botões da esquerda: ligar/desligar, favoritos, busca do sistema.
2. Touch strip configurável: volume, brilho dos pads, brilho da tela.
3. Garantir que o aparelho sai do modo MIDI quando o MikroDeck abre.
4. `watchers` para o estado "aplicativo aberto", com cor própria.
5. Comportamento de pressão: leve mostra na tela, firme executa.
6. Tray, autostart e instalador.
7. Descanso de tela e sons.
8. Repositório no GitHub com toda a engenharia reversa.
