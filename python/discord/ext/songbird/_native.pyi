import os
from collections.abc import Awaitable, Callable
from datetime import timedelta
from enum import IntEnum
from typing import Any, Literal, overload
from uuid import UUID

from typing_extensions import deprecated

class SongbirdError(Exception): ...
class ConnectionInvalid(SongbirdError): ...
class UnknownCryptoMode(SongbirdError): ...
class UnknownRetryStrategy(SongbirdError): ...
class UnknownTrackEvent(SongbirdError): ...
class JoinError(SongbirdError): ...
class ControlError(SongbirdError): ...
class PlayError(SongbirdError): ...
class SourceConsumed(SongbirdError): ...

class ConnectionInfo:
    @property
    def channel_id(self) -> int: ...
    @property
    def endpoint(self) -> str: ...
    @property
    def guild_id(self) -> int: ...
    @property
    def session_id(self) -> str: ...
    @property
    def token(self) -> str: ...
    @property
    def user_id(self) -> int: ...

class SongbirdClient:
    config: Config

    @staticmethod
    async def new(
        config: Config,
        user_id: int,
        guild_id: int,
        channel_id: int,
        update_voice_state_hook: Callable[[int, int | None, bool, bool], Any],  # pyright: ignore[reportExplicitAny]
        /,
    ) -> SongbirdClient: ...
    def update_server(self, endpoint: str, token: str, /) -> None:
        """Updates the voice server data."""
        ...
    def update_state(self, session_id: str, channel_id: int | None, /) -> None:
        """Updates the internal voice state of the current user."""
        ...
    async def connect(
        self, timeout: timedelta, reconnect: bool, self_deaf: bool, self_mute: bool, /
    ) -> None: ...
    async def disconnect(self) -> None: ...
    async def move_to(
        self, channel_id: int | None, timeout: timedelta | None, /
    ) -> None: ...
    def current_connection(self) -> ConnectionInfo | None: ...
    def is_connected(self) -> bool: ...
    async def mute(self) -> None: ...
    async def unmute(self) -> None: ...
    def is_mute(self) -> bool: ...
    async def deafen(self) -> None: ...
    async def undeafen(self) -> None: ...
    def is_deaf(self) -> bool: ...
    def play(self, track: Track) -> TrackHandle: ...
    def play_only(self, track: Track) -> TrackHandle: ...
    def play_input(self, source: AudioSource) -> TrackHandle:
        """
        Starts playing the given audio source. The audio source is consumed
        (it can no longer be modified). Returns a track handle for control.
        """
        ...
    def play_only_input(self, source: AudioSource) -> TrackHandle: ...
    def set_bitrate(self, bitrate: Bitrate) -> None: ...
    def stop(self) -> None: ...

class Config:
    """Configuration for drivers and calls."""

    crypto_mode: CryptoMode
    """
    Selected tagging mode for voice packet encryption.

    Defaults to :attr:`CryptoMode.Aes256Gcm`.
    """

    gateway_timeout: float | None
    """
    Number of seconds to wait for Discord to reply with connection information.

    This is a useful fallback in the event that:
     * the underlying Discord client restarts and loses a join request, or
     * a channel join fails because the bot is already believed to be there.

    Defaults to 10 seconds. If set to `None`, connections will never time out.
    """

    mix_mode: MixMode
    """
    Configures whether the driver will mix and output stereo or mono Opus data
    over a voice channel.
    """

    preallocated_tracks: int
    """
    Number of concurrently active tracks to allocate memory for.

    This should be set at, or just above, the maximum number of tracks
    you expect your bot will play at the same time. Exceeding the size of
    the internal queue will trigger a larger memory allocation and copy,
    possibly causing the mixer thread to miss a packet deadline.

    Defaults to `1`. The maximum value is `255`.

    Changes to this field in a running driver will only ever increase
    the capacity of the track store.
    """

    driver_retry_strategy: RetryStrategy
    """
    Strategy used to determine how long to wait between retry attempts.

    *Defaults to a :meth:`RetryStrategy.backoff` from 0.25s
    to 10s, with a jitter of `0.1`.*
    """

    driver_retry_limit: int
    """
    The maximum number of retries to attempt.

    `0` will attempt an infinite number of retries.
    """

    use_softclip: bool
    """
    Configures whether or not each mixed audio packet is [soft-clipped] into the
    [-1, 1] audio range.

    Defaults to `True`, preventing clipping and dangerously loud audio from being sent.
    **This operation adds ~3% cost to a standard (non-passthrough) mix cycle.**

    If you *know* that your bot will only play one sound at a time and that
    your volume is between `0.0` and `1.0`, then you can disable soft-clipping
    for a performance boost. If you are playing several sounds at once, do not
    disable this unless you make sure to reduce the volume of each sound.

    [soft-clipped]: https://opus-codec.org/docs/opus_api-1.3.1/group__opus__decoder.html#gaff99598b352e8939dded08d96e125e0b
    """

    driver_timeout: float | None
    """
    Configures the maximum number of seconds to wait for an attempted voice
    connection to Discord.

    Defaults to 10 seconds. If set to `None`, connections will never time out.
    """

    def __new__(cls) -> Config: ...

