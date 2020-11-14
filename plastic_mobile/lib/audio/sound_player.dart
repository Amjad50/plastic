import 'dart:typed_data';

import 'package:flutter_sound/flutter_sound.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';

final int SAMPLE_RATE = nes_sample_rate();

class SoundPlayer {
  FlutterSoundPlayer _player = FlutterSoundPlayer();
  bool _mPlayerIsInited = false;

  SoundPlayer() {
    _player.openAudioSession().then((value) {
      _mPlayerIsInited = true;
    });
  }

  void addBuffer(Uint8List buffer) {
    if (_mPlayerIsInited &&
        _player != null &&
        !_player.isStopped &&
        !_player.isPaused) {
      _player.foodSink.add(FoodData(buffer));
    }
  }

  void play() async {
    if (_mPlayerIsInited && _player.isStopped) {
      print("player starting");
      await _player.startPlayerFromStream(
        codec: Codec.pcm16,
        numChannels: 1,
        sampleRate: SAMPLE_RATE,
      );
      // We must not do stopPlayer() directely //await stopPlayer();
      _player.foodSink.add(FoodEvent(() async {
        //await _mPlayer.stopPlayer();
        //setState(() {});
        print("food sink callback");
      }));
    }
  }

  void pause() async {
    _player.pausePlayer();
  }

  void resume() async {
    _player.resumePlayer();
  }

  void stop() async {
    if (_player != null) await _player.stopPlayer();
  }

  void dispose() {
    stop();
    _player.closeAudioSession();
    _player = null;
  }
}
