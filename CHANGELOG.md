# Changelog

Este projeto documenta mudanças notáveis neste arquivo. Formato baseado em
[Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).

## [0.1.0] — Fundação do projeto (não lançado)

### Adicionado
- Estrutura de monorepo (workspace Cargo + app Tauri 2).
- Crates: `corvus-core`, `corvus-storage`, `corvus-clamav`, `corvus-policy`,
  `corvus-scanner`, `corvus-quarantine`.
- Backend Tauri com comandos para saúde do mecanismo, início/cancelamento
  de verificação, histórico e resultados, quarentena, e seletor nativo de
  pastas.
- Interface da Central de Verificação (painel de estado do mecanismo,
  cartões de verificação, progresso em tempo real, indicadores
  secundários e histórico) e da Quarentena.
- Persistência SQLite real (migrations, histórico de verificações, itens
  de quarentena).
- Quarentena segura: isolamento com ACLs restritas, nomes internos
  aleatórios, auditoria e restauração autenticada — **compilado com
  sucesso em ambiente Windows real**.
- Mecanismo de verificação dual `clamscan`/`clamd` (`ScanEngineAdapter`),
  com `clamd` opcional para varredura multi-thread e fallback automático
  para `clamscan` — **compilado**; comportamento real em uso (detecção e
  uso efetivo de `clamd`, ganho de desempenho) ainda não confirmado.
- Detecção e exclusão de arquivos "só na nuvem" do OneDrive (Arquivos sob
  Demanda) durante a verificação via `clamscan`, para não travar a
  varredura forçando downloads — implementado nesta sessão, **ainda não
  compilado nem testado**.
- Identidade visual: paleta, tipografia (Exo 2 / Inter) e logotipo
  vetorial original.
- Documentação inicial (arquitetura, segurança, desenvolvimento, roadmap).
- CI (GitHub Actions) para lint/testes de Rust e frontend.

### Conhecido como pendente
- Mecanismo dual `clamscan`/`clamd` e a exclusão de arquivos do OneDrive:
  ver limitações de validação acima — nenhum dos dois foi confirmado em
  uso real ainda.
- A exclusão de arquivos do OneDrive só cobre o caminho `clamscan`; o
  caminho `clamd` ainda pode travar no mesmo cenário.
- Verificação completa, monitoramento contínuo, atualização automática de
  assinaturas, ML e serviço Windows privilegiado — ver
  `docs/roadmap/README.md`.
