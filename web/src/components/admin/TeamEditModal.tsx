import {
  ActionIcon,
  Avatar,
  Badge,
  Button,
  Center,
  CopyButton,
  Divider,
  Grid,
  Group,
  Modal,
  ModalProps,
  ScrollArea,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Tooltip,
  useMantineTheme,
} from '@mantine/core'
import { showNotification } from '@mantine/notifications'
import { mdiCheck, mdiContentCopy, mdiLockOutline, mdiPlus, mdiStar } from '@mdi/js'
import { Icon } from '@mdi/react'
import { FC, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { showErrorMsg } from '@Utils/Shared'
import api, { AdminTeamModel, TeamInfoModel } from '@Api'

interface TeamEditModalProps extends ModalProps {
  team: TeamInfoModel
  mutateTeam: (team: TeamInfoModel) => void
}

export const TeamEditModal: FC<TeamEditModalProps> = (props) => {
  const { team, mutateTeam, ...modalProps } = props

  const theme = useMantineTheme()
  const [disabled, setDisabled] = useState(false)
  const [enrolling, setEnrolling] = useState(false)
  const [selectedEnrollGameId, setSelectedEnrollGameId] = useState<string | null>(null)
  const [activeTeam, setActiveTeam] = useState<TeamInfoModel>(team)
  const [teamInfo, setTeamInfo] = useState<AdminTeamModel>({})

  const { t } = useTranslation()

  // Fetch games to allow enrolling this team into games
  const { data: allGamesRes } = api.edit.useEditGetGames(
    { count: 100 },
    undefined,
    Boolean(modalProps.opened)
  )
  const allGames = allGamesRes?.data ?? []

  useEffect(() => {
    setTeamInfo({ ...team })
    setActiveTeam(team)
    setSelectedEnrollGameId(null)
  }, [team])

  const onEnrollInGame = async () => {
    if (!selectedEnrollGameId || !activeTeam.name) return
    setEnrolling(true)
    try {
      const gameIdNum = parseInt(selectedEnrollGameId, 10)
      await api.edit.editSeedTeams(gameIdNum, {
        teamNames: [activeTeam.name],
      })
      showNotification({
        color: 'teal',
        message: t('admin.success.team_enrolled', 'Successfully enrolled team into game'),
        icon: <Icon path={mdiCheck} size={1} />,
      })
      const updatedGameIds = Array.from(new Set([...(activeTeam.gameIds ?? []), gameIdNum]))
      const updated = { ...activeTeam, gameIds: updatedGameIds }
      setActiveTeam(updated)
      mutateTeam(updated)
      setSelectedEnrollGameId(null)
    } catch (e) {
      showErrorMsg(e, t)
    } finally {
      setEnrolling(false)
    }
  }

  const onChangeTeamInfo = async () => {
    setDisabled(true)

    try {
      await api.admin.adminUpdateTeam(activeTeam.id!, teamInfo)
      showNotification({
        color: 'teal',
        message: t('team.notification.updated'),
        icon: <Icon path={mdiCheck} size={1} />,
      })
      mutateTeam({
        ...activeTeam,
        name: teamInfo.name,
        bio: teamInfo.bio,
        locked: teamInfo.locked ?? activeTeam.locked,
      })
      modalProps.onClose()
    } catch (e) {
      showErrorMsg(e, t)
    } finally {
      setDisabled(false)
    }
  }

  return (
    <Modal {...modalProps}>
      {/* User Info */}
      <Stack gap="md" m="auto" mt={15}>
        <Grid grow>
          <Grid.Col span={8}>
            <TextInput
              label={
                <Group justify="left" gap="xs">
                  <Text size="sm">{t('team.label.name')}</Text>
                </Group>
              }
              type="text"
              w="100%"
              value={teamInfo.name ?? 'team'}
              disabled={disabled}
              maxLength={128}
              onChange={(event) => setTeamInfo({ ...teamInfo, name: event.target.value })}
            />
          </Grid.Col>
          <Grid.Col span={4}>
            <Center>
              <Avatar alt="avatar" radius="xl" size={70} src={activeTeam.avatar}>
                {activeTeam.name?.slice(0, 1) ?? 'T'}
              </Avatar>
            </Center>
          </Grid.Col>
        </Grid>

        <Textarea
          label={t('team.label.bio')}
          value={teamInfo.bio ?? t('team.placeholder.bio')}
          w="100%"
          disabled={disabled}
          maxLength={4096}
          autosize
          minRows={2}
          maxRows={4}
          onChange={(event) => setTeamInfo({ ...teamInfo, bio: event.target.value })}
        />

        <Group justify="left">
          <Text size="sm">{t('team.label.members')}</Text>
          {team.locked && <Icon path={mdiLockOutline} size={0.8} color={theme.colors.yellow[6]} />}
        </Group>
        <ScrollArea h={165} offsetScrollbars>
          <Stack gap="xs">
            {activeTeam.members?.map((user) => (
              <Group key={user.id} justify="space-between">
                <Group justify="left">
                  <Avatar alt="avatar" src={user.avatar} radius="xl">
                    {user.userName?.slice(0, 1) ?? 'U'}
                  </Avatar>
                  <Stack gap={0}>
                    <Text fw={500}>{user.userName}</Text>
                    <Text size="xs" c="dimmed">{`#${user.id?.substring(28)}`}</Text>
                  </Stack>
                </Group>
                <Group justify="right">
                  {user.captain && <Icon path={mdiStar} size={1} color={theme.colors.yellow[4]} />}
                </Group>
              </Group>
            ))}
          </Stack>
        </ScrollArea>

        {activeTeam.inviteCode && (
          <TextInput
            label={t('team.label.invite_code', 'Invite Code')}
            value={activeTeam.inviteCode}
            readOnly
            rightSection={
              <CopyButton value={activeTeam.inviteCode}>
                {({ copied, copy }) => (
                  <Tooltip label={copied ? t('common.copied') : t('common.copy')}>
                    <ActionIcon color={copied ? 'teal' : 'gray'} variant="subtle" onClick={copy}>
                      <Icon path={copied ? mdiCheck : mdiContentCopy} size={0.8} />
                    </ActionIcon>
                  </Tooltip>
                )}
              </CopyButton>
            }
          />
        )}

        <Divider />
        <Stack gap="xs">
          <Text size="sm" fw={600}>
            {t('admin.label.game_participations', 'Game Participations')}
          </Text>

          {activeTeam.gameIds && activeTeam.gameIds.length > 0 ? (
            <Group gap="xs" wrap="wrap">
              {activeTeam.gameIds.map((gid) => {
                const gameObj = allGames.find((g) => g.id === gid)
                const label = gameObj?.title ? `${gameObj.title} (ID: ${gid})` : `Game #${gid}`
                return (
                  <Badge key={gid} variant="filled" color="blue">
                    {label}
                  </Badge>
                )
              })}
            </Group>
          ) : (
            <Text size="xs" c="dimmed">
              {t('admin.content.not_enrolled_in_any_game', 'Not enrolled in any game yet.')}
            </Text>
          )}

          <Group align="flex-end" gap="xs">
            <Select
              style={{ flex: 1 }}
              label={t('admin.label.enroll_in_game', 'Enroll into Game')}
              placeholder={t('admin.placeholder.select_game_to_enroll', 'Select game to join...')}
              clearable
              data={allGames
                .filter((g) => !activeTeam.gameIds?.includes(g.id!))
                .map((g) => ({
                  value: g.id!.toString(),
                  label: `${g.title || 'Game #' + g.id} (ID: ${g.id})`,
                }))}
              value={selectedEnrollGameId}
              onChange={setSelectedEnrollGameId}
            />
            <Button
              disabled={!selectedEnrollGameId || disabled}
              loading={enrolling}
              leftSection={<Icon path={mdiPlus} size={0.85} />}
              onClick={onEnrollInGame}
            >
              {t('admin.button.enroll', 'Enroll')}
            </Button>
          </Group>
        </Stack>
        <Divider />

        <Group grow m="auto" w="100%">
          <Button fullWidth disabled={disabled} onClick={onChangeTeamInfo}>
            {t('admin.button.save')}
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
