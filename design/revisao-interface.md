# Revisão do desenho do aparelho (interface) vs foto HD — rodada 4

Método igual às rodadas anteriores. Tudo normalizado pelo corpo do aparelho.
Corpo na foto: 1963 x 1098 (aspecto **1,788**). Corpo no desenho: 1542 x 864 (aspecto **1,785**).
Percentuais são fração da largura (L) ou da altura (A) do corpo. Precisão ±0,3 ponto percentual.
Diferença abaixo de 1 pp = dentro do aceitável, não vira tarefa.

## 1. Nota geral

**9,4 / 10** (era 9,0; antes 8,7; antes 7,0)

**Subiu:** o aspecto do corpo bateu (1,785 vs 1,788, erro de 0,2%), o knob voltou ao tamanho certo,
a fileira GROUP recuperou a hierarquia de "fileira grossa", a touch strip engrossou, e a captura
veio sem cursor.

**Ficou igual:** grade de pads, coluna central, tela, logo, fileira PITCH, fileira de modo, todos
os rótulos. Nenhum deles regrediu.

**Caiu:** nada. Nenhuma das três correções quebrou vizinho, que era o risco das rodadas 2 e 3.
Só a fileira GROUP passou 1,2 pp do alvo, e isso é cosmético.

O que ainda incomoda de longe: **um único item** — os três botões redondos da esquerda continuam
achatados (razão 1,30 contra 1,03 da foto). O resto é milimetragem.

## 2. Notas por bloco

| Bloco | R1 | R2 | R3 | **R4** | Motivo |
|---|---|---|---|---|---|
| Proporção geral do aparelho | 8,0 | 8,0 | 8,5 | **9,3** | Aspecto 1,785 vs 1,788, cravado. Margens verticais ainda 1,2 pp folgadas (fill 89,4%A vs 91,7%A). |
| Coluna esquerda (redondos, tela, knob, logo) | 7,0 | 8,5 | 8,5 | **9,0** | Knob resolvido (11,6%A vs 11,2%A). Tela 8,6%L x 5,2%A vs 8,66%L x 5,1%A. Redondos ainda achatados. |
| PITCH/MOD/PERFORM/NOTES + touch strip | 7,0 | 9,5 | 9,0 | **9,5** | Strip subiu de 5,7%A para 6,1%A (foto 6,8%A). Fileira PITCH 4,75%A vs 4,55%A. |
| Blocos GROUP / RESTART / PLAY | 5,0 | 8,5 | 9,0 | **9,2** | GROUP de 7,6%A para 10,0%A (foto 8,8%A): passou 1,2 pp. Fileira PLAY ainda baixa: 8,3%A vs 10,3%A. |
| Coluna central (FIXED VEL a MUTE) | 4,0 | 9,5 | 9,5 | **9,5** | Largura 8,0%L vs 8,25%L, topo e base travados. Botão 8,45%A vs 9,56%A, vão 1,85%A vs 1,55%A. |
| Grade de pads e botões de modo | 9,0 | 9,5 | 9,8 | **9,8** | Pad 10,96%L x 19,1%A (foto 10,75% x 18,4%). Vão horizontal 0,84%L vs 1,02%L. Vertical 1,74%A vs 2,55%A. |

## 3. Conferência das três correções desta rodada

| # | Item | Status | Medida |
|---|---|---|---|
| 1 | **Knob, raio 26 → 31, borda esquerda travada na tela** | **RESOLVIDO** | Diâmetro 100 px render / 864 de corpo = **11,6%A**. Foto: 123 px / 1098 = **11,2%A**. Erro 0,4 pp, dentro do aceitável. Alinhamento: borda esquerda do knob em x=135, borda esquerda da tela em x=137 (o knob está 2 px à esquerda). Na foto o knob nasce 10 px à direita da tela (0,5%L). Diferença de 0,6 pp: **não mexer**. |
| 2 | **Fileira GROUP, altura 48 → 53, crescendo para cima** | **PASSOU DO PONTO, por pouco** | Altura 87 px = **10,0%A**. Foto: 97 px = **8,8%A**. Era 7,6%A. Passou 1,2 pp do alvo. A base não se moveu (y=632 no render), como pedido. **Correção opcional**: 50 unidades em vez de 53 cai em 9,4%A. Só faça se for junto de outra mudança; sozinho não vale o risco de mexer no vizinho de novo. |
| 3 | **Corpo 4 unidades mais baixo, viewBox 960x536** | **RESOLVIDO no aspecto, PARCIAL no preenchimento** | Aspecto: **1,785** contra 1,788 da foto. Cravado, era 1,77. Preenchimento vertical: conteúdo ocupa **89,4%A** (era 88,7%A), foto 91,7%A. Margem de topo 5,8%A contra 4,6%A; margem de base 4,9%A contra 3,7%A. Os 4 unidades entregaram o aspecto mas quase nada de fill: 88,7 → 89,4. |

Também resolvido, sem ser da lista: **cursor do mouse fora da captura** (item 15 da rodada 3).

## 4. Diferenças que ainda existem

