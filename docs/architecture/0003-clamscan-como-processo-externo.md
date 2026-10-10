# ADR 0003 — ClamAV via `clamscan` externo (não `libclamav` embarcado)

## Contexto
O CorvusAV precisa de um mecanismo de detecção real já na primeira versão.
Duas opções: vincular `libclamav` diretamente (FFI) ou invocar o executável
`clamscan` como processo externo.

## Decisão
Usar `clamscan` como processo externo, isolado, sem shell (argumentos
passados como lista para `std::process::Command`/`tokio::process::Command`,
nunca concatenados em uma string interpretada por um shell).

## Justificativa
- Isolamento de falhas: uma falha ou memory-safety issue dentro do
  `libclamav` não derruba o processo do CorvusAV.
- Simplicidade: evita bindings FFI complexos e a necessidade de vincular
  estaticamente/dinamicamente contra uma biblioteca C na primeira versão.
- Facilita cancelamento (matar o processo) e limite de tempo
  (`tokio::time::timeout`/`select!` contra um `sleep`).

## Consequências
- Overhead de iniciar um processo por verificação (aceitável para
  verificações sob demanda; seria revisitado para monitoramento contínuo
  de alta frequência).
- Dependência de parsing da saída textual do `clamscan --verbose` — ver
  `corvus_clamav::adapter::parse_clamscan_line`, com testes unitários
  cobrindo os formatos `OK`, `FOUND` e `ERROR`.
- Decisão reavaliável: uma integração via `libclamav`/FFI pode ser
  considerada em fase posterior caso o overhead de processo se mostre
  relevante para o monitoramento contínuo (Fase 4).
