# Política de Segurança

## Reportando uma vulnerabilidade

O CorvusAV ainda não teve nenhuma versão pública distribuída (está na fase
de fundação, v0.1.0, não lançada). Enquanto o projeto não tiver um canal
dedicado, relate vulnerabilidades de forma responsável abrindo uma issue
privada/security advisory no repositório GitHub do projeto assim que ele
for publicado, ou entre em contato diretamente com o mantenedor do
projeto. Não abra uma issue pública detalhando uma vulnerabilidade
explorável antes que ela seja corrigida.

Inclua, quando possível:
- Versão/commit afetado.
- Passos para reproduzir.
- Impacto esperado (ex.: execução de código, escalonamento de privilégio,
  bypass de detecção).

## Escopo

Nesta fase (v0.1.0), o CorvusAV roda inteiramente no contexto do usuário,
sem serviço privilegiado e sem monitoramento contínuo. Vulnerabilidades de
maior interesse nesta fase:
- Execução de comando/injeção via caminhos de arquivo ou argumentos
  passados ao `clamscan`.
- Condições de corrida entre validação e uso de caminhos
  (TOCTOU) na verificação personalizada.
- Qualquer caminho em que um arquivo malicioso examinado possa ser
  executado ou interpretado pelo CorvusAV.

## Versões suportadas

Nenhuma versão foi lançada publicamente ainda. Esta política será
atualizada com uma tabela de versões suportadas a partir do primeiro
lançamento.
