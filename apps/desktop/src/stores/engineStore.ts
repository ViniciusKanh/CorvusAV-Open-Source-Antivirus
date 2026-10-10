import { create } from 'zustand'
import type { EngineHealth } from '../types/corvus'

interface EngineState {
  health: EngineHealth | null
  loading: boolean
  error: string | null
  setHealth: (health: EngineHealth) => void
  setLoading: (loading: boolean) => void
  setError: (error: string | null) => void
}

export const useEngineStore = create<EngineState>((set) => ({
  health: null,
  loading: false,
  error: null,
  setHealth: (health) => set({ health, error: null }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}))
