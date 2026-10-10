// Evita a janela de console extra em builds de release no Windows,
// mantendo-a em builds de depuração para facilitar o diagnóstico.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    corvusav_desktop_lib::run();
}
