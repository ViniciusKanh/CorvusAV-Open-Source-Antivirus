import { create } from 'zustand'
import type { ScanJob, ScanResult } from '../types/corvus'

const MAX_RECENT_RESULTS = 25

interface ScanState {
  activeJob: ScanJob | null
  recentResults: ScanResult[]
  history: ScanJob[]
  historyLoading: boolean
  setActiveJob: (job: ScanJob | null) => void
  pushResult: (result: ScanResult) => void
  setHistory: (jobs: ScanJob[]) => void
  setHistoryLoading: (loading: boolean) => void
}

export const useScanStore = create<ScanState>((set, get) => ({
  activeJob: null,
  recentResults: [],
  history: [],
  historyLoading: false,
  setActiveJob: (job) => set({ activeJob: job }),
  pushResult: (result) =>
    set({ recentResults: [result, ...get().recentResults].slice(0, MAX_RECENT_RESULTS) }),
  setHistory: (jobs) => set({ history: jobs }),
  setHistoryLoading: (loading) => set({ historyLoading: loading }),
}))
