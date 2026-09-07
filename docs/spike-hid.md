# RESOLVIDO: o Maschine 2 como administrador destrava o aparelho DE FORMA PERMANENTE (2026-09-02)

Rodar o **Maschine 2 como administrador uma vez** grava um estado persistente no aparelho.
A partir daí ele aceita comandos de LED de qualquer processo, a qualquer momento, **sem
nenhuma sequência de inicialização**, e o estado sobrevive a desconectar o cabo.

Não é por sessão do Windows, não é por conexão. É uma vez só, e pronto.

## As provas, em ordem

Todos os testes abaixo foram feitos com o cabo desconectado e reconectado fisicamente antes de
cada um, sem nada da NI aberto (só o serviço, que sozinho não faz nada), e a tela do aparelho
apagada (prova de que nenhum software da NI participou).

| Teste | O que fez | Resultado |
|---|---|---|
| Abertura imediata, com as 4 leituras | abre em 5 ms, lê `0xf8`/`0xd0`/input `0x01`/`0xd0`, escreve | **acende** (azul) |
| Escrita 2 s depois de abrir | mesma coisa, mas espera 2 s antes de escrever | **acende** (verde) |
| Abertura 10 s depois de religar | espera 10 s antes de sequer abrir o aparelho | **acende** (magenta) |
| **Sem inicialização nenhuma** | abre e escreve o frame direto, em 9 ms, zero leituras | **acende** (amarelo) |

O último teste é o que fecha a questão: nenhuma sequência é necessária. O aparelho simplesmente
está acordado.

## Teorias minhas que caíram no caminho

Registrado para ninguém (inclusive eu) reaproveitar:

1. **Tamanho do pacote**: achei que o Windows inflava o frame de 81 para 265 bytes e o firmware
   descartava. Errado. Funciona com o enchimento.
2. **Modo MIDI**: achei que SHIFT + botão da NI era a chave. Errado, coincidência.
3. **Sequência de inicialização**: achei que as 4 leituras que o Maschine 2 faz eram o gatilho.
   Errado, o modo nu funciona.
4. **Janela de tempo**: achei que existia uma janela curta depois de energizar. Errado, funciona
   com 10 segundos de atraso.
5. **Corrida de abertura**: achei que importava ser o primeiro a abrir o aparelho. Errado.

O que resistiu a todos os testes: o estado persistente gravado pelo Maschine 2 elevado.

## Persistência: confirmada (2026-09-02)

Testado pelo Thryki depois da descoberta:

- Reiniciou o Windows: continua funcionando.
- Desconectou e reconectou o cabo várias vezes seguidas: funciona sempre.
- O **modo MIDI também é guardado no aparelho**: liga o modo MIDI, tira o cabo, recoloca, e ele
  volta em modo MIDI. Desliga o modo MIDI, tira o cabo, recoloca, e volta desligado.

Ou seja, é memória não volátil do próprio aparelho, não estado do driver nem do Windows.

## O que ainda não sabemos

- **O que exatamente foi gravado.** O Maschine 2 elevado consegue; o não elevado não.
- **Se um aparelho de fábrica precisa disso.** Provável que sim, e vira uma linha no manual.

## Regra para o manual do MikroDeck

> Na primeira vez, abra o Maschine 2 como administrador uma vez com o Mikro conectado.
> Depois disso o MikroDeck funciona sozinho, sem nada da Native Instruments rodando.

## Receita técnica para o motor

Nada de especial. Abrir o aparelho por HID (`hidapi`, VID `0x17cc`, PID `0x1700`) e escrever o
report `0x80` com 81 bytes. O Windows enche até 265 bytes sozinho e funciona. Ler os reports
`0x01` e `0x02` para entrada. Escrever o report `0xe0` para a tela.

---

# DECISÃO (2026-09-02, 02h): o Maschine 2 é pré-requisito. Seguimos para o motor.

Depois de capturar a inicialização real com USBPcap e reproduzi-la byte a byte, ficou provado
que **os bytes não bastam** para acordar o aparelho. A decisão do Thryki foi aceitar o Maschine 2
como pré-requisito e parar a caça ao comando.

## Regra de uso do MikroDeck (vai para o manual)

