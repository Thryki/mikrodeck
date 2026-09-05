# MikroDeck

Transforma o **Maschine Mikro MK3** da Native Instruments num Stream Deck:
16 pads com atalhos, cores por estado, páginas, touch strip, botões físicos e
texto na tela do aparelho.

Windows. Feito por Thryki.

---

## Antes de tudo: destravar o aparelho (uma vez só)

O Mikro MK3 sai de fábrica ignorando comandos de LED. Para liberar:

1. Conecte o Mikro no computador.
2. Abra o **Maschine 2 como administrador**, uma vez.
3. Pode fechar.

Pronto. Isso grava um estado permanente no aparelho. Ele sobrevive a
desconectar o cabo e a reiniciar o computador, e depois disso o MikroDeck
funciona sozinho, sem nada da Native Instruments rodando.

Sem esse passo os pads não acendem. A leitura de pads e botões funciona
mesmo assim.

## Instalar

Rode `MikroDeck_0.1.0_x64-setup.exe` (ou o `.msi`). O programa aparece na
bandeja do sistema, ao lado do relógio.

- **Fechar a janela não encerra o MikroDeck.** Ele continua na bandeja, que é
  onde ele precisa estar para os pads funcionarem.
- Para sair de verdade: botão direito no ícone da bandeja, **Sair**.
- Para subir junto com o Windows: Configurações → **Ao ligar o computador**.

## O básico

O desenho do aparelho na tela é clicável. Clique num pad ou botão e o painel
da direita abre com:

- **Nome** — o que aparece na tela do aparelho quando você encosta no pad.
- **Ação** — o que ele faz.
- **Cores** — em repouso, ao apertar e, para programas, quando o programa está
  aberto.
- **Brilho** — geral ou só daquele pad.

O que você aperta no aparelho acende na tela também, ao vivo.

### Páginas

Cada página tem seus 16 pads e seus botões. As setas `<` e `>` do aparelho
trocam de página, e dá para mudar quais botões fazem isso em Configurações.

Botão direito na aba da página para renomear, duplicar ou apagar.

**+ Página pronta** adiciona uma página já montada (Spotify, Claude, Casa,
Trabalho) no fim da lista, sem mexer no que você já tem.

### Os três botões da coluna da esquerda

Vêm com função de fábrica e ficam sempre acesos fraco, para você achar sem
manual. Todos podem ser trocados: basta programar o botão numa página.

| Botão | O que faz |
| --- | --- |
| Círculo | Liga e desliga o MikroDeck sem fechar o programa |
| Estrela | Vai para a primeira página |
| Lupa | Abre a busca do Windows |

Desligado, o aparelho apaga tudo e volta a ser um Maschine comum. Só o botão
redondo continua aceso, para você conseguir voltar.

### Touch strip

Em Configurações você escolhe o que ela controla: volume do Windows, brilho
dos pads, ou escolher a página pela posição do dedo.

### Tela do aparelho

Mostra o nome da página em corpo grande. Encostar num pad mostra o nome dele.
Mexer no volume mostra a barra. Parado por um tempo, entra o descanso: um texto
que você escolhe correndo na tela.

## Ações disponíveis

| Ação | Para quê |
| --- | --- |
| Abrir programa | Executável, atalho do menu iniciar, qualquer coisa |
| Abrir link | Abre no navegador padrão |
| Comando | Uma linha de shell |
| Atalho | `ctrl+shift+n`, `win+l`, o que você quiser |
| Mídia | Tocar, pausar, pular faixa, volume, mudo |
| Trocar de página | Próxima, anterior ou uma página específica |
| Ligar e desligar | Pausa o MikroDeck |
| Home Assistant | Chama um serviço da sua casa |
| Requisição HTTP | Webhook ou qualquer serviço que aceite uma chamada |

### Home Assistant

Em Configurações, preencha o endereço do servidor e um token de acesso de longa
duração (fim da página do seu perfil no Home Assistant). Depois é só escolher a
ação **Home Assistant** num pad e dizer o serviço (`light.toggle`) e a entidade
(`light.sala`).

O token fica gravado em texto puro em `%USERPROFILE%\.mikrodeck\config.json`.

## Configurar por conversa (MCP)

O MikroDeck vem com um servidor MCP. Ligue no Claude Code e peça os atalhos em
português; a mudança entra no aparelho na hora, sem reiniciar nada.

Em Configurações → **Configurar por conversa (MCP)** tem o comando pronto para
copiar. Ele tem esta cara:

```bash
claude mcp add mikrodeck -- "C:\Program Files\MikroDeck\mikrodeck-mcp.exe"
```

Depois, na conversa: *"cria uma página chamada Edição com os atalhos do
Photoshop que eu mais uso"*.

## Onde ficam as coisas

| O quê | Onde |
| --- | --- |
| Configuração | `%USERPROFILE%\.mikrodeck\config.json` |
| Servidor MCP | ao lado do executável instalado |

O arquivo de configuração pode ser editado à mão. O MikroDeck percebe a
mudança em até um segundo e aplica sozinho.

## Se der problema

**Os pads não acendem.** Faça o passo do Maschine 2 como administrador,
lá em cima.

**O aparelho não aparece.** Configurações → Diagnóstico diz se ele foi
encontrado. Desconecte e conecte o cabo: o MikroDeck reconecta sozinho.

**Quero começar de novo.** Configurações → Restaurar configuração de exemplo.

## Para quem quer mexer no código

- `motor/` — o núcleo em Rust: HID, estado, ações, tela, vigias.
- `app/` — o aplicativo Tauri e a interface em React.
- `mcp/` — o servidor MCP.
- `docs/maschine-mikro-mk3-hid-protocol.md` — o protocolo HID do aparelho,
  em inglês, para quem quiser escrever o próprio software.

```bash
cd mikrodeck/motor && cargo test
```

O aparelho **não fala MIDI pela USB**. Ele é HID puro; o "modo MIDI" é emulado
pelo driver da Native Instruments. Falar HID direto dá acesso a tudo, sem
driver nenhum. Os detalhes estão no documento de protocolo.
