package com.amjad.plastic

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack

class AudioPlayer(sampleRate: Int) {
    private val track: AudioTrack

    init {
        track = AudioTrack.Builder()
                .setAudioAttributes(
                        AudioAttributes.Builder()
                                .setUsage(AudioAttributes.USAGE_MEDIA)
                                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                                .setLegacyStreamType(AudioManager.STREAM_MUSIC)
                                .build())
                .setAudioFormat(
                        AudioFormat.Builder()
                                .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                                .setSampleRate(sampleRate)
                                .build())
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setBufferSizeInBytes(4096)
                .build()
    }

    fun playState(): Int {
        return track.playState
    }

    fun pause() {
        track.pause()
    }

    fun resume() {
        track.play()
    }

    fun addBuffer(data: ByteArray) {
        track.write(data, 0, data.size, AudioTrack.WRITE_NON_BLOCKING)
    }

    fun stop() {
        track.stop()
        track.release()
    }
}