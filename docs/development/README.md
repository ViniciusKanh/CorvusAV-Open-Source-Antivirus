# Ambiente de desenvolvimento

## Pré-requisitos (Windows 10/11 x64)

- [Rust](https://rustup.rs/) (edição 2021) com `cargo`.
- [Node.js](https://nodejs.org/) 18 ou superior, com `npm`.
- [ClamAV para Windows](https://www.clamav.net/downloads) — instale e
  garanta que `clamscan.exe` esteja no `PATH`, ou defina a variável de
  ambiente `CORVUSAV_CLAMAV_PATH` apontando diretamente para o executável.
- Microsoft Edge WebView2 Runtime (já vem instalado por padrão na maioria
  das instalações atualizadas do Windows 10/11).
- Para gerar o instalador Windows (`npm run tauri build`): as
  [pré-condições de build do Tauri para Windows](https://tauri.app/start/prerequisites/)
  (Microsoft C++ Build Tools / Visual Studio).

## Comandos principais

```powershell
# Instalar dependências do frontend
cd C:\dev\CorvusAV\apps\desktop
npm install

# Rodar em modo desenvolvimento (abre a janela do CorvusAV)
npm run tauri dev

# Build de produção (gera o .exe e, se os pré-requisitos estiverem
# presentes, o instalador NSIS)
npm run tauri build
```

## Testes

```powershell
# Workspace Rust (na raiz do repositório)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Frontend (em apps/desktop)
npm run build
npm run test
```

## Banco de dados local

O SQLite do CorvusAV fica em `%LocalAppData%\CorvusAV\data\corvusav.db`
nesta versão (ver `docs/architecture/0002-diretorio-de-dados.md`). Para
inspecionar: `sqlite3 "%LocalAppData%\CorvusAV\data\corvusav.db"` (CLI do
SQLite) ou qualquer visualizador de SQLite.

## Ambiente desta sessão (como esta primeira entrega foi construída e validada)

Esta implementação inicial foi criada por uma sessão de IA sem acesso ao
Windows real do projeto. O acesso ao repositório (`C:\dev\CorvusAV`) se deu
por uma ponte que monta a pasta dentro de uma VM Linux (Ubuntu 22.04)
separada da máquina Windows real. Isso teve consequências diretas na
validação possível:

- **Frontend (TypeScript/React/Vite/Tailwind)**: totalmente instalável e
  compilável nessa VM Linux (é código multiplataforma). Foi executado
  `npm install`, `npm run build` (compila com sucesso, `tsc -b && vite
  build`) e `npm run test` (14 testes do Vitest, todos aprovados) de fato
  nesta sessão.
- **Rust (workspace Cargo com 5 crates + backend Tauri)**: a VM Linux desta
  sessão **não tinha `rustc`/`cargo` instalados, e não há acesso de rede a
  `crates.io` nem a `sh.rustup.rs`** (bloqueados pelo proxy do ambiente).
  Não foi possível instalar o Rust nem compilar/testar o código Rust nesta
  sessão. Todo o código Rust foi escrito com cuidado deliberado (tipos
  explícitos, tratamento de erro, assinaturas de API conferidas contra o
  conhecimento do autor sobre `tauri`, `tokio`, `rusqlite`), mas **ainda não
  foi compilado nem uma única vez**.
- **ClamAV**: não está instalado na VM desta sessão; a lógica de detecção e
  execução (`corvus-clamav`) foi escrita e coberta por testes unitários que
  não dependem de um `clamscan` real (testes de parsing de linha e de
  versão), mas a execução ponta a ponta contra um `clamscan.exe` real nunca
  foi exercitada.
- **Tauri (`npm run tauri dev` / `npm run tauri build`)**: não pôde ser
  executado — exige o toolchain Rust (ausente aqui) e, para a janela
  abrir de fato, um ambiente Windows real com WebView2.

**Próximo passo obrigatório**: antes de considerar esta fundação validada,
rode no Windows real do projeto, nesta ordem:

```powershell
cd C:\dev\CorvusAV
cargo build --workspace
cargo test --workspace
cd apps\desktop
npm install
npm run tauri dev
```

Qualquer erro de compilação ou teste que aparecer deve ser reportado (cole
a mensagem de erro) para correção dirigida — é esperado que ajustes pontuais
sejam necessários na primeira compilação real em Rust/Tauri, especialmente
em torno de nomes de API do `tauri-plugin-dialog` (a integração do diálogo
de seleção de pastas em `pick_custom_scan_targets` é o ponto de maior risco,
por depender de uma API de callback cuja assinatura pode variar entre
versões do plugin).
