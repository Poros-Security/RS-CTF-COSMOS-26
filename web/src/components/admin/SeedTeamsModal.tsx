import {
  ActionIcon,
  Alert,
  Button,
  CopyButton,
  Group,
  Modal,
  ModalProps,
  ScrollArea,
  Select,
  Stack,
  Table,
  Text,
  Textarea,
  Tooltip,
} from '@mantine/core'
import { showNotification } from '@mantine/notifications'
import { mdiCheck, mdiClose, mdiContentCopy, mdiDownload } from '@mdi/js'
import { Icon } from '@mdi/react'
import { FC, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { showErrorMsg } from '@Utils/Shared'
import { quoteSpreadsheetCsvCell } from '@Utils/Csv'
import api, { Division, SeededTeamResult } from '@Api'

interface SeedTeamsModalProps extends ModalProps {
  gameId?: number
  divisions?: Division[]
  onSuccess?: () => void
}

export const SeedTeamsModal: FC<SeedTeamsModalProps> = ({ gameId, divisions, onSuccess, ...props }) => {
  const { t } = useTranslation()
  const [teamNamesRaw, setTeamNamesRaw] = useState('')
  const [selectedGameId, setSelectedGameId] = useState<string | null>(gameId ? String(gameId) : null)
  const [divisionId, setDivisionId] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [results, setResults] = useState<SeededTeamResult[] | null>(null)

  const activeGameId = gameId ?? (selectedGameId ? parseInt(selectedGameId, 10) : undefined)

  // Fetch games list if opened without a pre-set gameId
  const { data: allGamesRes } = api.edit.useEditGetGames(
    { count: 100 },
    undefined,
    !gameId && props.opened
  )
  const allGames = allGamesRes?.data ?? []

  // Fetch divisions dynamically if a game is selected from dropdown
  const { data: dynamicDivisions } = api.edit.useEditGetDivisions(
    activeGameId ?? 0,
    undefined,
    Boolean(activeGameId && !divisions && props.opened)
  )

  const effectiveDivisions = divisions ?? dynamicDivisions ?? []

  const handleClose = () => {
    if (results && onSuccess) {
      onSuccess()
    }
    setResults(null)
    setTeamNamesRaw('')
    setDivisionId(null)
    if (!gameId) {
      setSelectedGameId(null)
    }
    props.onClose()
  }

  const handleSubmit = async () => {
    const lines = teamNamesRaw
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean)

    if (lines.length === 0) {
      showNotification({
        color: 'red',
        message: t('admin.error.no_team_names', 'Please enter at least one team name'),
        icon: <Icon path={mdiClose} size={1} aria-hidden />,
      })
      return
    }

    try {
      setLoading(true)
      let data: SeededTeamResult[] = []

      if (activeGameId) {
        const res = await api.edit.editSeedTeams(activeGameId, {
          teamNames: lines,
          divisionId: divisionId ? parseInt(divisionId, 10) : null,
        })
        data = Array.isArray(res.data)
          ? res.data
          : (res.data as { data?: SeededTeamResult[] })?.data ?? []
      } else {
        const res = await api.admin.adminSeedTeams({
          teamNames: lines,
        })
        data = Array.isArray(res.data)
          ? res.data
          : (res.data as { data?: SeededTeamResult[] })?.data ?? []
      }

      if (data && data.length > 0) {
        setResults(data)
        showNotification({
          color: 'teal',
          message: t('admin.success.seeded_teams', 'Successfully seeded {{count}} teams', {
            count: data.length,
          }),
          icon: <Icon path={mdiCheck} size={1} aria-hidden />,
        })
      }
    } catch (err) {
      showErrorMsg(err, t)
    } finally {
      setLoading(false)
    }
  }

  const csvContent = results
    ? [
        'Team Name,Invite Code',
        ...results.map((r) => `${quoteSpreadsheetCsvCell(r.teamName)},${quoteSpreadsheetCsvCell(r.inviteCode)}`),
      ].join('\n')
    : ''

  const handleDownloadCsv = () => {
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', activeGameId ? `game-${activeGameId}-seeded-teams.csv` : 'platform-seeded-teams.csv')
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    URL.revokeObjectURL(url)
  }

  return (
    <Modal
      {...props}
      onClose={handleClose}
      title={
        <Text fw={650} size="lg">
          {results
            ? t('admin.title.seeded_teams_results', 'Seeded Teams & Invite Codes')
            : t('admin.title.seed_teams', 'Seed Teams')}
        </Text>
      }
      size={results ? 'lg' : 'md'}
    >
      {!results ? (
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            {t(
              'admin.content.seed_teams_help',
              'Create empty teams or enroll existing teams. Enter one team name per line. If a team already exists, it will be enrolled into the selected game. The first participant to join with the invite code becomes the captain.'
            )}
          </Text>

          {!gameId && (
            <Select
              label={t('admin.label.target_game', 'Target Game')}
              placeholder={t('admin.placeholder.select_game', 'Universal / Platform Only (or select a game)')}
              clearable
              data={[
                { value: '', label: t('admin.seed_teams.universal', 'Universal / Platform Only (no game yet)') },
                ...allGames.map((g) => ({
                  value: g.id?.toString() ?? '',
                  label: `${g.title || 'Game #' + g.id} (ID: ${g.id})`,
                })),
              ]}
              value={selectedGameId ?? ''}
              onChange={(val) => {
                setSelectedGameId(val || null)
                setDivisionId(null)
              }}
            />
          )}

          <Textarea
            label={t('admin.label.team_names', 'Team Names (one per line)')}
            placeholder={'Team 01\nTeam 02\nTeam 03'}
            autosize
            minRows={4}
            maxRows={10}
            value={teamNamesRaw}
            onChange={(e) => setTeamNamesRaw(e.currentTarget.value)}
            required
          />

          {effectiveDivisions.length > 0 && (
            <Select
              label={t('admin.label.games.review.division_filter', 'Division')}
              placeholder={t('admin.placeholder.select_division', 'Select division (optional)')}
              clearable
              data={effectiveDivisions.map((d) => ({ value: d.id.toString(), label: d.name }))}
              value={divisionId}
              onChange={setDivisionId}
            />
          )}
          <Group justify="flex-end" mt="md">
            <Button variant="default" onClick={handleClose}>
              {t('common.button.cancel')}
            </Button>
            <Button loading={loading} onClick={handleSubmit}>
              {t('admin.button.seed_teams', 'Seed Teams')}
            </Button>
          </Group>
        </Stack>
      ) : (
        <Stack gap="md">
          <Alert color="teal" variant="light" title={t('admin.title.seed_success', 'Teams Created Successfully')}>
            {t(
              'admin.content.seed_distribute_info',
              'Distribute these invite codes to each team. When the first participant enters the code, they will automatically become the captain, and remaining teammates will join as members.'
            )}
          </Alert>
          <Group justify="space-between">
            <Text fw={500} size="sm">
              {t('admin.content.total_teams', 'Total: {{count}} teams', { count: results.length })}
            </Text>
            <Group gap="xs">
              <CopyButton value={csvContent}>
                {({ copied, copy }) => (
                  <Button
                    variant="light"
                    size="xs"
                    color={copied ? 'teal' : 'gray'}
                    leftSection={<Icon path={mdiContentCopy} size={0.75} />}
                    onClick={copy}
                  >
                    {copied ? t('common.copied', 'Copied CSV') : t('common.copy_csv', 'Copy CSV')}
                  </Button>
                )}
              </CopyButton>
              <Button
                variant="light"
                size="xs"
                leftSection={<Icon path={mdiDownload} size={0.75} />}
                onClick={handleDownloadCsv}
              >
                {t('common.download_csv', 'Download CSV')}
              </Button>
            </Group>
          </Group>
          <ScrollArea.Autosize mah={350} type="auto">
            <Table striped highlightOnHover withTableBorder withColumnBorders>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>{t('admin.table.team_name', 'Team Name')}</Table.Th>
                  <Table.Th>{t('admin.table.invite_code', 'Invite Code')}</Table.Th>
                  <Table.Th w={60}>{t('common.action', 'Copy')}</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {results.map((item) => (
                  <Table.Tr key={item.teamId}>
                    <Table.Td fw={500}>{item.teamName}</Table.Td>
                    <Table.Td>
                      <Text size="xs" ff="monospace" style={{ wordBreak: 'break-all' }}>
                        {item.inviteCode}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <CopyButton value={item.inviteCode}>
                        {({ copied, copy }) => (
                          <Tooltip label={copied ? t('common.copied') : t('common.copy')}>
                            <ActionIcon color={copied ? 'teal' : 'gray'} variant="subtle" onClick={copy}>
                              <Icon path={copied ? mdiCheck : mdiContentCopy} size={0.8} />
                            </ActionIcon>
                          </Tooltip>
                        )}
                      </CopyButton>
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          </ScrollArea.Autosize>
          <Group justify="flex-end" mt="md">
            <Button onClick={handleClose}>{t('common.button.close', 'Done')}</Button>
          </Group>
        </Stack>
      )}
    </Modal>
  )
}
