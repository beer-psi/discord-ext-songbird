from uuid import UUID, uuid4

from ._native import AudioSource, LoopState, PlayMode
from ._native import Track as NativeTrack


class Track:
    def __init__(self, source: AudioSource):
        self.source: AudioSource = source
        self.playing: PlayMode = PlayMode.Play
        self.volume: float = 1.0
        self.loop_state: LoopState = LoopState.finite(0)
        self.uuid: UUID = uuid4()

    def play(self) -> None:
        self.playing = PlayMode.Play

    def pause(self) -> None:
        self.playing = PlayMode.Pause

    def into_track(self):
        return NativeTrack(
            self.source, self.playing, self.volume, self.loop_state, self.uuid
        )
