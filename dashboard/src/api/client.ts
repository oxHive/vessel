// --- Types ---

export interface Generation {
  id: string
  project_id: string
  tag: string
  category: string
  context_notes: string | null
  created_at: number
  review_state: string
}

export interface GenerationOutput {
  id: string
  generation_id: string
  platform: string
  content: string
  revision_number: number
  created_at: number
}

export interface Profile {
  id: string
  name: string
  formality: string
  humor: string
  technical_depth: string
  self_promotion: string
  created_at: number
  updated_at: number
}

export interface Project {
  id: string
  profile_id: string
  repo_path: string | null
  github_repo: string | null
  provider: string
  created_at: number
}

export interface Settings {
  port: number
  hivemind_port: number
  hivemind_available: boolean
  db_path: string
  version: string
}

export type FeedbackSignal = 'liked' | 'disliked' | 'reused'

export interface PlatformMeta {
  name: string
  charLimit: number | null
}

export const PLATFORMS: Record<string, PlatformMeta> = {
  twitter:       { name: 'X (Twitter)',    charLimit: 280 },
  linkedin:      { name: 'LinkedIn',       charLimit: 3000 },
  bluesky:       { name: 'Bluesky',        charLimit: 300 },
  mastodon:      { name: 'Mastodon',       charLimit: 500 },
  discord:       { name: 'Discord',        charLimit: null },
  github_release:{ name: 'GitHub Release', charLimit: null },
}

// --- Fetch helper ---

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  })
  if (!res.ok) throw new Error(`${res.status} ${path}`)
  return res.json() as Promise<T>
}

// --- API ---

export const api = {
  // Generations
  getGenerations: () =>
    request<{ count: number; generations: Generation[] }>('/api/v1/generations'),

  getGeneration: (id: string) =>
    request<{ generation: Generation; outputs: GenerationOutput[] }>(`/api/v1/generations/${id}`),

  // Feedback
  postFeedback: (generation_id: string, platform: string, signal: FeedbackSignal) =>
    request<{ recorded: boolean }>('/api/v1/feedback', {
      method: 'POST',
      body: JSON.stringify({ generation_id, platform, signal }),
    }),

  postRevision: (generation_id: string, note: string, platform?: string) =>
    request<{ queued: boolean }>(`/api/v1/generations/${generation_id}/revisions`, {
      method: 'POST',
      body: JSON.stringify({ note, platform: platform ?? null }),
    }),

  markReviewDone: (generation_id: string) =>
    request<{ done: boolean }>(`/api/v1/generations/${generation_id}/done`, {
      method: 'POST',
    }),

  // Profiles
  getProfiles: () =>
    request<{ count: number; profiles: Profile[] }>('/api/v1/profiles'),

  createProfile: (body: { name: string; formality?: string; humor?: string; technical_depth?: string; self_promotion?: string }) =>
    request<{ id: string }>('/api/v1/profiles', { method: 'POST', body: JSON.stringify(body) }),

  updateProfile: (id: string, body: Partial<Omit<Profile, 'id' | 'created_at' | 'updated_at'>>) =>
    request<{ updated: boolean; id: string }>(`/api/v1/profiles/${id}`, { method: 'PATCH', body: JSON.stringify(body) }),

  // Projects
  getProjects: () =>
    request<{ count: number; projects: Project[] }>('/api/v1/projects'),

  createProject: (body: { profile_id: string; repo_path?: string; github_repo?: string; provider?: string }) =>
    request<{ id: string }>('/api/v1/projects', { method: 'POST', body: JSON.stringify(body) }),

  getProjectTags: (id: string) =>
    request<{ tags: string[] }>(`/api/v1/projects/${id}/tags`),

  // Settings
  getSettings: () =>
    request<Settings>('/api/v1/settings'),

  storeGithubToken: (project_id: string, token: string) =>
    request<{ stored: boolean }>('/api/v1/settings/github-token', {
      method: 'POST',
      body: JSON.stringify({ project_id, token }),
    }),

  deleteGithubToken: (project_id: string) =>
    request<{ deleted: boolean }>(`/api/v1/settings/github-token/${project_id}`, { method: 'DELETE' }),
}
