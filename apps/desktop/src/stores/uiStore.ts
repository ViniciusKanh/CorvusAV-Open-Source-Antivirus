import { create } from 'zustand'
import type { SectionId } from '../types/navigation'

interface UiState {
  activeSection: SectionId
  setActiveSection: (section: SectionId) => void
}

export const useUiStore = create<UiState>((set) => ({
  // "Verificação" inicia selecionada, conforme a especificação da
  // Central de Verificação.
  activeSection: 'scanning',
  setActiveSection: (section) => set({ activeSection: section }),
}))
