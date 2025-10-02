import logging
from collections.abc import Callable
from datetime import timedelta
from typing import TYPE_CHECKING, Any, Optional
from uuid import UUID

import discord
from discord.errors import ClientException
from discord.utils import MISSING  # pyright: ignore[reportAny]
from typing_extensions import override

from ._native import (
    AudioSource,
    Bitrate,
    Config,
    ConnectionInvalid,
    PlayError,
    TrackEvent,
    TrackHandle,
)
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
    """
    A voice client for discord.py using the Songbird voice library written in Rust.

    You can use this client by passing the class into
    :meth:`discord.VoiceChannel.connect`:

    ```python
    await channel.connect(cls=songbird.SongbirdClient)
    ```

    To change the connection's settings, you can pass a :class:`Config` in using
    :class:`functools.partial`:

    ```python
    config = Config()
    config.gateway_timeout = timedelta(seconds=30)

    await channel.connect(cls=partial(songbird.SongbirdClient, config=config))
    ```

    Attributes:
    :attr Config config: The connection's configuration.
    :attr session_id: The voice connection session ID.
    :attr token: The voice connection token.
    :attr endpoint: The endpoint connected to.
    :attr channel: The channel connected to.
    """

    channel: "VocalGuildChannel"  # pyright: ignore[reportIncompatibleVariableOverride]

    def __init__(
        self,
        client: discord.Client,
        channel: discord.abc.Connectable,
        config: Config = MISSING,
    ) -> None:
        if client.user is None:
            raise ValueError("client not logged in")

        super().__init__(client, channel)

        if config is MISSING:
            self.config: Config = Config()
        else:
            self.config = config

        self._songbird: NativeSongbirdClient = MISSING
        self.session_id: Optional[str] = None
        self.token: Optional[str] = None
        self.endpoint: Optional[str] = None

        self._track_handle: Optional[TrackHandle] = None
        self._expecting_disconnect: bool = False

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
        self.session_id = data["session_id"]
        channel_id = data["channel_id"]

        await self._songbird.update_state(
            data["session_id"],
            int(data["channel_id"]) if data["channel_id"] is not None else None,  # pyright: ignore[reportUnnecessaryComparison]
        )

        if channel_id is None and not self._expecting_disconnect:  # pyright: ignore[reportUnnecessaryComparison]
            self.cleanup()
        elif self._expecting_disconnect:
            self._expecting_disconnect = False

    @override
    async def on_voice_server_update(self, data: "VoiceServerUpdatePayload", /) -> None:
        self.token = data["token"]
        self.endpoint = data.get("endpoint")

        if self.token is None or self.endpoint is None:  # pyright: ignore[reportUnnecessaryComparison]
            return

        await self._songbird.update_server(self.endpoint, self.token)

    @override
    async def connect(
        self,
        *,
        timeout: float,
        reconnect: bool,
        self_deaf: bool = False,
        self_mute: bool = False,
    ) -> None:
        self._songbird = await NativeSongbirdClient.new(
            self.config,
            self.client.user.id,  # pyright: ignore[reportOptionalMemberAccess]
            self.channel.guild.id,
            self.channel.id,
            self._update_voice_state,
        )
        await self._songbird.connect(
            timedelta(seconds=timeout), reconnect, self_deaf, self_mute
        )

    @override
    async def disconnect(self, *, force: bool = False) -> None:
        if not force and not await self.is_connected():
            return

        self._expecting_disconnect = True

        self.stop()

        try:
            await self._songbird.disconnect()
        finally:
            # drop the native client
            self._songbird = MISSING
            self.cleanup()

    async def set_bitrate(self, bitrate: Bitrate) -> None:
        """
        Sets the bitrate for the Opus encoder. The default value is 128kbps:

        ```python
        Bitrate.bits_per_second(128_000)
        ```

        Alternatively, :meth:`Bitrate.auto` and :meth:`Bitrate.max` are available.
        """

        await self._songbird.set_bitrate(bitrate)

    async def move_to(
        self,
        channel: Optional[discord.abc.Snowflake],
        *,
        timeout: Optional[timedelta] = MISSING,
    ) -> None:
        """
        Moves the client to a different voice channel.

        Parameters:
        :param channel: The channel to move to. Must be a voice channel.
        :param timeout: How long to wait for the move to complete.
        """

        if timeout is MISSING:
            timeout = timedelta(seconds=30)

        await self._songbird.move_to(
            channel.id if channel is not None else None, timeout
        )

    async def is_connected(self) -> bool:
        """Whether the client is connected to a voice channel."""

        if self._songbird is MISSING:
            return False

        try:
            return await self._songbird.is_connected()
        except ConnectionInvalid:
            return False

    async def play(
        self,
        track: Track,
        *,
        after: Optional[Callable[[Optional[Exception]], Any]] = None,  # pyright: ignore[reportExplicitAny]
    ) -> TrackHandle:
        """
        Plays a :class:`Track`.

        The finalizer, `after` is called after the source has been exhausted
        or an error occurred.

        If an error happens while the audio player is running, the exception is
        caught and the audio player is then stopped.  If no `after` callback is
        passed, any caught exception will be logged using the library logger.

        Returns a track handle for controlling playback. The track handle
        is also stored by the client, so you can just call playback methods
        on the client instead.

        Parameters:
        :param Track track: The track to play.
        :param after: The finalizer called when playback has finished. This function
            must have a single parameter containing an optional exception that
            occurred during playback.
        :return: the track handle
        :rtype: TrackHandle
        """

        if self._track_handle is not None:
            raise ClientException("Already playing audio.")

        if not await self.is_connected():
            raise ClientException("Not connected to voice.")

        self._track_handle = await self._songbird.play(track)

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
        """
        Plays an :class:`AudioSource`.

        Generally, playing a :class:`Track` using :meth:`play` is recommended
        if you want fine-grained control over the starting configuration, e.g.
        whether to start paused, playback volume, and so on.

        Other parameters have the same meaning as :meth:`play`.
        """

        if self._track_handle is not None:
            raise ClientException("Already playing audio.")

        if not await self.is_connected():
            raise ClientException("Not connected to voice.")

        self._track_handle = await self._songbird.play_input(input)

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
        """
        Stops the currently playing track. Cannot be resumed.
        """

        if self._track_handle:
            self._track_handle.stop()
            self._track_handle = None

    def pause(self) -> None:
        """
        Pauses the currently playing track.
        """

        if self._track_handle:
            self._track_handle.pause()

    def resume(self) -> None:
        """
        Resumes the currently playing track.
        """

        if self._track_handle:
            self._track_handle.play()

    def set_volume(self, volume: float) -> None:
        """
        Sets the volume for the current track.
        """

        if self._track_handle:
            self._track_handle.set_volume(volume)

    async def make_playable(self) -> None:
        """
        Wait until the current track is ready for playing.
        """

        if self._track_handle:
            await self._track_handle.make_playable()

    async def seek(self, position: timedelta) -> None:
        """
        Seeks along the track to the specified position.
        """

        if self._track_handle:
            await self._track_handle.seek(position)

    def enable_loop(self) -> None:
        """
        Sets the current track to loop indefinitely.
        """

        if self._track_handle:
            self._track_handle.enable_loop()

    def disable_loop(self) -> None:
        """
        Sets the current track to no longer loop.
        """

        if self._track_handle:
            self._track_handle.disable_loop()

    def loop_for(self, count: int) -> None:
        """
        Sets the current track to loop for `count` times. `count` must be a
        non-negative integer.
        """

        if self._track_handle:
            self._track_handle.loop_for(count)

    @property
    def track_uuid(self) -> Optional[UUID]:
        return self._track_handle.uuid if self._track_handle else None
