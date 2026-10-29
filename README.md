# VidManager

Gerenciador de downloads de vídeo para Windows, no estilo Internet Download Manager, com foco em
baixar vídeos de sites como YouTube, Vimeo, Twitch e milhares de outros suportados pelo
[yt-dlp](https://github.com/yt-dlp/yt-dlp) — mais **Threads**, que tem suporte próprio (o yt-dlp
não extrai de lá).

Construído com **Tauri 2 + React + TypeScript + Rust**.

---

## Recursos

- **Baixar com um Enter**: cole o link e o download começa na melhor qualidade compatível (H.264/AAC em MP4 quando disponível).
- **Escolher formato**: painel com resoluções disponíveis, fps, tamanho estimado, opção "máxima qualidade" e "só áudio".
- **Playlists e canais**: baixa a playlist inteira ou um intervalo (`1-10`, `5,8,20`).
- **Threads**: cola o link do post (inclusive `/share/...`) e o app baixa o vídeo ou todos os vídeos de um carrossel. Suporte nativo, sem depender do yt-dlp.
- **Áudio**: extração para MP3, M4A, Opus, FLAC ou WAV com capa embutida.
- **Legendas**: baixa e embute legendas em português/inglês quando existirem.
- **Fila e concorrência**: até 6 downloads simultâneos, limite de velocidade, pausa, retomada, cancelamento e repetição. Cancelar também apaga o arquivo parcial; pausar preserva para retomar do mesmo ponto.
- **Histórico persistente**: SQLite local, sobrevive a fechar o app; downloads interrompidos voltam como pausados.
- **Monitor de área de transferência**: sugere o download quando você copia um link.
- **Extensão Chrome/Edge (MV3)**: envia a página atual para o app com um clique, incluindo cookies para vídeos que exigem login.
- **API local autenticada**: `127.0.0.1:41300`, token de pareamento e CORS restrito.
- **Interface**: dark, PT-BR, barra de título própria, notificações nativas.

---

## Requisitos

| Item | Observação |
|---|---|
| Windows 10/11 | WebView2 já vem instalado na maioria dos PCs |
| Node 20+ | apenas para desenvolver/compilar |
| Rust (toolchain GNU ou MSVC) | este projeto compila com `x86_64-pc-windows-gnu` |
| yt-dlp | baixado automaticamente na primeira execução |
| Deno | runtime JavaScript do yt-dlp para o YouTube; baixado automaticamente na primeira execução |
| ffmpeg | detectado no `PATH`; se faltar, o app baixa em Configurações → Binários |

---

## Desenvolvimento

```powershell
cd C:\Users\spekv\vidmanager
npm install
npm run app:dev
```

O `npm run app:dev` sobe o Vite na porta 1420 e abre o app Tauri.

### Compilar o instalador

```powershell
npm run app:build
```

Saída em `src-tauri/target/release/bundle/nsis/VidManager_0.1.0_x64-setup.exe`.

> Se estiver usando o toolchain GNU, o linker pode emitir um aviso
> `.rsrc merge failure: multiple non-default manifests`. O app continua funcional; o aviso vem do
> `windres` ao mesclar recursos.

---

## Instalar a extensão (Chrome ou Edge)

1. Abra `chrome://extensions` ou `edge://extensions`.
2. Ative o **modo desenvolvedor**.
3. Clique em **Carregar sem compactação** e selecione a pasta `extension/`.
4. Abra o VidManager → **Configurações → Extensão e API local** e copie o código de 6 dígitos.
5. No popup da extensão, digite o código e clique em **Conectar**.

Depois disso, abra qualquer vídeo e clique no ícone do VidManager → **Baixar com VidManager**.
Também há a opção **Baixar com VidManager** no menu de contexto (clique direito).

> A extensão é instalada "sem compactação" porque a Chrome Web Store não aceita esse tipo de
> ferramenta. Ela não é publicada em loja.

---

## Onde ficam os arquivos

| O quê | Caminho |
|---|---|
| Downloads | `%USERPROFILE%\Downloads\VidManager\` (configurável) |
| Banco/histórico | `%APPDATA%\VidManager\vidmanager.db` |
| Binários | `%APPDATA%\VidManager\bin\` |
| Cookies temporários | `%APPDATA%\VidManager\cookies\` (apagados ao concluir) |

Template padrão de nome:

```
%(uploader,channel,creator|Outros)s/%(title).150B [%(id)s].%(ext)s
```

Quando o site não informa autor, a pasta vira `Outros` (antes virava `NA`).

---

## API local

Base: `http://127.0.0.1:41300`

