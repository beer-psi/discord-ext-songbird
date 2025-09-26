import logging
from collections.abc import Callable
from datetime import timedelta
from typing import TYPE_CHECKING, Any, Optional
from uuid import UUID

import discord
from discord.errors import ClientException
from discord.utils import MISSING  # pyright: ignore[reportAny]
from typing_extensions import override

from ._native import AudioSource, Config, PlayError, TrackEvent, TrackHandle
from ._native import SongbirdClient as NativeSongbirdClient
from .track import Track

if TYPE_CHECKING:
    from discord.guild import VocalGuildChannel
    from discord.types.voice import (
        GuildVoiceState as GuildVoiceStatePayload,
    )
    from discord.types.voice import (
        VoiceServerUpdate as VoiceServerUpdatePayload,
    )

logger = logging.getLogger("discord.ext.songbird")


class SongbirdClient(discord.VoiceProtocol):
    channel: "VocalGuildChannel"  # pyright: ignore[reportIncompatibleVariableOverride]

    def __init__(
        self, client: discord.Client, channel: discord.abc.Connectable
    ) -> None:
        if client.user is None:
            raise ValueError("client not logged in")

        super().__init__(client, channel)

        self.songbird: NativeSongbirdClient = NativeSongbirdClient(
            self.channel.id, Config()
        )
        self.token: Optional[str] = None

        self._track_handle: Optional[TrackHandle] = None

    async def _update_voice_state(
        self, guild_id: int, channel_id: Optional[int], self_deaf: bool, self_mute: bool
    ):
        if guild_id != self.channel.guild.id:
            raise ValueError(
                f"voice state hook unexpectedly called for guild ID {guild_id}"
            )

        await self.channel.guild.change_voice_state(
            channel=discord.Object(id=channel_id) if channel_id is not None else None,
            self_deaf=self_deaf,
            self_mute=self_mute,
        )

    @override
    async def on_voice_state_update(self, data: "GuildVoiceStatePayload", /) -> None:
        await self.songbird.update_state(
            data["session_id"],
            int(data["channel_id"]) if data["channel_id"] is not None else None,  # pyright: ignore[reportUnnecessaryComparison]
        )

    @override
    async def on_voice_server_update(self, data: "VoiceServerUpdatePayload", /) -> None:
        self.token = data["token"]
        endpoint = data.get("endpoint")

        if self.token is None or endpoint is None:  # pyright: ignore[reportUnnecessaryComparison]
            return

        await self.songbird.update_server(endpoint, self.token)

    @override
    async def connect(
        self,
        *,
        timeout: float,
        reconnect: bool,
        self_deaf: bool = False,
        self_mute: bool = False,
    ) -> None:
        await self.songbird.start(
            self.client.user.id,  # pyright: ignore[reportOptionalMemberAccess]
            self.channel.guild.id,
            self._update_voice_state,
        )
        await self.songbird.connect(
            timedelta(seconds=timeout), reconnect, self_deaf, self_mute
        )

    @override
    async def disconnect(self, *, force: bool = False) -> None:
        if not force and not await self.is_connected():
            return

        self.stop()

        try:
            await self.songbird.disconnect()
        finally:
            self.cleanup()

    async def move_to(
        self,
        channel: Optional[discord.abc.Snowflake],
        *,
        timeout: Optional[timedelta] = MISSING,
    ) -> None:
        if timeout is MISSING:
            timeout = timedelta(seconds=30)

        await self.songbird.move_to(
            channel.id if channel is not None else None, timeout
        )

    async def is_connected(self) -> bool:
        return await self.songbird.is_connected()

    async def play(
        self,
        track: Track,
        *,
        after: Optional[Callable[[Optional[Exception]], Any]] = None,  # pyright: ignore[reportExplicitAny]
    ) -> TrackHandle:
        if self._track_handle is not None:
            raise ClientException("Already playing audio.")

        if not await self.is_connected():
            raise ClientException("Not connected to voice.")

        self._track_handle = await self.songbird.play(track)

        def on_end(uuid: UUID, error: Optional[PlayError] = None):
            if after is not None:
                try:
                    after(error)
                except Exception as e:
                    e.__context__ = error
                    logger.exception(
                        "Calling the after function for track %s failed.",
                        uuid,
                        exc_info=e,
                    )
            elif error is not None:
                logger.exception("Exception on track %s", uuid, exc_info=error)

            self._track_handle = None

        self._track_handle.add_event(TrackEvent.End, on_end)
        self._track_handle.add_event(TrackEvent.Error, on_end)

        return self._track_handle

    async def play_input(
        self,
        input: AudioSource,
        *,
        after: Optional[Callable[[Optional[Exception]], Any]] = None,  # pyright: ignore[reportExplicitAny]
    ) -> TrackHandle:
        if self._track_handle is not None:
            raise ClientException("Already playing audio.")

        if not await self.is_connected():
            raise ClientException("Not connected to voice.")

        self._track_handle = await self.songbird.play_input(input)

        def on_end(uuid: UUID, error: Optional[PlayError] = None):
            if uuid != self.track_uuid:
                logger.warning(
                    "Ignoring end event for track %s (our track is %s)",
                    uuid,
                    self.track_uuid,
                )
                return

            self._track_handle = None

            if after is not None:
                try:
                    after(error)
                except Exception as e:
                    e.__context__ = error
                    logger.exception(
                        "Calling the after function for track %s failed.",
                        uuid,
                        exc_info=e,
                    )
            elif error is not None:
                logger.exception("Exception on track %s", uuid, exc_info=error)

        self._track_handle.add_event(TrackEvent.End, on_end)
        self._track_handle.add_event(TrackEvent.Error, on_end)

        return self._track_handle

    def stop(self) -> None:
        if self._track_handle:
            self._track_handle.stop()
            self._track_handle = None

    def pause(self) -> None:
        if self._track_handle:
            self._track_handle.pause()

    def resume(self) -> None:
        if self._track_handle:
            self._track_handle.play()

    def set_volume(self, volume: float) -> None:
        if self._track_handle:
            self._track_handle.set_volume(volume)

    async def make_playable(self) -> None:
        if self._track_handle:
            await self._track_handle.make_playable()

    async def seek(self, position: timedelta) -> None:
        if self._track_handle:
            await self._track_handle.seek(position)

    def enable_loop(self) -> None:
        if self._track_handle:
            self._track_handle.enable_loop()

    def disable_loop(self) -> None:
        if self._track_handle:
            self._track_handle.disable_loop()

    def loop_for(self, count: int) -> None:
        if self._track_handle:
            self._track_handle.loop_for(count)

    @property
    def track_uuid(self) -> Optional[UUID]:
        return self._track_handle.uuid if self._track_handle else None
