import 'dart:typed_data';

import 'package:flutter/services.dart';

abstract class RawAudioPlugin {
  static const CHANNEL_NAME = "com.amjad.plastic/audio";

  static const MethodChannel _channel = const MethodChannel(CHANNEL_NAME);

  static Future<void> newAudioPlayer(int sampleRate) async {
    return await _channel
        .invokeMethod('newAudioPlayer', {'sampleRate': sampleRate});
  }

  static Future<void> addBuffer(Uint8List data) async {
    return await _channel.invokeMethod('addBuffer', {"data": data});
  }

  static Future<int> playState() async {
    return await _channel.invokeMethod<int>('playState');
  }

  static Future<void> pause() async {
    return await _channel.invokeMethod('pause');
  }

  static Future<void> resume() async {
    return await _channel.invokeMethod('resume');
  }

  static Future<void> stop() async {
    return await _channel.invokeMethod('stop');
  }
}
