import assert from 'node:assert/strict'
import test from 'node:test'
import { resolveAttackTarget, teamSvcPos } from './arenaAttack'

test('teamSvcPos falls back safely when team or services are absent', () => {
  const center = { x: 500, y: 500 }
  assert.deepEqual(teamSvcPos(null, null, center), center)
  assert.deepEqual(teamSvcPos(undefined, null, center), center)

  const emptyTeam = { x: 200, y: 300, svc: [] }
  assert.deepEqual(teamSvcPos(emptyTeam, null, center), { x: 200, y: 300 })

  const unindexedSvc = { name: 'unindexed' }
  const team = { x: 200, y: 300, svc: [{ name: 'web' }] }
  assert.deepEqual(teamSvcPos(team, unindexedSvc, center), { x: 200, y: 300 })
})

test('teamSvcPos computes exact slot coordinate for single and multiple services', () => {
  const s1 = { name: 'pwn' }
  const singleTeam = { x: 400, y: 600, svc: [s1] }
  // With 1 service: tot = 11, start = -5.5. x = 400 - 5.5 + 0 + 5.5 = 400.
  // y = 600 + 46.5 = 646.5.
  assert.deepEqual(teamSvcPos(singleTeam, s1), { x: 400, y: 646.5 })

  const sA = { name: 'web' }
  const sB = { name: 'crypto' }
  const sC = { name: 'rev' }
  const multiTeam = { x: 100, y: 200, svc: [sA, sB, sC] }
  // n = 3, w = 11, gap = 4. tot = 3 * 11 + 2 * 4 = 33 + 8 = 41. start = -20.5.
  // sA (idx 0): x = 100 - 20.5 + 0 * 15 + 5.5 = 85.
  // sB (idx 1): x = 100 - 20.5 + 1 * 15 + 5.5 = 100.
  // sC (idx 2): x = 100 - 20.5 + 2 * 15 + 5.5 = 115.
  assert.deepEqual(teamSvcPos(multiTeam, sA), { x: 85, y: 246.5 })
  assert.deepEqual(teamSvcPos(multiTeam, sB), { x: 100, y: 246.5 })
  assert.deepEqual(teamSvcPos(multiTeam, sC), { x: 115, y: 246.5 })
})

test('resolveAttackTarget matches by challenge title or defaults to first available service', () => {
  const s1 = { name: 'sql-inj' }
  const s2 = { name: 'buffer-overflow' }
  const team = { x: 300, y: 400, svc: [s1, s2] }

  const matchRes = resolveAttackTarget(team, 'buffer-overflow')
  assert.equal(matchRes.svc, s2)
  assert.deepEqual(matchRes.target, teamSvcPos(team, s2))

  const unknownRes = resolveAttackTarget(team, 'unknown-service')
  assert.equal(unknownRes.svc, s1)
  assert.deepEqual(unknownRes.target, teamSvcPos(team, s1))

  const nullTeamRes = resolveAttackTarget(null, 'anything')
  assert.equal(nullTeamRes.svc, null)
  assert.deepEqual(nullTeamRes.target, { x: 500, y: 500 })
})
