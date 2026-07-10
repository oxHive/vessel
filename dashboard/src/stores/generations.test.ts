import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
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

describe('subscribeToEvents', () => {
  class FakeEventSource {
    static instances: FakeEventSource[] = []
    listeners: Record<string, (e: MessageEvent) => void> = {}
    url: string
    closed = false
    constructor(url: string) {
      this.url = url
      FakeEventSource.instances.push(this)
    }
    addEventListener(type: string, cb: (e: MessageEvent) => void) {
      this.listeners[type] = cb
    }
    close() {
      this.closed = true
    }
  }

  beforeEach(() => {
    FakeEventSource.instances = []
    vi.stubGlobal('EventSource', FakeEventSource as unknown as typeof EventSource)
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('refetches generation on outputs-updated', async () => {
    const { useGenerationsStore } = await import('./generations')
    const { api } = await import('../api/client')
    const store = useGenerationsStore()
    store.subscribeToEvents('gen_1')
    const es = FakeEventSource.instances[0]
    expect(es.url).toBe('/api/v1/generations/gen_1/events')

    es.listeners['outputs-updated'](new MessageEvent('outputs-updated', { data: '{}' }))
    expect(api.getGeneration).toHaveBeenCalledWith('gen_1')
  })

  it('stores agent reply message', async () => {
    const { useGenerationsStore } = await import('./generations')
    const store = useGenerationsStore()
    store.subscribeToEvents('gen_1')
    const es = FakeEventSource.instances[0]
    es.listeners['agent-reply'](
      new MessageEvent('agent-reply', { data: JSON.stringify({ message: 'revised' }) }),
    )
    expect(store.agentReply).toBe('revised')
  })

  it('ignores malformed agent-reply payload', async () => {
    const { useGenerationsStore } = await import('./generations')
    const store = useGenerationsStore()
    store.subscribeToEvents('gen_1')
    const es = FakeEventSource.instances[0]
    es.listeners['agent-reply'](
      new MessageEvent('agent-reply', { data: JSON.stringify({ message: 'revised' }) }),
    )
    expect(store.agentReply).toBe('revised')

    expect(() =>
      es.listeners['agent-reply'](new MessageEvent('agent-reply', { data: 'not-json' })),
    ).not.toThrow()
    expect(store.agentReply).toBe('revised')
  })

  it('resets agent reply when subscribing to a new generation', async () => {
    const { useGenerationsStore } = await import('./generations')
    const store = useGenerationsStore()
    store.subscribeToEvents('gen_1')
    const es = FakeEventSource.instances[0]
    es.listeners['agent-reply'](
      new MessageEvent('agent-reply', { data: JSON.stringify({ message: 'revised' }) }),
    )
    expect(store.agentReply).toBe('revised')

    store.subscribeToEvents('gen_2')
    expect(store.agentReply).toBeNull()
  })

  it('closes previous source on resubscribe and on unsubscribe', async () => {
    const { useGenerationsStore } = await import('./generations')
    const store = useGenerationsStore()
    store.subscribeToEvents('gen_1')
    store.subscribeToEvents('gen_2')
    expect(FakeEventSource.instances[0].closed).toBe(true)
    store.unsubscribe()
    expect(FakeEventSource.instances[1].closed).toBe(true)
  })
})
