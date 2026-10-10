# Changelog

Este projeto documenta mudanças notáveis neste arquivo. Formato baseado em
[Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).

## [0.1.0] — Fundação do projeto (não lançado)

### Adicionado
- Estrutura de monorepo (workspace Cargo + app Tauri 2).
- Crates de domínio: `corvus-core`, `corvus-storage`, `corvus-clamav`,
  `corvus-policy`, `corvus-scanner`.
- Backend Tauri com comandos para saúde do mecanismo, início/cancelamento
  de verificação, histórico e resultados, e seletor nativo de pastas.
- Interface da Central de Verificação (painel de estado do mecanismo,
  cartões de verificação, progresso em tempo real, indicadores
  secundários e histórico).
- Persistência SQLite real (migrations, histórico de verificações).
- Identidade visual: paleta, tipografia (Exo 2 / Inter) e logotipo
  vetorial original.
- Documentação inicial (arquitetura, segurança, desenvolvimento, roadmap).
- CI (GitHub Actions) para lint/testes de Rust e frontend.

### Conhecido como pendente
- Backend Rust ainda não compilado em ambiente real (ver
  `docs/development/README.md`).
- Verificação completa, quarentena, monitoramento contínuo, atualização
  automática de assinaturas, ML e serviço Windows privilegiado — ver
  `docs/roadmap/README.md`.
