use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use pyo3::{Py, PyAny, Python};
use songbird::{
    error::{JoinError, JoinResult},
    id::{ChannelId, GuildId, UserId},
    input::Input,
    shards::{Shard, VoiceUpdate},
    tracks::{Track, TrackHandle},
    Call,
};
use tokio::sync::Mutex;

use crate::{
    config::Config,
    error::{SongbirdError, SongbirdResult},
};

#[derive(Debug)]
pub struct VoiceConnection {
    call: Arc<Mutex<Option<Call>>>,
    channel_id: ChannelId,
}

impl VoiceConnection {
    pub fn new<U: Into<UserId>, G: Into<GuildId>, C: Into<ChannelId>>(
        user_id: U,
        guild_id: G,
        channel_id: C,
        update_hook: DiscordPyVoiceUpdate,
    ) -> Self {
        Self {
            call: Arc::new(Mutex::new(Some(Call::new(
                guild_id.into(),
                update_hook.into(),
                user_id.into(),
            )))),
            channel_id: channel_id.into(),
        }
    }

    pub async fn update_server(&self, endpoint: String, token: String) -> SongbirdResult<()> {
        let Some(call) = &mut *self.call.lock().await else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        call.update_server(endpoint, token);
        Ok(())
    }

    pub async fn update_state<C: Into<ChannelId>>(
        &self,
        session_id: String,
        channel_id: Option<C>,
    ) -> SongbirdResult<()> {
        let Some(call) = &mut *self.call.lock().await else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        call.update_state(session_id, channel_id.map(|c| c.into()));
        Ok(())
    }

    pub async fn connect(
        &self,
        config: Config,
        timeout: Duration,
        reconnect: bool,
        self_deaf: bool,
        self_mute: bool,
    ) -> SongbirdResult<()> {
        // we just deep cloned the config, there definitely isn't another reference
        let mut config = config.deep_clone().into_inner();
        let stage_1 = {
            let Some(call) = &mut *self.call.lock().await else {
                return Err(SongbirdError::ConnectionInvalid);
            };

            config.gateway_timeout = Some(timeout);

            if !reconnect {
                config.driver_retry.retry_limit = Some(0)
            }

            call.set_config(config);

            call.mute(self_mute).await?;
            call.deafen(self_deaf).await?;

            call.join(self.channel_id).await?
        };

        // The second await has to be outside because mutexes around the call
        // have to be released before awaiting this result, as per Songbird
        // docs: https://docs.rs/songbird/latest/songbird/struct.Call.html#method.join
        stage_1.await?;

        Ok(())
    }

    pub async fn disconnect(&self) -> SongbirdResult<()> {
        let mut guard = self.call.lock().await;

        if let Some(call) = &mut *guard {
            call.remove_all_global_events();
            call.leave().await?;
        }

        *guard = None;

        Ok(())
    }

    pub async fn move_to<C: Into<ChannelId>>(
        &self,
        channel_id: Option<C>,
        timeout: Option<Duration>,
    ) -> SongbirdResult<()> {
        let stage_1 = {
            let Some(call) = &mut *self.call.lock().await else {
                return Err(SongbirdError::ConnectionInvalid);
            };

            let Some(channel_id) = channel_id else {
                call.leave().await?;
                return Ok(());
            };

            call.join(channel_id.into()).await?
        };

        // The second await has to be outside because mutexes around the call
        // have to be released before awaiting this result, as per Songbird
        // docs: https://docs.rs/songbird/latest/songbird/struct.Call.html#method.join
        match timeout {
            Some(timeout) => tokio::time::timeout(timeout, stage_1)
                .await
                .map_err(|_| JoinError::TimedOut)??,
            None => stage_1.await?,
        }

        Ok(())
    }

    pub async fn is_connected(&self) -> SongbirdResult<bool> {
        let Some(call) = &mut *self.call.lock().await else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.current_connection().is_some())
    }

    pub async fn mute(&self, mute: bool) -> SongbirdResult<()> {
        let Some(call) = &mut *self.call.lock().await else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        call.mute(mute).await?;

        Ok(())
    }

    pub async fn deafen(&self, deaf: bool) -> SongbirdResult<()> {
        let Some(call) = &mut *self.call.lock().await else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        call.deafen(deaf).await?;

        Ok(())
    }

    pub async fn play(&self, track: Track) -> SongbirdResult<TrackHandle> {
        let Some(call) = &mut *self.call.lock().await else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.play(track))
    }

    pub async fn play_input(&self, input: Input) -> SongbirdResult<TrackHandle> {
        let Some(call) = &mut *self.call.lock().await else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.play_input(input))
    }
}

#[derive(Debug)]
pub struct DiscordPyVoiceUpdate {
    pub update_voice_state_hook: Py<PyAny>,
}

#[async_trait]
impl VoiceUpdate for DiscordPyVoiceUpdate {
    async fn update_voice_state(
        &self,
        guild_id: GuildId,
        channel_id: Option<ChannelId>,
        self_deaf: bool,
        self_mute: bool,
    ) -> JoinResult<()> {
        Python::attach(|py| {
            let channel_id = channel_id.map(|c| c.0.get());

            pyo3_async_runtimes::tokio::into_future(
                self.update_voice_state_hook
                    .call1(py, (guild_id.0, channel_id, self_deaf, self_mute))
                    .unwrap()
                    .into_bound(py),
            )
        })
        .map_err(|_| JoinError::Dropped)?
        .await
        .map_err(|_| JoinError::Dropped)?;

        Ok(())
    }
}

impl From<DiscordPyVoiceUpdate> for Shard {
    fn from(value: DiscordPyVoiceUpdate) -> Self {
        Shard::Generic(Arc::new(value))
    }
}