1. Conecte o Mikro MK3.
2. Abra o **Maschine 2 como administrador** uma vez e espere ele conectar (os LEDs acendem).
3. Feche o Maschine 2. Pode até parar o serviço da NI.
4. A partir daí o MikroDeck controla tudo sozinho: pads, LEDs, strip, tela, entrada.
5. Se desconectar o cabo, repita a partir do passo 2. Reiniciar o Windows **não** exige repetir
   (o reboot não corta a energia da porta USB).

Detalhes que importam:
- Abrir o Maschine 2 **sem** administrador não inicializa o aparelho (testado, o Control Panel
  dá "No Maschine MK3 found" e nada acende).
- O Controller Editor sozinho não serve; precisa do Maschine 2 instalado e aberto antes.
- O Maschine MK3 Control Panel (`nimc3cpl.exe`) é da interface de áudio do MK3 grande. O Mikro
  não tem interface de áudio; o erro que ele mostra é esperado e irrelevante.

## O que a captura da inicialização real mostrou

Capturada com o cabo religado fisicamente e o Maschine 2 aberto como administrador. Do
religamento até a primeira escrita de LED, o aparelho só recebe:

| t | Operação | Quem manda |
|---|---|---|
| +0,00s | GET_DESCRIPTOR (device, config, strings, HID report) | Windows |
| +0,06s | SET_CONFIGURATION, SET_IDLE | Windows |
| +2,4s | (silêncio de 14 s enquanto o Maschine 2 carrega) | |
| +16,4s | GET_DESCRIPTOR das strings: fabricante, produto, serial (duas vezes) | Maschine 2 |
| +16,42s | GET_REPORT feature `0xf8` (11 bytes) | Maschine 2 |
| +16,42s | GET_REPORT feature `0xd0` (33 bytes) | Maschine 2 |
| +16,43s | GET_REPORT **input `0x01` pelo canal de controle** (14 bytes) | Maschine 2 |
| +16,43s | GET_REPORT feature `0xd0` de novo (33 bytes) | Maschine 2 |
| +16,43s | **escrita do frame de LED, 81 bytes, endpoint `0x01`** | Maschine 2 |
| +16,44s | escrita da tela, 2 × 265 bytes | Maschine 2 |

Nenhum SET_REPORT de classe além do SET_IDLE padrão. Nenhum comando vendor. Nenhum tráfego
na interface DFU. Só leituras e depois as escritas normais.

## O que foi reproduzido e falhou

`envia.exe --init` faz exatamente essa sequência: lê as três strings, os quatro reports na mesma
ordem, e escreve o mesmo frame. Testado depois de religamento físico real, detectado
automaticamente pelo script (`scripts/teste-init-admin.ps1`):

| Variante | Resultado |
|---|---|
| Sequência exata, processo comum | não acende |
| Sequência exata, **processo elevado (administrador)** | não acende |
| Sequência exata, elevado, **com polling do endpoint de entrada antes e durante** | não acende |
| Mesmo frame, mantendo o handle aberto e reenviando a 20 Hz por 15 s | não acende |

As leituras devolvem os mesmos bytes que o Maschine 2 recebeu. A escrita é aceita pelo SO.
O aparelho continua surdo.

Conclusão: o gatilho está em algo que o USBPcap não enxerga. Candidatos: flags do CreateFile ou
IOCTLs específicas do `hid.dll`, um filtro de kernel instalado junto com o Maschine, ou estado
que o `NIHardwareService` mantém e que o Maschine 2 elevado consegue tocar. Descobrir isso
exige interceptar as chamadas do próprio Maschine 2 (API Monitor / Process Monitor), não o
barramento USB. Fica registrado como investigação futura, opcional.

## Ferramentas que ficam

| Script | Uso |
|---|---|
| `scripts/captura-init.ps1` | grava o USB durante um religamento físico (janela de 90 s, `-A` obrigatório) |
| `scripts/captura-usb.ps1` | grava o nosso próprio tráfego, para validar a captura |
| `scripts/teste-init-admin.ps1` | detecta o religamento sozinho e roda `envia --init` elevado |
| `scripts/teste-init.ps1` | variante com desligar/religar pelo Windows (não corta energia; só serve para outros testes) |
| `scripts/reverter-driver.ps1` | volta a interface para o driver HID da Microsoft, se algum dia o WinUSB for reinstalado |
| `scripts/reiniciar-ni.ps1` | reinicia o serviço da NI quando ele perde o aparelho |
| `scripts/olho.sh` + `scripts/mede.js` | foto pela webcam e medição de brilho por região com desconto de exposição |

