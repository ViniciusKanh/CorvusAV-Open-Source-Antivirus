# Roadmap do CorvusAV

Uma fase prioritária ativa por vez, salvo dependência justificada. Cada
fase só é considerada concluída quando seu "gate" é verificado de fato
(compilação real, testes executados, validação manual em VM quando
indicado) — nunca por inspeção visual da interface isoladamente.

| Fase | Entrega | Gate de conclusão |
|---|---|---|
| **0 — Fundação** (esta entrega) | Ambiente Windows, workspace Cargo, scaffold Tauri/React, CI, padrões, docs | App inicia, crates compilam, lint/testes básicos passam, sem funcionalidade falsa |
| **1 — Scanner sob demanda** | Adaptador ClamAV, varredura de arquivo/pasta, cancelamento, SQLite | EICAR detectado pelo mecanismo instalado; arquivos limpos e erros diferenciados; registro persistido |
| **2 — Quarentena** | Isolamento supervisionado, ACLs, auditoria, restauração | Testes de permissão, colisão, falha parcial e restauração passam em VM |
| **3 — UI + IPC privilegiado** | Named Pipes seguros, serviço Windows, fila, progresso, resultados e histórico centralizados | Usuário comum opera sem elevação; comandos privilegiados autorizados e auditados |
| **4 — Monitoramento contínuo** | Observação user-mode, agendamento, FreshClam e atualização de base | Eventos perdidos tratados; scanner recupera após falhas; sem prometer bloqueio pré-execução |
| **5 — Detecção complementar** | YARA-X, heurísticas, avaliação estatística | Regras revisadas; falsos positivos e tempos medidos; decisões explicáveis |
| **6 — Beta distribuível** | Hardening, empacotamento, instalação/desinstalação, SBOM, release assinado conforme viabilidade | VM limpa, upgrade e rollback testados; riscos e limitações publicados |
| **Posterior — Pesquisa/Kernel** | ML embarcado validado; estudo independente de minifilter | Aprovação científica, legal e operacional antes de qualquer proteção em kernel |

## Estado real ao final da Fase 0 (v0.1.0)

Implementado e com algum grau de verificação nesta sessão:

- Estrutura de monorepo (Cargo workspace + app Tauri) conforme o plano.
- Contratos de domínio (`corvus-core`) com testes unitários (escritos; não
  executados nesta sessão — ver `docs/development/ambiente-desta-sessao.md`).
- Persistência SQLite (`corvus-storage`) com migrations e testes unitários
  (idem — escritos, não executados aqui).
- Adaptador ClamAV via `clamscan` como processo externo (`corvus-clamav`).
- Políticas de alvo de verificação e classificação de resultados
  (`corvus-policy`).
- Orquestração de tarefas com cancelamento cooperativo (`corvus-scanner`).
- Backend Tauri 2 com comandos e emissão de eventos de progresso.
- Interface React/TypeScript/Tailwind da Central de Verificação,
  **compilada e testada nesta sessão** (`npm run build`, `npm run test`
  — 14 testes aprovados).
- Identidade visual (paleta, tipografia, logotipo vetorial original,
  ícones do aplicativo).

Explicitamente **fora do escopo desta entrega** (ver `README.md` e
`docs/security/README.md`):

- Verificação completa (todas as unidades) — a interface informa a
  indisponibilidade em vez de simular a função.
- Quarentena, monitoramento contínuo, atualização automática de
  assinaturas, heurística/ML, serviço Windows privilegiado, driver de
  kernel.
- Cálculo de hash de arquivo nos resultados (`file_hash` fica `None` por
  ora) e progresso percentual (não há total de arquivos conhecido
  antecipadamente com o `clamscan`).
