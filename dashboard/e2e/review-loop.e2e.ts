import { expect, test } from '@playwright/test'
import { insertRevisedOutput, seedGeneration } from './helpers'

const api = (genId: string, tail: string) => `/api/v1/generations/${genId}/${tail}`

test('review loop: revise from browser, live update via SSE, done', async ({ page, request }) => {
  const genId = seedGeneration('hello world release')

  await page.goto(`/generation/${genId}`)
  await expect(page.getByText('X (Twitter)')).toBeVisible()
  await expect(page.getByText('hello world release')).toBeVisible()

  // Agent blocks on the long-poll (as vessel_poll_feedback would).
  const poll = request.get(api(genId, 'poll?timeout_ms=15000'))

  // User requests a platform revision from the card.
  await page.getByPlaceholder('Request a revision').fill('punchier')
  await page.getByRole('button', { name: 'Send' }).first().click()
  await expect(page.getByText('⏳ punchier')).toBeVisible()

  // The blocked poll wakes with the note.
  const pollBody = await (await poll).json()
  expect(pollBody.revisions[0].note).toBe('punchier')
  expect(pollBody.session_ended).toBe(false)

  // Agent's one-line status appears in the strip via SSE.
  await request.post(api(genId, 'agent-reply'), { data: { message: 'tightened the hook' } })
  await expect(page.getByText('tightened the hook')).toBeVisible()

  // Agent saves a revision (DB write + outputs-updated notify, like
  // vessel_save); the card updates live without a reload.
  insertRevisedOutput(genId, 'twitter', 'punchier hello world')
  await request.post(api(genId, 'outputs-updated'))
  await expect(page.getByText('punchier hello world')).toBeVisible()
  await expect(page.getByText('rev 1')).toBeVisible()
  await expect(page.getByText('⏳ punchier')).toBeHidden()

  // Done is terminal: inputs disappear, next poll ends the session.
  await page.getByRole('button', { name: 'Done reviewing' }).click()
  await expect(page.getByText('Review complete')).toBeVisible()
  await expect(page.getByPlaceholder('Request a revision')).toBeHidden()

  const after = await (await request.get(api(genId, 'poll?timeout_ms=2000'))).json()
  expect(after.session_ended).toBe(true)
})

test('generation-level note reaches the agent with null platform', async ({ page, request }) => {
  const genId = seedGeneration()

  await page.goto(`/generation/${genId}`)
  await page.getByPlaceholder('e.g. shorter overall').fill('drop hashtags everywhere')
  await page.getByRole('button', { name: 'Send' }).last().click()
  await expect(page.getByText('⏳ drop hashtags everywhere')).toBeVisible()

  const body = await (await request.get(api(genId, 'poll?timeout_ms=5000'))).json()
  expect(body.revisions[0].platform).toBeNull()
  expect(body.revisions[0].note).toBe('drop hashtags everywhere')
})