O USBPcap congela em capturas longas com a webcam ligada (o hub dela gera mais de 1 GB por minuto).
Para capturar de novo, desligue a webcam antes.

---

# O que habilita os LEDs: a inicialização do Maschine 2 (2026-09-02)

Depois de uma sequência de testes com correções pelo caminho, o quadro final é este:

**O aparelho ignora o comando de LED até que o Maschine 2 rode uma vez e o inicialize.**
Depois disso, o nosso código controla os LEDs normalmente, pela API HID padrão do Windows,
e o software da NI pode ser fechado e o serviço parado.

## Como chegamos aqui, incluindo os erros

1. Teoria do tamanho do pacote: **errada**. Achei que o Windows inflava o pacote para 265 bytes
   e o firmware descartava. Troquei o driver por WinUSB para mandar 81 bytes exatos. Não era isso:
   hoje o LED funciona pela API HID normal, com o enchimento e tudo.
2. Teoria do modo MIDI: **errada também**. O modo MIDI (SHIFT + botão da NI, tela mostra
   "MIDI MODE") coincidiu com o primeiro teste que funcionou, mas depois confirmamos o controle
   de LED com a tela mostrando "MASCHINE MIKRO", ou seja, fora do modo MIDI.
3. O que resiste aos testes: o aparelho precisa ser **inicializado pelo Maschine 2** uma vez a
   cada vez que é conectado. O Maschine Control Panel sozinho não serve, dá erro dizendo que não
   encontrou o MK3.

## Estado do aparelho

| Situação | LEDs obedecem? |
|---|---|
| Recém conectado, nada da NI aberto | não |
| Depois de abrir o Maschine 2 | **sim** |
| Maschine 2 fechado, depois de ter aberto | sim |
| Serviço `NIHardwareService` parado, depois de ter aberto | sim |
| Depois de desconectar e reconectar o cabo | volta a não obedecer |

## Driver: o software da NI não instala nada na interface que usamos

| Interface | Serviço | Fornecedor |
|---|---|---|
| `MI_00`, a que usamos | `HidUsb`, `input.inf` | Microsoft |
| `MI_01`, atualização de firmware | `nimm3dfu`, `oem9.inf` | Native Instruments |

Ou seja, o Maschine 2 não trocou driver nenhum do lado que interessa. O que ele faz é mandar
alguma sequência de inicialização pelo próprio HID. **É essa sequência que falta descobrir**,
e ela é a única coisa entre o projeto e um motor totalmente independente.

## O que já funciona hoje, depois da inicialização

Leitura de pads com pressão, botões, knob e touch strip. Escrita na tela. Cor arbitrária nos
16 pads, nos 25 LEDs da strip e nos 39 LEDs de botão. Tudo pela API HID padrão, sem WinUSB.

## Captura USB: ferramenta pronta, init ainda não capturada

### O que já funciona da captura

USBPcap instalado e funcional **depois de reiniciar o Windows** (o driver de filtro só acopla
aos controladores no boot). Detalhe que custou uma tentativa: é preciso passar `-A`
(`--capture-from-all-devices`), senão o USBPcapCMD fica esperando uma escolha interativa de
dispositivos e não grava nada.

Validado: gravei o nosso próprio tráfego e os pacotes aparecem certinhos, 265 bytes para a tela e
81 para os LEDs, no endpoint `0x01`. Script em `scripts/captura-usb.ps1`.

O Wireshark também está instalado; a análise é feita com `tshark`.

### O frame de LED que o serviço da NI manda (capturado)

```
80 | 7c 7c 7c 7c 7c 7c 7c 7c 00 00 7c 7c ... 7c | 0c 0c 0c 0c 0c 0c 0c 0c 0c 0c 0c 0c 0a 06 0c 0c | 00 x25
^id  ^ 39 bytes de botões                        ^ os 16 pads                                      ^ strip
```

Isso confirma o layout e corrige a codificação de brilho:

- Botão aceso fraco = `0x7c`. A tabela do pymikro (`[0, 13, 12, 10, 11]`) está errada.
- O byte é `(cor << 2) | brilho`, com brilho nos dois bits baixos. **Brilho 0 não é apagado**,
  é o nível mais fraco. O que apaga é o byte inteiro em `0x00`.
