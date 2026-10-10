# Contexto para assistentes de IA (Claude) neste repositório

Este projeto segue obrigatoriamente a skill `corvusav-security-engineer`
(`.claude/skills/corvusav-security-engineer/SKILL.md`). Leia-a antes de
qualquer alteração estrutural, de segurança ou de arquitetura.

## Resumo do projeto

CorvusAV: antivírus gratuito, open source, para Windows 10/11. Stack:
Rust (núcleo + Tauri backend), React/TypeScript/Tailwind (frontend),
SQLite (persistência), ClamAV via `clamscan` externo (detecção), YARA-X e
ML planejados para fases futuras. Ver `docs/roadmap/README.md` para o
estado real de cada fase.

## Regras de autonomia (resumo — a skill é a fonte de verdade)

- Preserve arquivos preexistentes; nunca sobrescreva mudanças do usuário.
- Nunca execute amostras de malware; EICAR oficial e fixtures sintéticas
  apenas.
- Nunca desative o Microsoft Defender, Firewall, UAC ou SmartScreen.
- Nunca declare uma função como concluída sem testes/compilação reais
  (não "invente" resultados de comandos).
- Um arquivo com erro de leitura/execução do mecanismo nunca é
  classificado como limpo.
- Mudanças de alto risco (instalação global, operação privilegiada, risco
  de sobrescrever arquivos existentes): apresente diagnóstico e peça
  autorização antes de agir.

## Estado desta entrega (v0.1.0 — Fase 0 do roadmap)

Ver `README.md` (seção "Status de validação desta entrega") e
`docs/development/README.md`. Em resumo: o frontend foi compilado e
testado; o backend Rust foi escrito mas **ainda não compilado** — trate
qualquer erro de compilação reportado pelo usuário como prioridade alta,
já que é o primeiro `cargo build` real deste código.

## Onde cada coisa vive

- Contratos de domínio: `crates/corvus-core/src/*.rs`.
- Persistência: `crates/corvus-storage/src/database.rs` (métodos) e
  `crates/corvus-storage/migrations/*.sql` (esquema).
- Adaptador ClamAV: `crates/corvus-clamav/src/{detect,adapter}.rs`.
- Políticas (alvos de verificação, classificação): `crates/corvus-policy/src/*.rs`.
- Orquestração: `crates/corvus-scanner/src/service.rs`.
- Comandos Tauri: `apps/desktop/src-tauri/src/commands.rs`.
- Central de Verificação: `apps/desktop/src/features/scanning/`.
- Tipos TypeScript espelhando o backend: `apps/desktop/src/types/corvus.ts`
  (mantenha em sincronia com `corvus-core` e `corvus-scanner::events`).
