# ADR 0002 — Diretório de dados na v0.1.0

## Contexto
O CorvusAV precisa de um diretório de dados para o banco SQLite, fora dos
recursos somente leitura do executável instalado. O documento de
referência da skill do projeto recomenda `%ProgramData%\CorvusAV` com ACLs
geridas por um serviço Windows privilegiado — que ainda não existe nesta
versão.

## Decisão
Na v0.1.0, sem serviço privilegiado, os dados ficam em
`%LocalAppData%\CorvusAV\data\corvusav.db` (resolvido por
`corvus_storage::paths::resolve_data_dir`, via o crate `dirs`), no
contexto do usuário que executa o aplicativo.

## Consequências
- Evita escrever em `%ProgramData%` (diretório compartilhado do sistema)
  sem um serviço responsável por suas ACLs — alinhado ao princípio de
  menor privilégio.
- Cada usuário do Windows tem seu próprio banco/histórico nesta versão.
- **Migração futura**: ao introduzir o serviço Windows privilegiado
  (Fase 3 do roadmap), o estado compartilhado (histórico, quarentena,
  políticas) deve migrar para `%ProgramData%\CorvusAV` com ACLs explícitas
  por função, incluindo uma rotina de migração dos dados existentes em
  `%LocalAppData%`.