- Valores de brilho conhecidos como bons: `0x7c` fraco, `0x7e` normal, `0x7f` forte.

### Sequência do serviço da NI, com o aparelho JÁ inicializado

| t | Operação |
|---|---|
| +0,000s | GET_DESCRIPTOR (enumeração) |
| +0,008s | GET_REPORT feature `0xf8`, devolve 11 bytes |
| +0,010s | GET_REPORT feature `0xd0`, devolve 33 bytes |
| +0,014s | GET_REPORT **input `0x01` pelo canal de controle**, devolve 14 bytes |
| +0,016s | GET_REPORT feature `0xd0` de novo |
| +0,126s | escreve o frame de LED (81 bytes) |
| +0,136s | escreve a tela, dois pacotes de 265 bytes |

**Só leituras antes de escrever. Nenhum comando de inicialização.**

Reproduzimos essa sequência exata em Rust (`envia.exe --init`), com o serviço da NI parado e o
aparelho reiniciado: as leituras devolvem os mesmos dados, e os LEDs continuam sem acender.
Também testado segurando o handle aberto e reenviando a 20 Hz: sem efeito.

### Por que essa captura não serve

**Desabilitar e reabilitar o aparelho pelo Windows não corta a energia.** O aparelho não volta ao
estado "surdo", então o serviço da NI não precisou mandar a inicialização de verdade: a captura
pegou uma conversa comum, não a inicialização.

Só o religamento físico do cabo devolve o aparelho ao estado inicial. Duas tentativas de capturar
isso falharam porque o cabo não foi religado dentro da janela de gravação.

### Próximo passo, único

Rodar `scripts/captura-init.ps1` (janela de 45 segundos) e **religar o cabo fisicamente** durante
a contagem, com o serviço da NI rodando. Depois procurar no dump o que o serviço manda que a gente
ainda não manda.

Comandos de análise já prontos:

```bash
TS="/c/Program Files/Wireshark/tshark.exe"
D="$HOME/AppData/Local/Temp/mikrodeck-captura"
"$TS" -r "$D/init-USBPcap1.pcap" -Y 'usb.device_address == 1' \
  -T fields -e frame.time_relative -e usb.transfer_type -e usb.endpoint_address \
  -e usb.endpoint_address.direction -e usb.data_len -e usb.bmRequestType \
  -e usb.setup.bRequest -e usb.setup.wValue -e usb.capdata
```

# Spike HID — resultados (2026-09-01)

Binários em `mikrodeck/spike-hid/`. Toolchain Rust instalado nesta máquina (stable MSVC).

## O aparelho

| Item | Valor |
|---|---|
| VID / PID | `0x17cc` / `0x1700` |
| Produto | `Maschine Mikro MK3 HID` |
| Serial | um por aparelho |
| Interface usada | `MI_00`, HID vendor-defined, usage page `0xff01` |
| Outra interface | `MI_01`, DFU (atualização de firmware). Não serve e não deve ser mexida. |

Nenhum software da Native Instruments está instalado nesta máquina. O aparelho responde
sem driver nenhum, o que confirma a premissa central do projeto.

## Report descriptor (lido do próprio aparelho)

| Report | Tipo | Dados | Total com ID | Uso |
|---|---|---|---|---|
| `0x01` | INPUT | 13 bytes | 14 | botões, knob, touch strip |
| `0x02` | INPUT | 63 bytes | 64 | pads com pressão |
| `0x80` | OUTPUT | 80 bytes | **81** | LEDs de botões, pads e strip |
| `0xe0` | OUTPUT | 264 bytes | **265** | tela |
| `0xf3` | OUTPUT | 1 byte | 2 | desconhecido |
| `0xf4` | OUTPUT | 32 bytes | 33 | desconhecido |
| `0xd0` `0xd8` `0xd9` `0xf8` | FEATURE | até 32 bytes | | desconhecido |

## Layout do report 0x80 (confirmado pelo tamanho)

| Bytes | Conteúdo |
|---|---|
| 0 | ID `0x80` |
| 1 a 39 | 39 LEDs de botões |
| 40 a 55 | 16 pads |
| 56 a 80 | 25 LEDs da touch strip |

Soma: 39 + 16 + 25 = 80 bytes de dados. Bate exatamente com o descriptor.
O pymikro manda 91 bytes porque aloca 10 a mais que sobram sem uso.

