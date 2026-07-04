import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

vi.mock('../api/client', () => ({
  api: {
    getGenerations: vi.fn().mockResolvedValue({
      count: 1,
      generations: [{ id: 'gen_1', tag: 'v1.0.0', category: 'release', project_id: 'p1', context_notes: null, created_at: 0 }],
    }),
    getGeneration: vi.fn().mockResolvedValue({
      generation: { id: 'gen_1', tag: 'v1.0.0', category: 'release', project_id: 'p1', context_notes: null, created_at: 0 },
      outputs: [{ id: 'out_1', generation_id: 'gen_1', platform: 'twitter', content: 'Hello', revision_number: 0, created_at: 0 }],
    }),
  },
}))

beforeEach(() => {
  setActivePinia(createPinia())
})

describe('useGenerationsStore', () => {
  it('fetchAll populates generations', async () => {
    const { useGenerationsStore } = await import('./generations')
    const store = useGenerationsStore()
    expect(store.generations).toHaveLength(0)
    await store.fetchAll()
    expect(store.generations).toHaveLength(1)
    expect(store.generations[0].tag).toBe('v1.0.0')
  })

  it('fetchOne sets current with outputs', async () => {
    const { useGenerationsStore } = await import('./generations')
    const store = useGenerationsStore()
    await store.fetchOne('gen_1')
    expect(store.current?.generation.id).toBe('gen_1')
    expect(store.current?.outputs).toHaveLength(1)
  })
})
