use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use pyo3::{pyclass, Py, PyAny, Python};
use songbird::{
    driver::Bitrate,
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

#[derive(Debug, Clone)]
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

    pub fn update_server(&self, endpoint: String, token: String) -> SongbirdResult<()> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        call.update_server(endpoint, token);
        Ok(())
    }

    pub fn update_state<C: Into<ChannelId>>(
        &self,
        session_id: String,
        channel_id: Option<C>,
    ) -> SongbirdResult<()> {
        // Workaround for songbird not propagating new voice state if disconnected
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        call.update_state(session_id, channel_id.map(Into::into));

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
            config.driver_timeout = Some(timeout);

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
        let mut call_lock = self.call.lock().await;

        if let Some(call) = &mut *call_lock {
            call.remove_all_global_events();
            call.leave().await?;
        }

        *call_lock = None;

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

    pub fn current_connection(&self) -> SongbirdResult<Option<PyConnectionInfo>> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };
        let Some(connection_info) = call.current_connection() else {
            return Ok(None);
        };

        Ok(Some(PyConnectionInfo {
            channel_id: connection_info.channel_id.0.into(),
            endpoint: connection_info.endpoint.clone(),
            guild_id: connection_info.guild_id.0.into(),
            session_id: connection_info.session_id.clone(),
            token: connection_info.token.clone(),
            user_id: connection_info.user_id.0.into(),
        }))
    }

    pub fn is_connected(&self) -> SongbirdResult<bool> {
        let Some(call) = &mut *self.call.blocking_lock() else {
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

    pub fn is_mute(&self) -> SongbirdResult<bool> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.is_mute())
    }

    pub async fn deafen(&self, deaf: bool) -> SongbirdResult<()> {
        let Some(call) = &mut *self.call.lock().await else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        call.deafen(deaf).await?;

        Ok(())
    }

    pub fn is_deaf(&self) -> SongbirdResult<bool> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.is_deaf())
    }

    pub fn play(&self, track: Track) -> SongbirdResult<TrackHandle> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.play(track))
    }

    pub fn play_only(&self, track: Track) -> SongbirdResult<TrackHandle> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.play_only(track))
    }

    pub fn play_input(&self, input: Input) -> SongbirdResult<TrackHandle> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.play_input(input))
    }

    pub fn play_only_input(&self, input: Input) -> SongbirdResult<TrackHandle> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.play_only_input(input))
    }

    pub fn set_bitrate(&self, bitrate: Bitrate) -> SongbirdResult<()> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.set_bitrate(bitrate))
    }

    pub fn stop(&self) -> SongbirdResult<()> {
        let Some(call) = &mut *self.call.blocking_lock() else {
            return Err(SongbirdError::ConnectionInvalid);
        };

        Ok(call.stop())
    }
}

#[pyclass(
    frozen,
    module = "discord.ext.songbird._native",
    name = "ConnectionInfo"
)]
pub struct PyConnectionInfo {
    /// ID of the voice channel being joined, if it is known.
    #[pyo3(get)]
    channel_id: u64,

    /// URL of the voice websocket gateway server assigned to this call.
    #[pyo3(get)]
    endpoint: String,

    /// ID of the target voice channel's parent guild.
    ///
    /// Bots cannot connect to a guildless (i.e., direct message) voice call.
    #[pyo3(get)]
    guild_id: u64,

    /// Unique string describing this session for validation/authentication purposes.
    #[pyo3(get)]
    session_id: String,

    /// Ephemeral secret used to validate the above session.
    #[pyo3(get)]
    token: String,

    /// UserID of this bot.
    #[pyo3(get)]
    user_id: u64,
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
        let channel_id = channel_id.map(|c| c.0.get());
        let fut = Python::attach(|py| {
            pyo3_async_runtimes::tokio::into_future(
                self.update_voice_state_hook
                    .call1(py, (guild_id.0, channel_id, self_deaf, self_mute))?
                    .into_bound(py),
            )
        })
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to call update voice state hook");
            JoinError::Dropped
        })?;

        fut.await.map_err(|e| {
            tracing::error!(error = ?e, "exception in update voice state hook");
            JoinError::Dropped
        })?;

        Ok(())
    }
}

impl From<DiscordPyVoiceUpdate> for Shard {
    fn from(value: DiscordPyVoiceUpdate) -> Self {
        Shard::Generic(Arc::new(value))
    }
}