| Método | Rota | Autenticação | Descrição |
|---|---|---|---|
| GET | `/ping` | — | health check |
| POST | `/pair` | código de 6 dígitos | troca código por token |
| POST | `/download` | header `X-VidManager-Token` | enfileira um download |

Exemplo:

```powershell
curl.exe -X POST http://127.0.0.1:41300/download `
  -H "Content-Type: application/json" `
  -H "X-VidManager-Token: SEU_TOKEN" `
  -d '{\"url\":\"https://www.youtube.com/watch?v=EXEMPLO\",\"preset\":\"best\"}'
```

A API só escuta em `127.0.0.1` e exige token em todas as rotas sensíveis.

---

## Threads

O yt-dlp não tem extrator de Threads (issue [#7523](https://github.com/yt-dlp/yt-dlp/issues/7523),
aberta desde 2024), então o app faz a extração por conta própria, em `src-tauri/src/threads.rs`:

- aceita `threads.com` e `threads.net`, inclusive links curtos `/share/...` (o redirecionamento é seguido);
- lê o post com User-Agent de crawler — o Meta entrega o HTML completo, com as URLs do CDN, sem login;
- carrossel com vários vídeos vira um download por vídeo, nomeados `autor/legenda [id] (2 de 9).mp4`;
- vale para **posts públicos**: post privado, de conta protegida ou removido retorna erro claro;
- "Escolher formato" lista o post, mas a qualidade é a que o Threads publica (um MP4 já com áudio);
- as URLs do CDN expiram: se um download antigo falhar, use **Repetir** — o app resolve o post de novo.

---

## Solução de problemas

| Sintoma | O que fazer |
|---|---|
| "yt-dlp não encontrado" | Configurações → Binários → Instalar |
| Download falha com erro de extração | Atualize o yt-dlp (Configurações → Binários → Atualizar) |
| Faltam resoluções no YouTube | Instale o runtime JavaScript em Configurações → Binários (Deno) |
| Vídeo pede login/idade | Configurações → Cookies do navegador (Chrome/Edge/Firefox) ou use a extensão, que envia cookies |
| ffmpeg ausente | Configurações → Binários → Instalar (baixa ~80 MB e extrai só o executável) |
| Extensão não conecta | App aberto? API local ativa? Porta igual nos dois lados? Código expirado (gere outro) |
| Threads: "não tem vídeo" | O post é só texto/imagem, ou é privado. Confira abrindo o link deslogado |
| Threads: erro em download antigo | As URLs do CDN expiram; clique em Repetir para resolver o post de novo |
| Antivírus/SmartScreen | O instalador não é assinado digitalmente; use "Mais informações → Executar assim mesmo" |

---

## Estrutura do projeto

```
vidmanager/
├─ src/                     # React (telas, componentes, store, estilos)
├─ src-tauri/
│  ├─ src/
│  │  ├─ lib.rs             # bootstrap, fila, clipboard, eventos
│  │  ├─ ytdlp.rs           # probe, args do yt-dlp, progresso, erros
│  │  ├─ threads.rs         # extrator próprio de posts do Threads
│  │  ├─ api.rs             # axum: /ping /pair /download
│  │  ├─ commands.rs        # comandos IPC
│  │  ├─ binaries.rs        # yt-dlp/ffmpeg: detectar, instalar, atualizar
│  │  ├─ db.rs              # SQLite
│  │  └─ state.rs           # estado compartilhado
│  └─ tauri.conf.json
├─ extension/               # extensão MV3 (Chrome/Edge)
└─ scripts/                 # geração de ícone
```

---

## Aviso legal

O VidManager é uma ferramenta pessoal. Baixar conteúdo protegido por direitos autorais sem
autorização pode violar os termos de uso dos sites e a legislação aplicável. Use com conteúdo
próprio, de domínio público, Creative Commons ou que você tenha direito de baixar. Não há suporte a
DRM ou a serviços de streaming por assinatura.