Codificação: pads e strip usam `0x04 * índice_da_cor + intensidade`, intensidade de 0 a 3.
Cores: off, red, orange, orange_light, yellow_warm, yellow, lime, green, mint, cyan,
turquoise, blue, plum, violet, purple, magenta, fuchsia, white.
Botões usam brilho da tabela `[0, 13, 12, 10, 11]` para os níveis 0 a 4.

## Tela

128 x 32 pixels, 1 bit por pixel. Enviada em dois pacotes de 265 bytes (report `0xe0`),
cada um com 128 x 16 pixels. Header de 9 bytes: `[0xE0, x_lo, x_hi, y_lo, y_hi, w_lo, w_hi, h_lo, h_hi]`
com largura 128 e altura 2 em unidades de 8 linhas. O `y` do segundo pacote é 2.
**O bitmap é invertido**: bit 1 apaga o pixel, bit 0 acende. Confirmado na prática.

## Riscos do CLAUDE.md: situação

### Risco 1 — convivência com o serviço da NI: não se aplica aqui
Nenhum serviço da NI roda nesta máquina e o aparelho abre normalmente. Se um dia o
Maschine for instalado, o ponto a vigiar é o `share_mode` do `CreateFile`. O hidapi
moderno já abre com `FILE_SHARE_READ | FILE_SHARE_WRITE`, então deve conviver.

### Risco 2 — byte 0 de report ID no Windows: a nota estava errada
O aparelho usa report IDs explícitos. O buffer tem que começar com o ID real (`0x80`,
`0xe0`), **sem** prefixo `0x00`. A regra do byte zero vale só para aparelhos sem report ID.

### Risco 3 — tela: resolvido
128 x 32, 1 bpp, invertida, dois pacotes. Testado e funcionando.

## O que funciona hoje no Windows

- **Leitura**: perfeita. Chegam os reports `0x01` e `0x02`, com pressão dos pads de 0 a 4095.
- **Tela**: perfeita e repetível. Acende, apaga, desenha.
- **LEDs**: não funcionam. Nenhuma escrita do report `0x80` tem efeito.

## Por que os LEDs não acendem no Windows

O Windows obriga toda escrita HID neste aparelho a ter `OutputReportByteLength` bytes,
que é 265, o tamanho do maior output report (o da tela). Isso vale para os três caminhos:

- `hid_write` do hidapi: preenche o buffer com zeros até 265 antes de chamar `WriteFile`.
- `WriteFile` chamado direto: com um buffer de 81 bytes, o Windows ainda lê e envia 265.
- `HidD_SetOutputReport`: falha com erro 31 (`ERROR_GEN_FAILURE`), o aparelho recusa.

A tela funciona justamente porque o report `0xe0` já tem 265 bytes naturalmente.
O report de LEDs tem 81 e chega inflado.

Mecanismo provável: em USB, o tamanho da transferência é delimitado por um pacote curto.
Uma transferência de 81 bytes termina com um pacote curto de 17 bytes; uma de 265 termina
com um de 9. O firmware valida o tamanho total contra o tamanho do report e descarta o
pacote que não bate.

### O que foi testado e descartado

| Hipótese | Como foi testada | Resultado |
|---|---|---|
| Codificação de cor errada | Todos os 80 bytes com `0x07`, `0x06`, `0x0D` e `0xFF` | Nada acende |
| Layout de offsets errado | Varredura por faixas: 1-20, 21-40, 41-60, 61-80 | Nada acende |
| Falta de brilho global | `0xf3` com `0x01`, `0x40`, `0x7F`, `0xFF`, depois LEDs | Nada acende |
| Falta de inicialização | `0xf4` preenchido, depois LEDs | Nada acende |
| Sem permissão de escrita | `CreateFileW` com `GENERIC_READ \| GENERIC_WRITE` | Handle abre, e a tela responde |
| Caminho de escrita quebrado | Alternar a tela três vezes | Responde sempre |

Verificação feita por webcam (Insta360 Link 2C) apontada para o aparelho, com foto
automática após cada tentativa. Script em `scripts/olho.sh`.

## Modo MIDI: descartado (pesquisado em 2026-09-01)

O Mikro MK3 entra em modo MIDI com SHIFT + botão Project (o da engrenagem). Não serve:

