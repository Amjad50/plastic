package com.amjad.plastic

import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.lang.ClassCastException

class RawAudioPlugin : MethodChannel.MethodCallHandler {

    private var player: AudioPlayer? = null

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "newAudioPlayer" -> {
                if (player == null) {
                    if (call.hasArgument("sampleRate")) {
                        try {
                            val sampleRate = call.argument<Int>("sampleRate")
                            if (sampleRate != null) {
                                player = AudioPlayer(sampleRate)
                                result.success(null)
                            } else {
                                result.error("nullArgument", "newAudioPlayer expect argument `sampleRate` of type int and not null", null)
                            }
                        } catch (e: ClassCastException) {
                            result.error("wrongTypeArgument", "newAudioPlayer expect argument `sampleRate` of type int", null)
                        }
                    } else {
                        result.error("argumentNotFound", "newAudioPlayer expect argument `sampleRate` of type int", null)
                    }
                } else {
                    result.error("PlayerAlreadyInitialized", "The player is already initialized", null)
                }
            }
            "addBuffer" -> {
                if (player != null) {
                    if (call.hasArgument("data")) {
                        try {
                            val data = call.argument<ByteArray>("data")
                            if (data != null) {
                                player!!.addBuffer(data)
                                result.success(null)
                            } else {
                                result.error("nullArgument", "addBuffer expect argument `data` of type ByteArray and not null", null)
                            }
                        } catch (e: ClassCastException) {
                            result.error("wrongTypeArgument", "addBuffer expect argument `data` of type ByteArray", null)
                        }
                    } else {
                        result.error("argumentNotFound", "addBuffer expect argument `data` of type ByteArray", null)
                    }
                } else {
                    result.error("NullPlayer", "The player is not initialized", null)
                }
            }
            "playState" -> {
                if (player != null) {
                    result.success(player!!.playState())
                } else {
                    result.error("NullPlayer", "The player is not initialized", null)
                }
            }
            "pause" -> {
                if (player != null) {
                    player!!.pause()
                    result.success(null)
                } else {
                    result.error("NullPlayer", "The player is not initialized", null)
                }
            }
            "resume" -> {
                if (player != null) {
                    player!!.resume()
                    result.success(null)
                } else {
                    result.error("NullPlayer", "The player is not initialized", null)
                }
            }
            "stop" -> {
                if (player != null) {
                    player!!.stop()
                    result.success(null)
                } else {
                    result.error("NullPlayer", "The player is not initialized", null)
                }
            }
            else -> {
                throw IllegalArgumentException("Method with name ${call.method} is not found")
            }
        }
    }

    companion object {
        const val CHANNEL_NAME = "com.amjad.plastic/audio"
    }
}