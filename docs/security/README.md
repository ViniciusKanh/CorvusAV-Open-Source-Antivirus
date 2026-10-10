# Modelo de ameaças — CorvusAV v0.1.0

Este documento resume o modelo de ameaças da primeira versão. Será
expandido a cada fase do roadmap, em especial ao introduzir o serviço
Windows privilegiado (Fase 3) e a quarentena (Fase 2).

## Superfície de ataque atual

- **Arquivos examinados**: tratados como não confiáveis. O CorvusAV nunca
  executa um arquivo examinado; a varredura é delegada inteiramente ao
  `clamscan`, invocado via `std::process::Command`/`tokio::process::Command`
  sem shell (sem interpolação de string, argumentos passados como lista).
- **Caminhos de entrada**: a verificação personalizada usa o diálogo nativo
  do sistema operacional (via `tauri-plugin-dialog`); os caminhos retornados
  são validados (`corvus-policy::validate_custom_targets`) quanto a
  existência/acessibilidade e a um limite de itens antes de serem usados.
- **IPC**: nesta versão, a única camada de IPC é a ponte padrão do Tauri
  entre a webview e o processo Rust (comandos `#[tauri::command]`), que já
  roda no mesmo processo e no contexto do usuário — não há ainda um serviço
  privilegiado separado nem Named Pipes. Isso será revisitado na Fase 3.
- **Processo `clamscan`**: executado no contexto do usuário, sem privilégios
  administrativos. Timeout de 30 minutos por verificação; cancelamento mata
  o processo (`kill_on_drop`/`start_kill`). A saída é interpretada de forma
  estrita (`parse_clamscan_line`): linhas não reconhecidas são descartadas
  em vez de interpretadas de forma otimista.

## Decisões relevantes de segurança já aplicadas

- Um arquivo com erro de leitura/execução do mecanismo nunca é classificado
  como `clean` (`Verdict::Error`, nunca rebaixado — ver
  `corvus-core::result::Verdict::combine`).
- O CorvusAV nunca desativa o Microsoft Defender, nem declara
  "Sistema protegido" sem antes validar a disponibilidade real do motor.
- Nenhum segredo ou credencial é persistido; o banco SQLite não armazena o
  conteúdo de arquivos examinados, apenas metadados (caminho, veredito,
  hash quando disponível, evidências de detecção).
- `.gitignore` exclui explicitamente artefatos de build, bancos locais e
  uma eventual pasta de amostras reais de malware — nenhuma amostra real é
  incluída no repositório; os únicos arquivos de teste previstos são o
  EICAR oficial e fixtures sintéticas benignas (ainda não adicionadas nesta
  entrega).

## Limitações conhecidas (não são bugs, são escopo)

- Sem quarentena: nenhum arquivo é movido ou excluído automaticamente.
- Sem monitoramento em tempo real: a proteção é inteiramente sob demanda.
- Sem verificação completa: apenas rápida e personalizada estão disponíveis.
- Sem assinatura/checagem de integridade do próprio instalador ainda
  (depende de infraestrutura de assinatura de código, fora do escopo do
  MVP).
- Sem registro como provedor de antivírus do Windows Security Center.

## Reportando vulnerabilidades

Ver `SECURITY.md` na raiz do repositório.
