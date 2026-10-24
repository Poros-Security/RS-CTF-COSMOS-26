/**
 * Attack-Defense targeting and coordinate resolution for the live arena
 * (/games/:id/attack).
 *
 * In A&D, team avatars ring the colosseum rim, and each team's defended
 * services/flags are laid out in a horizontal row beneath their avatar
 * (<g id="svc-${t.id}" transform="translate(0 41)">).
 *
 * When an attack lands, this module resolves the exact (x, y) coordinates
 * of the specific defended service/flag badge on the victim team so that
 * beams, tracers, spark blooms, and floating ACC/PWNED cues strike the
 * actual flag node (mirroring how KotH beams strike the central Torii gate).
 */

export interface ArenaSvcTarget {
  name: string
  status?: string
  pwnUntil?: number
}

export interface ArenaTeamTarget {
  x: number
  y: number
  svc?: ArenaSvcTarget[]
}

export const ARENA_SVC_SLOT_WIDTH = 11
export const ARENA_SVC_SLOT_GAP = 4
export const ARENA_SVC_Y_OFFSET = 46.5

/**
 * Compute the arena-space (0..1000) center coordinate of a team's specific service/flag slot.
 * Falls back to the team avatar center (or default center) if the service is missing or unindexed.
 */
export function teamSvcPos(
  team: ArenaTeamTarget | null | undefined,
  svc: ArenaSvcTarget | null | undefined,
  fallbackCenter = { x: 500, y: 500 }
): { x: number; y: number } {
  if (!team) return fallbackCenter
  if (!svc || !team.svc || team.svc.length === 0) return { x: team.x, y: team.y }
  const idx = team.svc.indexOf(svc)
  if (idx < 0) return { x: team.x, y: team.y }
  const n = team.svc.length
  const tot = n * ARENA_SVC_SLOT_WIDTH + (n - 1) * ARENA_SVC_SLOT_GAP
  const start = -tot / 2
  return {
    x: team.x + start + idx * (ARENA_SVC_SLOT_WIDTH + ARENA_SVC_SLOT_GAP) + ARENA_SVC_SLOT_WIDTH / 2,
    y: team.y + ARENA_SVC_Y_OFFSET,
  }
}

/**
 * Resolve the targeted service object and its coordinate from a victim team and challenge title.
 */
export function resolveAttackTarget(
  victim: ArenaTeamTarget | null | undefined,
  challengeTitle: string | null | undefined,
  fallbackCenter = { x: 500, y: 500 }
): { svc: ArenaSvcTarget | null; target: { x: number; y: number } } {
  if (!victim) return { svc: null, target: fallbackCenter }
  const svcs = victim.svc || []
  let svc: ArenaSvcTarget | null = null
  if (challengeTitle && svcs.length > 0) {
    svc = svcs.find((s) => s.name === challengeTitle) || null
  }
  if (!svc && svcs.length > 0) {
    svc = svcs[0]
  }
  const target = teamSvcPos(victim, svc, fallbackCenter)
  return { svc, target }
}
