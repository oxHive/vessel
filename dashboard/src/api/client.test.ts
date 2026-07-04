import { describe, it, expect, vi, beforeEach } from 'vitest'

const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

beforeEach(() => mockFetch.mockReset())

function makeOk(body: unknown) {
  return Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve(body),
  })
}

describe('api.getGenerations', () => {
  it('calls GET /api/v1/generations and returns count + list', async () => {
    mockFetch.mockReturnValue(makeOk({ count: 1, generations: [{ id: 'gen_1', tag: 'v1.0.0', category: 'release', project_id: 'p1', context_notes: null, created_at: 0 }] }))
    const { api } = await import('./client')
    const result = await api.getGenerations()
    expect(mockFetch).toHaveBeenCalledWith('/api/v1/generations', expect.any(Object))
    expect(result.count).toBe(1)
    expect(result.generations[0].tag).toBe('v1.0.0')
  })
})

describe('api.postFeedback', () => {
  it('calls POST /api/v1/feedback with correct body', async () => {
    mockFetch.mockReturnValue(makeOk({ recorded: true }))
    const { api } = await import('./client')
    await api.postFeedback('gen_1', 'twitter', 'liked')
    expect(mockFetch).toHaveBeenCalledWith('/api/v1/feedback', expect.objectContaining({
      method: 'POST',
      body: JSON.stringify({ generation_id: 'gen_1', platform: 'twitter', signal: 'liked' }),
    }))
  })
})
