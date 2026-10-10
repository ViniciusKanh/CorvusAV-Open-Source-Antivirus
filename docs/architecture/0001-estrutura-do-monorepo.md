# ADR 0001 — Estrutura do monorepo

## Contexto
O CorvusAV precisa de um núcleo Rust reutilizável (futuramente também por
um serviço Windows privilegiado), um app desktop Tauri, e documentação e
scripts auxiliares, tudo versionado e evoluindo junto.

## Decisão
Monorepo único em `C:\dev\CorvusAV`, com workspace Cargo na raiz contendo:

- `crates/corvus-core` — domínio (tipos, estados, erros), sem dependências
  de infraestrutura.
- `crates/corvus-storage` — persistência SQLite.
- `crates/corvus-clamav` — adaptador do mecanismo ClamAV.
- `crates/corvus-policy` — regras de decisão e políticas de alvo.
- `crates/corvus-scanner` — orquestração das tarefas de verificação.
- `apps/desktop` — app Tauri 2 + React/TypeScript/Tailwind, com o backend
  Rust do app em `apps/desktop/src-tauri` (também membro do workspace
  Cargo).

Apenas os diretórios necessários a cada etapa são criados; o restante da
estrutura prevista (`corvus-service`, `corvus-ipc`, `corvus-quarantine`,
`corvus-yara`, `installer/wix`, etc.) está documentado no roadmap, não
criado como pastas vazias.

## Consequências
- Um único `cargo build --workspace` compila todo o backend.
- Dependências entre crates são explícitas via `path` no
  `[workspace.dependencies]` da raiz, facilitando refatoração.
- O app desktop pode evoluir para consumir um futuro serviço Windows sem
  precisar mudar a localização dos crates de domínio.