| # | Onde | O que está no desenho | O que a foto mostra | Gravidade | Correção concreta |
|---|---|---|---|---|---|
| 1 | Três botões redondos (círculo, estrela, lupa) | Caixa 60 x 46 px = 3,9%L x 5,3%A, **razão 1,30** | Caixa 64 x 62 px = 3,26%L x 5,65%A, **razão 1,03**, quase quadrada | **Média** | Reduzir **só a largura**, de 37 para 30 unidades SVG (60 → 48 px). Não tocar na altura nem nos centros: o espaçamento já bate (7,1%A no desenho, 7,06%A na foto) e o vão entre eles também (1,8%A vs 1,37%A). É a única diferença que se vê sem medir. |
| 2 | Fileira PLAY / REC / STOP / SHIFT | Altura 72 px = **8,3%A** | 113 px = **10,3%A** | **Média** | O vão GROUP → RESTART está em 8,2%A e na foto é 7,1%A. Tirar ~9 px de render (≈5 unid. SVG) desse vão e dar para a altura da fileira PLAY, empurrando o topo dela para cima. Base fica onde está. |
| 3 | Margens verticais do corpo | Topo 5,8%A, base 4,9%A, fill 89,4%A | Topo 4,6%A, base 3,7%A, fill 91,7%A | **Média-baixa** | Cortar ~10 px de render (≈6 unid. SVG) da margem de topo e ~10 px da de base, reduzindo o corpo na mesma medida para não estragar o aspecto de 1,785. |
| 4 | Vãos internos da coluna central | Botão 8,45%A, vão 1,85%A | Botão 9,56%A, vão 1,55%A | Baixa | Encolher o vão para 1,55%A e distribuir a sobra nos 8 botões. Topo (linha do pad 13) e base (linha do pad 1) já estão travados: é só redistribuir. |
| 5 | Vão da fileira de modo para a 1ª fileira de pads | 3,6%A | 4,7%A | Baixa | Abrir 10 px de render. Sai de graça junto do item 3. |
| 6 | Vão vertical entre pads | 1,74%A | 2,55%A | Baixa | Abrir o vão e reduzir a altura do pad na mesma medida (o pad hoje está 0,7 pp alto: 19,1%A vs 18,4%A). Perímetro da grade fica onde está. |
| 7 | Coluna VOLUME / SWING / TEMPO | Borda esquerda 9 px (0,58%L) à direita do PERFORM. PLUG-IN 5 px à direita do NOTES | As duas colunas nascem na mesma vertical (x=402 e x=583 na foto) | Baixa | Alinhar VOLUME/SWING/TEMPO com PERFORM e PLUG-IN/SAMPLING com NOTES. Carregado das rodadas 2 e 3. |
| 8 | Topo do VOLUME e do PLUG-IN | y=65, **8 px abaixo** do FIXED VEL e do PAD MODE (y=57) | As três colunas começam na mesma linha (±4 px) | Baixa | Subir VOLUME e PLUG-IN 8 px. Carregado das rodadas 2 e 3. |
| 9 | Corpo de texto do NOTE REPEAT | Menor que GROUP, AUTO e LOCK | Mesmo corpo dos vizinhos | Baixa | Manter o corpo padrão e apertar o entreletras. Carregado. |
| 10 | Sub-rótulo do REC | "Count In" | "Count-In", com hífen | Baixa | Acrescentar o hífen. Carregado. Custo zero. |
| 11 | SHIFT | Texto solto igual aos vizinhos | Rótulo dentro de uma caixinha clara serigrafada | Baixa | Opcional. É o único rótulo com caixa na foto. |
| 12 | Sub-rótulo do SELECT | Sem sub-rótulo | Há uma marca abaixo de SELECT, **ilegível na foto** | Baixa | Não inventar. Perguntar ao Thryki. Carregado das rodadas 1, 2 e 3. |

### Fechado nesta rodada, não mexer mais (diferença abaixo de 1 pp)

Knob (0,4 pp), touch strip (0,7 pp), tamanho do pad (0,7 pp), vão horizontal dos pads (0,18 pp),
margem esquerda (0,3 pp), tela (0,1 pp), fileira PITCH (0,2 pp), altura da fileira de modo (0,55 pp),
largura da coluna central (0,25 pp), vão NOTES → coluna central (0,09 pp), altura da fileira
RESTART (0,94 pp), posição do logo (1,0 pp), aspecto do corpo (0,2%).

Não contam como diferença, por serem intencionais: MIKRODECK no lugar de MASCHINE, nomes e cores
configurados nos pads (só o pad 13 preenchido nesta captura), página na telinha, PLAY/REC coloridos,
acabamento flat sem relevo.

## 5. Veredito

**O desenho está pronto para entregar num app. Pode fechar.**

Doze diferenças sobraram e nenhuma delas atrapalha reconhecer o aparelho, achar um controle ou usar
a interface. Onze são de 0,3 a 1,2 ponto percentual, ou seja, invisíveis sem régua, ou detalhes de
texto de dez segundos.

A única que um olho destreinado nota é o **item 1: os três botões redondos estão achatados** (razão
1,30 contra 1,03). Se você for encostar em uma coisa só, encoste nessa: largura de 37 para 30
unidades, sem tocar em altura nem em centro. Junto com o hífen do "Count-In" (item 10), isso leva o
desenho para ~9,6 e encerra o assunto.

O restante da tabela é polimento opcional. Não vale mais uma rodada de revisão: o histórico mostra
que cada ajuste fino abaixo de 1 pp tem chance real de passar do ponto e mexer no vizinho.
