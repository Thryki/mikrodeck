# Animações de luz

Desenho final, fechado em 2026-09-04 depois de um painel com três propostas.
A base é a proposta vencedora ("seis modos, um matiz por vez, brilho como
ritmo"). As outras duas entraram onde os juízes marcaram: o retrato do último
frame enviado, o teste de orçamento de escritas e o ganho automático do
medidor de som. O que os juízes derrubaram está na seção 7, com o motivo.

## 1. O que é e por que existe

Quando o aparelho fica parado, a tela já roda um texto. Agora os 16 pads
também se mexem: respiram, percorrem a borda, varrem em colunas, pulsam do
centro ou viram medidor do som que sai do Windows. Ao apertar um pad, a luz
faz um eco curto ao soltar. Tudo dentro de uma regra só: o MikroDeck é uma
central de utilidades, não um brinquedo. Um matiz por vez, o brilho faz o
ritmo, brilho 3 aparece só no ponto de atenção, e só os 16 pads se movem. O
silêncio dos botões e da strip em volta é parte do desenho. O aparelho aceita
30 escritas por segundo e a tela em descanso gasta 16; toda animação aqui cabe
nas 14 que sobram, com folga provada em conta na seção 6.

## 2. Os modos

Regras que valem para todos os modos:

| Regra | Como |
|---|---|
| Teto de brilho | Brilho geral B da config (padrão 2). Nenhum quadro passa dele. |
| Brilho 3 | Só no ponto de atenção: cabeça do cometa, pico do medidor, eco. |
| Fraco e apagado | Brilho 0 é o nível fraco. Apagado é `Cor::Apagado`. São níveis diferentes e os modos usam os dois. |
| Um matiz por quadro | Todo modo pinta uma cor só por vez. Exceção: Respiração em cor automática, que usa as cores da página. |
| Roda de cores | `RODA: [Cor; 8]` = vermelho, laranja, amarelo, verde, ciano, azul, violeta, magenta. Cor "auto" anda um passo nessa roda. Cor fixa: só o brilho anda. |
| Passo mínimo | 125 ms em todo modo. Teto duro de 8 quadros por segundo, não importa a frequência do laço. |
| Botões e strip | Não animam. A strip cai para o brilho fraco e segue mostrando o nível dela. Botões ficam como estão. |
| Ritmo | Lento, médio, rápido. Muda o passo. Ignorado no modo Som. |

Passo por ritmo, nos modos de passo fixo (Contorno, Colunas, Pulso):

| Ritmo | Passo | Motivo |
|---|---|---|
| Lento | 500 ms | Dobro do médio. |
| Médio | 250 ms | Padrão. Múltiplo do passo da tela (125 ms), múltiplo do laço (25 ms). |
| Rápido | 150 ms | Não é 125 ms de propósito: coincidir com o passo da tela dá tremor visível. Os dois juízes pediram isso. Múltiplo do laço de 25 ms. |

Respiração e Pulso têm período em vez de passo: médio 4 s, lento 8 s, rápido 2 s.
Trocar entre os dois não troca o ritmo da mesa.

### Tabela dos modos

| id | Nome | O que acende | Cor | Quadros/s (lento / médio / rápido) | Escritas/s no pior caso, com a tela |
|---|---|---|---|---|---|
| `nenhuma` | Nenhuma | Nada muda nos pads. Só o texto na tela. | página | 0 | 16 |
| `respiracao` | Respiração | A página inteira sobe e desce de brilho. Pads vazios ficam apagados. É o padrão. | auto = cor de cada pad; fixa = uma cor nos 16 | até 0,75 / 1,5 / 3 | 19 |
| `contorno` | Contorno | Um cometa percorre a borda e fecha no centro, com rastro. | um matiz, passo na roda a cada volta | 2 / 4 / 6,7 | 23 |
| `colunas` | Colunas | Uma coluna de 4 pads varre da esquerda para a direita e volta. | um matiz, passo na roda a cada duas varreduras | 2 / 4 / 6,7 | 23 |
| `pulso` | Pulso | Uma gota no centro se espalha para a borda e some. Espera. Repete. | um matiz, passo na roda a cada pulso | 0,6 / 1,25 / 2,5 | 19 |
| `som` | Som | Medidor estéreo da saída de áudio, de baixo para cima, com ponto de pico. | um matiz, passo na roda a cada 8 s | até 8, só quando muda | 24 |

Sem texto na tela (campo vazio) a tela não escreve e sobram as 30 para os LEDs.

### Respiração

Só os pads com controle participam. Os vazios ficam apagados, então o desenho
da página se mantém e dá para achar o pad no escuro. O pad de programa aberto
respira na cor de aberto (`cor_do_controle` já resolve). Nunca chega em
apagado: o vale é o brilho fraco.

Ciclo no médio, começando do alto para emendar com a página sem costura.
Degraus do meio curtos, extremos longos, como uma senoide amostrada:

| B | Sequência (nível: ms) | Período | Trocas/s |
|---|---|---|---|
| 3 | 3: 900, 2: 300, 1: 400, 0: 1700, 1: 400, 2: 300 | 4000 | 1,5 |
| 2 | 2: 1000, 1: 500, 0: 2000, 1: 500 | 4000 | 1 |
| 1 | 1: 1500, 0: 2500 | 4000 | 0,5 |
| 0 | fraco parado | sem quadro | 0 |

Toda duração é múltiplo de 50 ms, então o rápido (metade) cai em múltiplos de
25 ms e bate com o laço. Isso conserta o 187,5 ms que um juiz pegou na
proposta.

### Contorno

Caminho exato do pedido do Thryki, em `CONTORNO: [u8; 16]`:

```
1, 5, 9, 13, 14, 15, 16, 12, 8, 4, 3, 2, 6, 10, 11, 7
```

Cometa: cabeça em B, o pad anterior em B-1, o seguinte em B-2, até o fraco
(0), e depois apagado. Com B=2 são 3 pads acesos; com B=3, quatro; com B=0, um
ponto sem rastro. O salto do 7 para o 1 coincide com o passo na roda de cores,
então lê como "nova volta", não como falha. Uma volta: 8 s, 4 s ou 2,4 s.

### Colunas

Colunas: 1-5-9-13, 2-6-10-14, 3-7-11-15, 4-8-12-16. Vai e volta (1, 2, 3, 4,
3, 2) em vez de saltar da 4 para a 1: sem salto, a varredura vira uma linha
contínua. Coluna atual em B, a coluna de onde ela veio no fraco (0), o resto
apagado. Dois tons, um matiz. Varredura completa em 6 passos (1,5 s no médio).

### Pulso

Dois anéis, que é tudo que uma grade 4x4 tem: `CENTRO = [6, 7, 10, 11]` e
`BORDA` com os 12 restantes. A borda vai um nível atrás do centro:

| Quadro | B=2 | B=3 |
|---|---|---|
| 1 | centro 2 | centro 3 |
| 2 | centro 1, borda 2 | centro 2, borda 3 |
| 3 | centro 0, borda 1 | centro 1, borda 2 |
| 4 | centro apagado, borda 0 | centro 0, borda 1 |
| 5 | tudo apagado | centro apagado, borda 0 |
| 6 | | tudo apagado |

Depois, escuro até completar o período. No médio: 5 quadros em 1,25 s e 2,75 s
de espera. Uma cor por pulso.

### Som

Está na seção 3.

## 3. Reação ao som

É um modo, não um modificador dos outros. Misturar som com Contorno ou Colunas
vira ruído em cima de ruído, e é isso que faz o aparelho parecer brinquedo.

| Item | Como |
|---|---|
| Fonte | `IAudioMeterInformation` no mesmo `IMMDevice` de saída que `Volume::abrir` já ativa em `motor/src/audio.rs`. A feature `Win32_Media_Audio_Endpoints` já está no Cargo.toml. Sem captura, sem thread nova, sem buffer de áudio. Uma chamada COM por tique. |
| Leitura | `GetMeteringChannelCount` + `GetChannelsPeakValues` a cada tique do laço (25 ms), guardando o máximo por canal desde o último quadro, para não perder transiente. |
| Quadro | A cada 125 ms, e só se algum nível mudou. Ritmo da config ignorado. |
| Escala | Pico linear vira dB (20·log10). Faixa de 42 dB mapeada em 12 níveis por coluna. |
| Ganho automático | Teto da escala segue o maior pico recente: sobe na hora, desce 4 dB por segundo, nunca abaixo de -18 dB nem acima de 0 dB. Piso = teto - 42 dB. Música baixa não dá medidor morto. Veio da proposta "seis modos dentro do orçamento" e responde ao risco de o medidor ler depois do volume do Windows. |
| Coluna | 4 pads, cada um enche em três degraus (fraco 0, 1, 2) antes do pad de cima começar: 12 níveis. |
| Pico | O pad onde está o nível máximo alcançado fica em brilho 3. Segura 8 quadros (1 s) e depois cai um nível por quadro. |
| Queda | Ataque imediato, queda de no máximo um nível por quadro. Do topo ao zero em 1,5 s. Não treme. |
| Estéreo | Colunas 1 e 2 (pads 1, 5, 9, 13 e 2, 6, 10, 14) = canal esquerdo. Colunas 3 e 4 = direito. Mono ou mais de 2 canais: as quatro colunas iguais, com o maior pico. |
| Cor | Uma só. Fixa da config, ou auto andando um passo na roda a cada 8 s. Nada de verde, amarelo e vermelho de VU: isso é árvore de natal. A cor não reage ao som. |

Brilho geral abaixo de 2 (um juiz cobrou):

| B | Degraus por pad | Níveis por coluna | Pico |
|---|---|---|---|
| 3 | 3 (0, 1, 2) | 12 | 3 |
| 2 | 3 (0, 1, 2) | 12 | 3 |
| 1 | 2 (0, 1) | 8 | 2 |
| 0 | 1 (0) | 4 | 1 |

Silêncio: 3 s (24 quadros) com todos os canais abaixo de -50 dB e a barra já
no zero, e o modo passa para Respiração com as cores da página. Som de volta
acima do limiar: medidor na hora. A config não muda; é estado interno do
Animador.

Sem dispositivo (`Medidor::abrir` devolve `None`): cai em Respiração e a
interface mostra "sem saída de áudio" em cinza ao lado do seletor, via campo
novo `saida_de_audio` no `Diagnostico`.

Limites conhecidos: o medidor lê depois do volume do Windows (volume zero é
silêncio). Programa em modo exclusivo não alimenta o medidor. Troca de
dispositivo padrão não é seguida, a mesma limitação do `Volume`; resolver os
dois juntos depois.

## 4. Animação ao apertar um pad

Fora do descanso a luz é feedback tátil, não espetáculo. O efeito fica no
próprio pad e vai para baixo em brilho. Nunca espalha para os lados.

| Momento | O que acontece | Escritas |
|---|---|---|
| Apertar | Como hoje: `cor_pressionado` ou branco, brilho 3. | 1 |
| Soltar, `eco` (padrão) | O pad volta na cor de repouso em brilho 3, cai para 2 depois de 100 ms, e para o brilho de repouso do pad depois de 200 ms. A luz "pousa" de volta. Brilho de repouso 3: nada a fazer, o frame não suja. | 2 a mais |
| Soltar, `nenhuma` | Como hoje. | 1 |

O eco usa `quadro_de_repouso`, então respeita o brilho próprio do pad e a cor
de programa aberto.

Dentro do descanso, qualquer evento acorda com corte seco: um quadro só, as
cores da página, sem fade. Quem apertou quer o pad agora; latência zero vale
mais que transição. Vale para `PadTocado` também, o toque leve, então encostar
no pad já mostra a página antes de apertar.

Primeiro aperto no descanso executa sempre. A proposta vencedora deixou em
aberto a regra "primeiro aperto firme só acorda quando o modo esconde as
cores". Ela cai por três motivos apontados por um juiz: (1) quebra memória
muscular, quem aperta "pausar Spotify" no descanso não vê nada acontecer; (2)
o aparelho manda `PadTocado` antes de `PadApertado`, e `compositor.tocou()`
roda antes de `estado.processar`, então na maioria dos apertos o descanso já
acabou quando a regra seria checada, e ela viraria comportamento inconsistente
dependendo da velocidade do dedo; (3) exigiria desviar também o caminho de
`segurando_janela`. Stream Deck sempre age.

Adormecer (entrada no descanso): 3 passos de 250 ms com as cores da página
caindo B, B-1, 0; daí o modo começa do primeiro quadro. Na Respiração isso já
é a primeira descida, sem costura. Botões não animam; a strip cai para o fraco
e segue mostrando o nível.

## 5. Config

JSON em `~/.mikrodeck/config.json`, tudo com `serde(default)` para config
antiga carregar sem migração. Padrões:

```json
"descanso": {
  "ativo": true,
  "texto": "MikroDeck",
  "segundos": 90,
  "luz": {
    "modo": "respiracao",
    "ritmo": "medio",
    "cor": "auto"
  }
},
"ao_apertar": "eco"
```

| Campo | Valores | Padrão |
|---|---|---|
| `descanso.ativo` | interruptor geral: tela e luz | `true` |
| `descanso.texto` | texto da tela; vazio = a tela fica na página, só a luz anima | `"MikroDeck"` |
| `descanso.segundos` | espera, mínimo 5 | `90` |
| `descanso.luz.modo` | `nenhuma`, `respiracao`, `contorno`, `colunas`, `pulso`, `som` | `respiracao` |
| `descanso.luz.ritmo` | `lento`, `medio`, `rapido`; ignorado em `som` | `medio` |
| `descanso.luz.cor` | `auto` ou nome de cor da tabela (`vermelho` a `branco`) | `auto` |
| `ao_apertar` | `nenhuma`, `eco` | `eco` |

Sem campo de brilho: o teto é sempre o brilho geral.

Texto vazio muda de significado. Hoje `definir_descanso` filtra texto vazio e
desliga o descanso inteiro. Com a luz, descanso deixa de ser só a tela: `ativo`
é o interruptor, e texto vazio só deixa a tela na página. O teste
`texto_vazio_nao_liga_o_descanso` vira `texto_vazio_deixa_a_tela_na_pagina`.
Quem já tem texto configurado não sente nada. Um juiz marcou isso como decisão
de produto escondida; está aqui à vista, e é do Thryki a palavra final.

Enums em Rust, em `config.rs`, todos `#[serde(rename_all = "snake_case")]`
como `FuncaoStrip`:

```rust
pub struct LuzDescanso { pub modo: ModoLuz, pub ritmo: Ritmo, pub cor: CorLuz }
pub enum ModoLuz { Nenhuma, Respiracao, Contorno, Colunas, Pulso, Som }
pub enum Ritmo { Lento, Medio, Rapido }
pub enum CorLuz { Auto, Fixa(Cor) }   // serializa como string: "auto" ou o nome, via cor_do_nome
pub enum AoApertar { Nenhuma, Eco }
```

### Configurações (interface)

Seção "Descanso de tela" mantém o checkbox e a linha Texto / Depois de.
Abaixo entra a linha "Luz dos pads", sentence case, com o mesmo componente
dos seletores de strip e knob:

| Controle | Opções | Estado |
|---|---|---|
| Modo | Nenhuma, Respiração, Contorno, Colunas, Pulso, Som | sempre ativo |
| Ritmo | Lento, Médio, Rápido | desabilitado em Som e Nenhuma |
| Cor | Automática + as 17 cores, com a bolinha que o painel do pad já usa | desabilitado em Nenhuma |
| Ver agora | botão à direita: força o descanso no aparelho na hora | para não esperar 90 s a cada ajuste |

Com modo Som e `saida_de_audio` falso no diagnóstico: texto pequeno em cinza,
"sem saída de áudio", ao lado do seletor.

Seção nova "Pads", pequena, com um checkbox: "Eco de luz ao soltar o pad".
Grava `ao_apertar`. O brilho geral fica onde está, na tela principal.

`tipos.ts` e `ponte.ts` ganham os campos com os mesmos padrões, para o
navegador sem aparelho continuar funcionando. Rótulos em `ROTULOS_MODO_LUZ` e
`ROTULOS_RITMO`.

### MCP

`definir_ajustes` ganha quatro argumentos: `descanso_luz_modo`,
`descanso_luz_ritmo`, `descanso_luz_cor` e `ao_apertar`, com enum no schema
para o assistente não chutar. `ler_configuracao` imprime os quatro na linha do
descanso: `Descanso: "MikroDeck" depois de 90s, luz respiracao medio auto. Ao
apertar: eco.`

## 6. Arquitetura no motor

### Módulo novo: `motor/src/luz/`

Sem HID, sem `Instant` escondido: recebe o tempo e devolve pads. Testável sem
aparelho, como o `estado`.

| Arquivo | O que tem |
|---|---|
| `luz/mod.rs` | `Quadro`, `Animador`, `EstadoSom`, `PASSO_MINIMO` |
| `luz/modos.rs` | funções puras de cada modo e as constantes `CONTORNO`, `CENTRO`, `BORDA`, `RODA`, `linha(pad)`, `coluna(pad)` |

```rust
/// Índice = pad impresso menos 1. Cor::Apagado para apagado.
pub type Quadro = [(Cor, u8); 16];

pub struct Animador {
    luz: LuzDescanso,
    ao_apertar: AoApertar,
    /// Quando adormeceu. None = acordado.
    inicio: Option<Instant>,
    /// Último passo calculado. Passo igual não recalcula nada.
    passo_visto: Option<u64>,
    quadro: Option<Quadro>,
    /// Pad em eco e quando foi solto.
    eco: Option<(u8, Instant)>,
    /// Teto do ganho, nível por coluna, pico e quadros em silêncio.
    som: EstadoSom,
}

impl Animador {
    pub fn novo(luz: LuzDescanso, ao_apertar: AoApertar) -> Self;
    /// A config mudou pela interface ou pelo MCP.
    pub fn configurar(&mut self, luz: LuzDescanso, ao_apertar: AoApertar);
    /// Devolve true se o quadro mudou. `pico` só vem no modo Som.
    pub fn tique(&mut self, agora: Instant, dormindo: bool, brilho: u8,
                 repouso: &Quadro, pico: Option<(f32, f32)>) -> bool;
    /// O que pintar por cima dos pads, se houver.
    pub fn quadro(&self) -> Option<&Quadro>;
    /// Corte seco: zera inicio e quadro.
    pub fn acordar(&mut self);
    pub fn eco(&mut self, pad: u8, agora: Instant);
    /// Dormindo ou eco em curso: o laço acelera para 25 ms.
    pub fn precisa_de_tique(&self) -> bool;
    /// Modo Som e dormindo: o laço lê o medidor.
    pub fn quer_som(&self) -> bool;
}
```

`tique` faz: passo = (agora - inicio) / passo_do_modo. Passo igual ao último:
devolve `false` sem tocar em nada. Passo novo: chama a função pura do modo e
guarda o `Quadro`. Os primeiros 750 ms são o adormecer (exceto na Respiração).

Funções puras em `modos.rs`, uma por modo, todas `(passo, brilho, cor, ...)
-> Quadro`. Som recebe `&mut EstadoSom` e o pico.

### Quem chama quem

```
laço (servico.rs), a cada tique ou evento
  |
  |-- compositor.dormindo()            -> bool
  |-- medidor.picos()                  -> Option<(f32, f32)>, só se animador.quer_som()
  |-- estado.quadro_de_repouso(abertos)-> Quadro
  |-- animador.tique(...)              -> bool
  |-- repintar(aparelho, estado, abertos, animador.quadro())
  |       '-- estado.pintar_com(frame, abertos, Option<&Quadro>)
  '-- atualizar_tela(...)
```

Mudanças por arquivo:

| Arquivo | Mudança |
|---|---|
| `hid/frame.rs` | Guarda `enviado: [u8; 81]` em `marcar_limpo`; `esta_sujo` vira `bytes != enviado`. Teste: escrever A, B, A não suja. Os dois juízes conferiram no código que hoje `sujo` nunca desfaz: pintar repouso e por cima pintar a animação deixaria o frame sujo com bytes iguais ao último enviado, uma escrita perdida por tique, e as cores sairiam erradas como no spike. Entra primeiro e vale mesmo sem animação. |
| `estado.rs` | `pintar_com(frame, abertos, luz: Option<&Quadro>)`. Com `Some`, o laço de pads escreve o quadro em vez das cores de repouso, uma vez só, e a strip cai para o fraco. Os outros chamadores passam `None`. Ganha `quadro_de_repouso(abertos) -> Quadro`, que a Respiração e o eco usam. Pausado continua limpando tudo; o quadro não é passado. |
| `render/composicao.rs` | `dormindo()`: parado >= espera, sem segurando, sem aviso ativo, descanso ativo, independente do texto. `forcar_descanso()`: joga `ultimo_toque` para trás da espera. `cena_atual` só devolve `Descanso` com texto; sem texto segue `Pagina`. |
| `servico.rs` | `recv_timeout` de 25 ms enquanto `animador.precisa_de_tique()`, 100 ms no resto. No timeout: `atualizar_nivel_strip` a cada 4 tiques quando em 25 ms (uma chamada COM a menos por tique), `dormindo = compositor.dormindo() && !pausado`, `pico` só se `quer_som()`, `animador.tique`, `repintar` com `animador.quadro()`, `atualizar_tela`. Em evento: `compositor.tocou()` e `animador.acordar()` antes de tudo; em `PadSolto`, `animador.eco(pad, agora)`. `AtomicBool` `descanso_pedido` ao lado de `teste_pedido`, atendido com `compositor.forcar_descanso()`. O caminho pad -> estado -> ação não muda uma linha. |
| `audio.rs` | `Medidor { meter: IAudioMeterInformation }` com `abrir()` (mesmo enumerador, `eRender`/`eConsole`, `Activate` no mesmo `IMMDevice`) e `picos() -> Option<(f32, f32)>`. Aberto uma vez no supervisor ao lado do `Volume`, com os mesmos `unsafe impl Send/Sync` e a mesma justificativa. |
| `config.rs` | `LuzDescanso`, `ModoLuz`, `Ritmo`, `CorLuz`, `AoApertar`, campo `luz` em `Descanso` e `ao_apertar` em `Config`, todos com default. |
| `app/src-tauri/src/lib.rs` | comando `previsualizar_descanso` (mesmo mecanismo do `testar_leds`); `Diagnostico.saida_de_audio`. |
| `app/src/Configuracoes.tsx`, `tipos.ts`, `ponte.ts` | seletores, checkbox, botão, rótulos, padrões. |
| `mcp/src/ferramentas.rs` | quatro argumentos novos e a linha no resumo. |

### Orçamento de escritas, com conta

Teto real: a thread de escrita roda a 30 Hz (`Aparelho::abrir(30)`), então
são 30 escritas por segundo, não 31. A thread manda uma coisa por tique, LED na
frente, 12 ms entre as duas metades da tela.

| Fonte | Escritas/s |
|---|---|
| Tela em descanso com texto: 8 quadros × 2 pacotes | 16 |
| Tela em descanso sem texto | 0 |
| LED, Respiração, rápido, B=3 | 3 |
| LED, Contorno ou Colunas, rápido (150 ms) | 6,7 |
| LED, Pulso, rápido | 2,5 |
| LED, Som (125 ms, só quando muda) | 8 no máximo, 4 a 6 em música contínua |
| Pior caso: Som + tela com texto | 24 de 30, sobram 6 |
| Segundo pior: Contorno rápido + tela | 22,7 de 30 |

Três travas independentes garantem a conta:

1. Passo mínimo de 125 ms em todo modo: o Animador produz no máximo 8 quadros
   distintos por segundo, não importa a frequência do laço.
2. `frame.rs` só suja quando os bytes diferem do último frame enviado: tique
   sem passo novo não gera escrita, e pintar duas vezes o mesmo valor também não.
3. A thread de escrita segue mandando uma coisa por tique.

Strip e botões vivem no mesmo frame de 81 bytes: não custam escrita extra
quando mudam junto com os pads.

Laço a 25 ms só enquanto `precisa_de_tique()`: 40 acordadas por segundo com um
lock do estado. O volume é lido a cada 4 tiques nesse ritmo. Medir CPU no
descanso na primeira versão; se subir, é aqui que se corta.

### Testes

| Teste | O que confere |
|---|---|
| `frame_nao_suja_ao_voltar_ao_valor_enviado` | Escrever A, B, A deixa o frame limpo. |
| `orcamento_de_escritas` | Para cada modo × ritmo × B, simula 10 s de tiques de 25 ms com relógio falso e conta quadros distintos. Falha se passar de 8 por segundo. Veio das duas outras propostas. Todo modo novo entra aqui antes de entrar no aparelho. |
| `contorno_segue_a_lista_do_davi` | A cabeça passa por 1, 5, 9, 13, 14, 15, 16, 12, 8, 4, 3, 2, 6, 10, 11, 7, nessa ordem. |
| `colunas_vai_e_volta_sem_salto` | 1, 2, 3, 4, 3, 2, 1. |
| `respiracao_mantem_vazios_apagados_e_nunca_apaga_os_cheios` | Vale é fraco, não apagado. |
| `respiracao_rapida_cai_no_laco` | Toda duração no rápido é múltiplo de 25 ms. |
| `um_matiz_por_quadro` | Em todo modo menos Respiração auto, todas as cores não apagadas de um quadro são iguais. |
| `nenhum_quadro_passa_do_brilho_geral` | Fora do ponto de atenção, nível <= B. |
| `som_escala` | -42 dB = 0, 0 dB = 12 com teto em 0; queda de 1 nível por quadro; pico segura 8 quadros; teto do ganho desce 0,5 dB por quadro e para em -18. |
| `som_silencio_vira_respiracao` | 24 quadros abaixo de -50 dB com barra no zero trocam o modo; um quadro acima volta. |
| `eco_pousa_em_200_ms` | 3, 2, brilho do pad. Com brilho 3 não gera quadro. |
| `acordar_e_corte_seco` | Depois de `acordar`, `quadro()` é `None` no mesmo tique. |
| `config_antiga_carrega` | JSON sem `luz` e sem `ao_apertar` dá os padrões. |
| `texto_vazio_deixa_a_tela_na_pagina` | `dormindo()` true, `cena_atual` é `Pagina`. |

Medição no aparelho, além dos testes: a Respiração em 4 níveis pode ler como
piscar lento. Conferir com a webcam e `scripts/mede.js`, não com o olho. Se
piscar, a saída é alongar o vale e encurtar o pico, não inventar nível que o
aparelho não tem.

## 7. O que ficou de fora e por que

| Ideia | De onde | Por que caiu |
|---|---|---|
| Cintilar (brasas sorteadas) | vencedora | Os dois juízes: não carrega informação, é o único modo sem ponto de atenção, e a própria proposta o chamava de "primeiro a cortar". Sete opções no seletor por causa dele não vale. |
| Onda ao apertar (vizinhos sobem um nível) | vencedora e outra | Em sequência de toques vira poluição; a proposta admitia. Num Stream Deck o feedback é o pad apertado, não a mesa piscando em volta. |
| "Primeiro aperto só acorda" nos modos que escondem cor | vencedora | Muda o contrato "apertar = agir" e seria inconsistente pela ordem `PadTocado` -> `PadApertado`. Ver seção 4. |
| Ritmo rápido em 125 ms | vencedora | Coincide com o passo da tela; a proposta só prometia travar em 166 ms "se tremer". Nasceu em 150. |
| Relógio (pads enchem com os minutos) | "máquina de estados no tique de 100 ms" | Sem a hora ninguém lê de relance; mais exceções na UI que qualquer outro modo; precisa de hora local que o std não dá. |
| Som como modificador de outros modos (`reage_ao_som`) | mesma | Em três degraus de brilho o efeito é quase invisível e lê como tremida. Som só onde é o conteúdo. |
| VU verde, amarelo e vermelho | mesma | Árvore de natal. Um matiz. |
| Forma de onda deslizando e espectro por coluna | "seis modos dentro do orçamento" | Onda de verdade pede captura loopback WASAPI com FFT a 30 quadros ou mais. Não cabe nas escritas que sobram e uma grade 4x4 não tem resolução. Se um dia entrar, é "espectro", outro modo, outro spike. |
| Strip como medidor de pico com 25 LEDs | mesma | Contradiz "só os 16 pads se movem". A strip segue mostrando o que ela controla. |
| Esmaecer e Apagar como modos próprios | mesma | A Respiração já deixa a página visível e fraca a maior parte do ciclo, e `nenhuma` já cobre "não mexer". Se o Thryki quiser a página parada e fraca, é uma constante na Respiração. |
| Roda com os 16 matizes | vencedora | Os 16 do aparelho são desiguais (dois laranjas, dois amarelos, cinco entre ameixa e fúcsia). Roda de 8 espaçados. Trocar é uma constante. |
| Campo de brilho na luz | vencedora | O teto é o brilho geral. Menos um controle. |
| Ritmo "tela" no Contorno (cometa sincronizado com o texto) | vencedora | Bonito, não é v1. |

## 8. Ordem de implementação

Cada passo fecha sozinho e dá para testar no aparelho antes do seguinte. Para
testar, baixar `descanso.segundos` para 5 na config.

| Passo | O que entra | Como provar |
|---|---|---|
| 1 | `frame.rs`: `enviado` e o teste A, B, A. | `cargo test`. No aparelho, nada muda. |
| 2 | `config.rs`: enums, `LuzDescanso`, `ao_apertar`, defaults. `tipos.ts` e `ponte.ts` com os mesmos padrões. | Config antiga carrega; a de exemplo grava os campos novos. |
| 3 | `luz/` com `Quadro`, `Animador`, `PASSO_MINIMO` e só a Respiração. `estado.pintar_com` com `Option<&Quadro>` e `quadro_de_repouso`. `compositor.dormindo()`. Laço a 25 ms quando precisa, adormecer. Teste `orcamento_de_escritas`. | Aparelho parado 5 s: a página respira, pads vazios apagados, strip fraca. Encostar num pad: página de volta no mesmo instante. Prova o cano inteiro. |
| 4 | Contorno, roda de cores, ritmo. | Cometa passa por 1, 5, 9, 13... e troca de cor no salto do 7 para o 1. Rápido não treme. |
| 5 | Seletores Modo, Ritmo e Cor em Configurações. MCP com os quatro argumentos. | Trocar pela interface e pelo MCP muda no aparelho em até 1 s, sem reiniciar. |
| 6 | Ver agora: `forcar_descanso`, `descanso_pedido`, comando `previsualizar_descanso`, botão. | Clicar força o descanso na hora. |
| 7 | Colunas e Pulso. | Colunas vai e volta sem salto. Pulso pulsa a cada 4 s no médio. |
| 8 | Eco ao soltar, checkbox em "Pads". | Filmar com a webcam: 3, 2, B em 200 ms. Com brilho geral 3, nada muda. |
| 9 | `Medidor`, modo Som, ganho automático, silêncio vira Respiração, `saida_de_audio` no diagnóstico e o aviso na interface. | Música: barras estéreo com pico. Volume baixo: barra viva pelo ganho. Pausar: 3 s e respira. |
| 10 | Medir com `scripts/mede.js`: escada da Respiração, tremor do rápido, CPU no descanso. Ajustar durações se preciso. | Números, não olho. |
| 11 | Opcional: `Aviso::Pads(Quadro)` a até 8 por segundo para o desenho na interface mostrar a mesma animação, só com a janela visível. | O desenho na tela acompanha a mesa. |
| 12 | De graça com o mesmo motor, depois: animação de boot do ui-spec, uma volta de Contorno em branco ao conectar, e a página acende. | Ligar o cabo. |

Decisões que ficam com o Thryki, já com a recomendação aplicada no texto: texto
vazio deixa a tela na página (seção 5); apertar no descanso executa sempre
(seção 4); ritmo rápido em 150 ms (seção 2).