- **É emulado pelo serviço da NI**, não é nativo do firmware. Os controladores Maschine não são
  class-compliant; a porta MIDI que aparece no sistema é criada pelo software da NI, que fala HID
  com o aparelho e traduz. Prova indireta forte: o DrivenByMoss usa o modo MIDI do Mikro MK3 e
  funciona no Windows e no Mac, mas não no Linux, justamente por depender do serviço da NI.
- **Os LEDs ficam pobres**: 2 estados por pad (on/off) e cor escolhida numa paleta de 16, dentro do
  Controller Editor. O MIDI de retorno só alterna o estado, não escolhe cor. Um Stream Deck precisa
  de cor arbitrária por pad em tempo real.
- **A cor sairia do controle do app**: ficaria num template da NI, não na nossa config.
- **Perde o que já funciona**: a tela (report `0xe0`) e a pressão dos pads não passam pelo modo MIDI.

Conclusão: o modo MIDI contraria a premissa de motor autônomo sem driver e entrega menos do que
o HID já entrega hoje.

## Captura USB com Wireshark: o que esperar

O USBPcap está instalado, mas o driver de filtro só se acopla aos controladores USB depois de
reiniciar o computador. Antes disso, `USBPcapCMD --extcap-interfaces` não lista nada e o
dispositivo `\.\USBPcap1` não existe, mesmo rodando como administrador.

Vale notar: como o software da NI usa **driver próprio** (os Maschine não são class-compliant),
ele não passa pela pilha HID do Windows. Ou seja, uma captura mostraria o tamanho real das
transferências no fio, o que confirmaria o diagnóstico, mas não daria uma receita que dê para
reproduzir pela API HID do Windows. O caminho continuaria sendo o acesso USB cru.

## Caminho para resolver

O HID do Windows não consegue entregar o report `0x80` no tamanho certo. Para mandar
transferências de tamanho exato é preciso acesso USB cru na interface `MI_00`:

- **WinUSB via Zadig**: troca o driver da `MI_00`. Leitura e escrita passam a ser feitas
  por libusb (crate `rusb`), com controle total do tamanho. O aparelho deixa de ser HID
  enquanto o driver estiver trocado, então o software da NI não funcionaria (irrelevante
  nesta máquina, que não tem nada da NI instalado). Reversível pelo Gerenciador de Dispositivos.
- **libusbK como filtro**: mantém o HID no lugar e adiciona acesso cru por cima. Menos
  invasivo e mais fácil de reverter, porém menos comum.

Para distribuição futura, a instalação do driver pode ser automatizada com libwdi,
sem o usuário abrir o Zadig na mão.

## WinUSB instalado (2026-09-01, sessão 2)

Driver da interface `MI_00` trocado de HID para WinUSB pelo Zadig. Agora falamos por libusb
(crate `rusb`), no endpoint de interrupção `0x01` (saída) e `0x81` (entrada), pacote máximo 64.

O que isso resolveu:

- **Tamanho exato**: mandamos 81 bytes e o aparelho recebe 81. Antes o Windows inflava para 265.
  Confirmado também que o backend HID do libusb inflava igual: passando 81, ele reportava 265.
- **Tela por WinUSB**: escrevi "WinUSB OK / teste 123" e apareceu. As escritas são interpretadas.
- **Leitura por WinUSB**: pads com pressão continuam chegando normalmente.
- **Report descriptor por transferência de controle**: confirmei na mão que o report `0x80` tem
  80 bytes de dados e valor lógico máximo 127.

O que ainda **não** funciona: os LEDs. O report `0x80` é aceito sem erro e ignorado.

### Feature reports lidos do aparelho

| Report | Conteúdo | Leitura |
|---|---|---|
| `0xd0` | config | `d0 01 00 00 17 0a 50 ff...` — brilho global de LED já em 10, o máximo. `0x50` = 80 num campo de 0 a 100. |
| `0xd8` | info, só leitura | contém o PID `0x1700` e o serial curto do aparelho |
| `0xd9` | serial em texto | 24 caracteres, um por aparelho |
| `0xf8` | tela | `f8 80 00 20 00 01 00 00 00 64 00` — os `80 00` e `20 00` são 128 e 32, as dimensões da tela |

Os campos graváveis do `0xf8` aceitam `SET_REPORT` mas não mudam: relendo, voltam iguais.

### Tudo que foi testado nos LEDs, sem efeito

