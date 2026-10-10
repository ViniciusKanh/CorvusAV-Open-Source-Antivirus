# Como contribuir

Obrigado por considerar contribuir com o CorvusAV. O projeto está em fase
inicial (v0.1.0) — veja `docs/roadmap/README.md` para o que já existe e o
que está planejado antes de propor uma funcionalidade nova.

## Antes de abrir um Pull Request

1. Leia `.claude/skills/corvusav-security-engineer/SKILL.md` — ele define
   os princípios de arquitetura e segurança do projeto.
2. Rode localmente, no Windows:
   ```powershell
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
   e, em `apps/desktop`:
   ```powershell
   npm run build
   npm run test
   ```
3. Não inclua segredos, credenciais, amostras reais de malware, ou
   artefatos de build (`target/`, `node_modules/`, `dist/`) no commit.
4. Verifique `git diff --check` antes de enviar.

## Estilo de código

- **Rust**: `rustfmt` padrão, `clippy` sem warnings, evite `unwrap`/`expect`
  em caminhos operacionais (fora de testes), erros explícitos via
  `thiserror`. Identificadores em inglês; comentários/documentação em
  português do Brasil.
- **TypeScript/React**: modo `strict`, componentes funcionais, sem `any`
  não justificado, estados de carregamento/erro/vazio tratados
  explicitamente.
- **SQL**: migrations incrementais, sempre com parâmetros (nunca
  concatenação de string).

## Commits e licenciamento

A estratégia de licenciamento do projeto ainda está em definição (ver
`README.md`). Ao contribuir, você concorda que sua contribuição será
licenciada sob os termos que vierem a ser adotados pelo projeto.
