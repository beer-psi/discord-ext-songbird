# pyright: standard
# This example requires the 'message_content' privileged intent to function.

import asyncio
import logging
import os

import discord
import yt_dlp
import yt_dlp.utils
from discord.ext import commands, songbird


def bug_reports_message(before=";"):
    return None


# Suppress noise about console usage from errors
yt_dlp.utils.bug_reports_message = bug_reports_message

ytdl = yt_dlp.YoutubeDL(
    {
        "format": "ba[abr>0][vcodec=none]/best",
        "outtmpl": "%(extractor)s-%(id)s-%(title)s.%(ext)s",
        "restrictfilenames": True,
        "noplaylist": True,
        "nocheckcertificate": True,
        "ignoreerrors": False,
        "logtostderr": False,
        "no_warnings": True,
        "default_search": "auto",
        "source_address": "0.0.0.0",
    }
)

discord.utils.setup_logging(root=True)
logging.getLogger().setLevel(logging.WARN)
logging.getLogger("discord").setLevel(logging.INFO)
logging.getLogger("songbird").setLevel(logging.INFO)


class Music(commands.Cog):
    def __init__(self, bot: commands.Bot):
        self.bot = bot

    @commands.command()
    async def join(self, ctx, *, channel: discord.VoiceChannel):
        """Joins a voice channel"""

        if ctx.voice_client is not None:
            return await ctx.voice_client.move_to(channel)

        await channel.connect(cls=songbird.SongbirdClient, self_deaf=True)

    @commands.command()
    async def play(self, ctx, *, query):
        """Plays a file from the local filesystem"""

        source = songbird.File(query)
        await ctx.voice_client.play_input(source)

        await ctx.send(f"Now playing: {query}")

    @commands.command()
    async def stream(self, ctx, *, url):
        """Plays from a url (almost anything youtube_dl supports)"""
        async with ctx.typing():
            loop = asyncio.get_event_loop()
            data = await loop.run_in_executor(
                None, lambda: ytdl.extract_info(url, download=False)
            )

            if "entries" in data:
                # take first item from a playlist
                data = data["entries"][0]

            source = songbird.HttpRequest(data["url"])  # pyright: ignore[reportTypedDictNotRequiredAccess]
            await ctx.voice_client.play_input(source)

        await ctx.send(f"Now playing: {data.get('title')}")

    @commands.command()
    async def volume(self, ctx, volume: commands.Range[int, 0, 100]):
        """Changes the player's volume"""

        if ctx.voice_client is None:
            return await ctx.send("Not connected to a voice channel.")

        ctx.voice_client.set_volume(volume / 100)

        await ctx.send(f"Changed volume to {volume}%")

    @commands.command()
    async def stop(self, ctx):
        """Stops and disconnects the bot from voice"""

        await ctx.voice_client.disconnect()

    @play.before_invoke
    @stream.before_invoke
    # @stream.before_invoke
    async def ensure_voice(self, ctx):
        if ctx.voice_client is None:
            if ctx.author.voice:
                await ctx.author.voice.channel.connect(cls=songbird.SongbirdClient)
            else:
                await ctx.send("You are not connected to a voice channel.")
                raise commands.CommandError("Author not connected to a voice channel.")
        else:
            ctx.voice_client.stop()


intents = discord.Intents.default()
intents.message_content = True

bot = commands.Bot(
    command_prefix=commands.when_mentioned_or("!"),
    description="Relatively simple music bot example",
    intents=intents,
)


@bot.event
async def on_ready():
    # Tell the type checker that User is filled up at this point
    assert bot.user is not None

    print(f"Logged in as {bot.user} (ID: {bot.user.id})")
    print("------")


async def main():
    async with bot:
        await bot.add_cog(Music(bot))
        await bot.start(os.environ["DISCORD_TOKEN"])


asyncio.run(main())