Tamanhos 41, 42, 62, 64, 80, 81, 82, 84, 88, 90, 91, 92, 96, 265. Valores `0x01`, `0x03`, `0x06`,
`0x07`, `0x0D`, `0x0F`, `0x3F`, `0x47`, `0x7F`, `0xC0`, `0xFF` em todos os bytes e em faixas
isoladas. Report IDs `0x80`, `0x81`, `0x82`, e sem ID nenhum. Envio contínuo a 60 Hz por 12 e por
60 segundos. Brilho `0xf3` de 1 a 10 antes e depois do frame. Report `0xf4` preenchido.
Escrita de feature `0xd0` e `0xf8`. `SET_REPORT` de output pelo pipe de controle: o aparelho dá
stall (`Pipe error`).

Medição feita por webcam com desconto de exposição (a parede ao fundo serve de referência),
script `scripts/mede.js`. Sem isso, variação de exposição da câmera passa por LED aceso.

### Descoberta importante: o aparelho nunca acende nada

Gravei em vídeo o ciclo de desconectar e reconectar o cabo USB. A tela apaga, o aparelho reinicia
e volta com o logo "MASCHINE MIKRO". **Em nenhum instante acende um LED**, nem animação de boot.
Ele fica em espera até algum software assumir.

Restam duas explicações, e elas se separam com um teste só:

1. **Falta um comando de "host conectado"** que ainda não descobrimos, e que o software da NI manda.
2. **Defeito de hardware**: um trilho de alimentação de LED morto explicaria os 39 botões, os 16 pads
   e os 25 LEDs da strip falharem todos juntos, com o resto do aparelho funcionando.

Teste que decide: instalar o software da NI (Controller Editor serve). Se ele acender os LEDs, o
problema é de protocolo e a captura USB entrega o comando que falta. Se nem ele acender, é hardware.

## PROVA: os LEDs funcionam. O hardware está bom.

Durante um teste de escrita sem intervalo entre frames, o aparelho recebeu 374 escritas e recusou
790 mil. A enxurrada travou o firmware, ele **reiniciou e tocou a animação de inicialização**:
os 16 pads acesos em amarelo, magenta, verde, ciano, azul e rosa, apagando aos poucos ao longo de
0,9 segundo. Capturado em vídeo, quadros 103 a 111 de `fluxo.mp4`.

As cores eram variadas, e eu tinha mandado tudo vermelho. Ou seja, não era o meu comando: era o
firmware acendendo os LEDs sozinho.

Isso elimina a hipótese de defeito de hardware. O painel de LED está inteiro e o firmware sabe
acioná-lo. **Falta o comando que passa o controle para o host.**

### Limite de taxa

O endpoint aceita cerca de 37 escritas por segundo. Com 16 ms de intervalo (60 Hz nominal) não dá
erro nenhum. Sem intervalo, quase tudo é recusado e o firmware trava. Não floodar.

### Ainda sem efeito

Reinício por USB (`libusb reset`) seguido de escrita: o reset de porta não corta energia, não
reproduz a animação, e os LEDs continuam apagados. Ler e escrever ao mesmo tempo também não muda.

### Regra do tamanho da transferência (descoberta em 2026-09-01)

O comando da tela (`0xe0`) aceita coordenadas e tamanho no header. Mandei um retângulo pequeno
(x=0, y=0, largura 32, altura 8 linhas) de duas formas:

| Transferência | Resultado |
|---|---|
| 41 bytes (header + 32 de bitmap) | nada acontece |
| 265 bytes (o mesmo header, resto preenchido) | **o retângulo aparece na tela** |

Ou seja: **cada report precisa ser enviado no seu comprimento declarado completo**. O header diz o
que desenhar, mas a transferência tem que vir cheia. Confirmado visualmente.

Isso não explica os LEDs: o comprimento declarado do `0x80` é 81 bytes e é isso que mandamos.
Também testei o `0x80` numa transferência de 265 bytes, com o resto em zeros e com o resto
preenchido por comandos válidos de brilho (`0xf3 0x0a` repetido). Nada acende.

### Layout dos LEDs confirmado por três implementações independentes

`openAV-Ctlra` (C, libusb no mesmo endpoint), `r00tman/maschine-mikro-mk3-driver` (Rust) e
`Desidiosus/maschine-mikro-mk3-driver` (fork ativo) concordam:

- Report `0x80`, 81 bytes, endpoint de interrupção `0x01`, interface 0. **Sem handshake nenhum.**
- Offsets 1 a 39 botões, 40 a 55 pads, 56 a 80 strip.
- Byte = `(cor << 2) | brilho`, brilho de 0 a 3.
- Brilho de botão: `0x7C` fraco, `0x7E` normal, `0x7F` forte. A tabela do pymikro
  (`[0, 13, 12, 10, 11]`) não bate com as outras duas e deve ser ignorada.
- Ordem dos pads no buffer: 13, 14, 15, 16, 9, 10, 11, 12, 5, 6, 7, 8, 1, 2, 3, 4.

Reproduzi a sequência de inicialização do fork Desidiosus na ordem exata (sensibilidade dos pads
`0xf4`, contraste `0xf8` por feature, brilho global `0xf3 0x0a`, depois o frame `0x80`). Nada acende.

Também testado sem efeito: transferência bulk em vez de interrupção, quatro envios seguidos,
e escuta do eco de 81 bytes que o Ctlra menciona (o aparelho não devolve nada).

Detalhe: o buffer de leitura precisa ter mais de 81 bytes. Com 64 a transferência estoura e o
pacote é perdido em silêncio. Foi um erro meu na primeira versão do leitor.

### Software da NI instalado (2026-09-02)

O Controller Editor sozinho **não** detecta o aparelho. Foi preciso instalar e abrir o Maschine 2
antes; depois disso o Controller Editor passou a conectar. Processos que sobem junto:
`NIHardwareService`, `NIHostIntegrationAgent`, `NIHardwareAccessibilityHelper` e `nimc3cpl`
(o Control Panel do Mikro MK3).

Detalhe operacional importante: **com o driver WinUSB instalado pelo Zadig, nada da NI enxerga o
aparelho.** Para usar o software da NI é preciso remover o pacote de driver da libwdi
(`pnputil /delete-driver <oemXX.inf> /uninstall /force`), rodar `pnputil /scan-devices` e reiniciar
o `NIHardwareService`. Depois disso a interface volta para `HidUsb` com o `input.inf` da Microsoft.
Os scripts `scripts/reverter-driver.ps1` e `scripts/reiniciar-ni.ps1` fazem isso.

Ou seja, os dois mundos não convivem: ou WinUSB para o nosso código, ou HID para o software da NI.
Trocar leva menos de um minuto com os scripts.

### PROVA FINAL: o software da NI acende tudo no Windows (2026-09-02)

Com o Maschine 2 instalado e o Controller Editor conectado, o aparelho acende por completo:
os 16 pads em laranja, um pad em vermelho forte, todos os botões retroiluminados, PLAY em verde,
REC em vermelho, e a tela mostrando "MIDI MODE". Registrado em foto.

Conclusões que isso fecha:

- Os LEDs funcionam no Windows. Não é limitação do sistema operacional.
- Não é defeito de hardware.
- O nosso frame `0x80` está incompleto ou falta um comando antes dele. O software da NI sabe
  o que é, e a captura USB vai mostrar.

### Próximo passo definido

Capturar o que o software da NI manda ao assumir o aparelho. Ordem:

1. ~~Instalar o software da NI~~ **feito**. O Controller Editor exige o Maschine 2 instalado.
2. ~~Reverter o driver para HID~~ **feito**, com `scripts/reverter-driver.ps1`.
3. ~~Confirmar que o software da NI acende os LEDs~~ **feito**, acende tudo.
4. **Reiniciar o Windows**, para o driver de filtro do USBPcap acoplar aos controladores USB.
5. Capturar com `scripts/captura-usb.ps1` enquanto o Maschine conecta e acende os LEDs.
6. Achar no dump o comando que falta e reproduzir em Rust.
7. Voltar o driver para WinUSB com o Zadig e validar.

## Binários do spike

| Binário | O que faz |
|---|---|
| `spike-hid` | Abre o aparelho, acende um pad e imprime os eventos de entrada |
| `diag` | Lê e analisa o report descriptor, testa caminhos de escrita |
| `tela` | Escreve na tela |
| `leds265` | Testa `HidD_SetOutputReport` com o tamanho que o Windows exige |
| `raw` | `CreateFileW` e `WriteFile` direto, sem hidapi |
| `envia` | Manda um pacote arbitrário, para experimentação |