class CryptoMode(IntEnum):
    """Encryption schemes supported by Discord."""

    Aes256Gcm = 0
    """
    Discord’s currently preferred non-E2EE encryption scheme.

    Packets are encrypted and decrypted using the `AES256GCM` encryption scheme. An
    additional random 4B suffix is used as the source of nonce bytes for the packet.
    This nonce value increments by `1` with each packet.

    Encrypted content begins after the RTP header and extensions, following the SRTP
    specification.

    Nonce width of 4B (32b), at an extra 4B per packet (~0.2 kB/s).
    """

    XChaCha20Poly1305 = 1
    """
    A fallback non-E2EE encryption scheme.

    Packets are encrypted and decrypted using the `XChaCha20Poly1305` encryption
    scheme. An additional random 4B suffix is used as the source of nonce bytes for the
    packet. This nonce value increments by `1` with each packet.

    Encrypted content begins after the RTP header and extensions, following the SRTP
    specification.

    Nonce width of 4B (32b), at an extra 4B per packet (~0.2 kB/s).
    """

class MixMode(IntEnum):
    """Mixing behaviour for sent audio sources processed within the driver."""

    Mono = 0
    """Audio sources will be downmixed into a mono buffer."""

    Stereo = 1
    """
    Audio sources will be mixed into into a stereo buffer, where mono sources will be
    duplicated into both channels.
    """

class RetryStrategy:
    """Logic used to determine how long to wait between retry attempts."""

    @staticmethod
    def every(duration: timedelta) -> RetryStrategy:
        """The driver will wait for the same amount of time between each retry."""
        ...

    @staticmethod
    def backoff(min: timedelta, max: timedelta, jitter: float) -> RetryStrategy:
        """
        Exponential backoff waiting strategy, where the duration between attempts
        (approximately) doubles each time.
        """
        ...

class PlayMode(IntEnum):
    Play = 0
    Pause = 1

class LoopState:
    """Looping behavior for a track."""

    @staticmethod
    def infinite() -> LoopState:
        """Track will loop endlessly until manually stopped."""
        ...

    @staticmethod
    def finite(count: int = 0) -> LoopState:
        """
        Track will loop `n` more times.

        The default is `0`, stopping the track once its input ends.

        Raises if :param:`count` is negative or at least `2 ** 32 - 1`.
        """
        ...

class Track:
    source: AudioSource
    playing: PlayMode
    volume: float
    loop_state: LoopState
    uuid: UUID

    def __new__(
        cls,
        source: AudioSource,
        playing: PlayMode = PlayMode.Play,
        volume: float = 1.0,
        loop_state: LoopState = LoopState.finite(0),
        uuid: UUID | None = None,
        /,
    ) -> Track: ...
    def play(self) -> Track: ...
    def pause(self) -> Track: ...

class TrackHandle:
    """Handle for control of a track."""

    def play(self) -> None: ...
    def pause(self) -> None: ...
    def stop(self) -> None: ...
    def set_volume(self, volume: float) -> None: ...
    async def make_playable(self) -> None: ...
    async def seek(self, position: timedelta) -> None: ...
    @overload
    def add_event(
        self,
        event: Literal[
            TrackEvent.Play,
            TrackEvent.Pause,
            TrackEvent.End,
            TrackEvent.Loop,
            TrackEvent.Preparing,
            TrackEvent.Playable,
        ],
        callback: Callable[[UUID], Any | Awaitable[Any]],  # pyright: ignore[reportExplicitAny]
    ) -> None:
        """Attach an event handler to an audio track. This method requires an active event loop."""
        ...
    @overload
    def add_event(
        self,
        event: Literal[TrackEvent.Error],
        callback: Callable[[UUID, PlayError], Any | Awaitable[Any]],  # pyright: ignore[reportExplicitAny]
    ) -> None:
        """Attach an event handler to an audio track. This method requires an active event loop."""
        ...
    @overload
    def add_event(
        self,
        event: TrackEvent,
        callback: Callable[..., Any | Awaitable[Any]],  # pyright: ignore[reportExplicitAny]
    ) -> None:
        """Attach an event handler to an audio track. This method requires an active event loop."""
        ...
    def enable_loop(self) -> None: ...
    def disable_loop(self) -> None: ...
    def loop_for(self, count: int) -> None:
        """
        Set an audio track to loop a set number of times.

        Raises if `count` is negative or at least `2 ** 32 - 1`.
        """
        ...
    @property
    def uuid(self) -> UUID: ...

class TrackEvent(IntEnum):
    Play = 0
    Pause = 1
    End = 2
    Loop = 3
    Preparing = 4
    Playable = 5
    Error = 6

class AudioSource:
    pass

class File(AudioSource):
    def __new__(cls, path: str | os.PathLike[str]) -> File: ...

class HttpRequest(AudioSource):
    def __new__(cls, url: str) -> File: ...
    @staticmethod
    def with_headers(url: str, headers: dict[str, str | list[str]]) -> File: ...

class RawAudioSource(AudioSource):
    def __new__(cls) -> RawAudioSource: ...
    def mime_type(self) -> str | None: ...
    def length(self) -> int | None: ...
    def read(self) -> bytes: ...
    def seek(self, offset: int, whence: int = os.SEEK_SET) -> int: ...
    def seekable(self) -> bool: ...
    def close(self) -> None: ...

class Bitrate:
    @staticmethod
    @deprecated("Use Bitrate.bits instead")
    def bits_per_second(bitrate: int) -> Bitrate:
        """Explicit bitrate choice (in bits/second)."""
        ...

    @staticmethod
    def bits(bitrate: int) -> Bitrate:
        """Explicit bitrate choice (in bits/second)."""
        ...

    @staticmethod
    def max() -> Bitrate:
        """Maximum bitrate allowed (up to maximum number of bytes for the packet)."""
        ...

    @staticmethod
    def auto() -> Bitrate:
        """Default bitrate decided by the encoder (not recommended)."""
        ...
