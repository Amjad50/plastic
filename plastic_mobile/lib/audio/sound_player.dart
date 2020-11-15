import 'dart:async';
import 'dart:typed_data';

import 'package:plastic_mobile/audio/raw_audio_plugin.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';
import 'package:synchronized/synchronized.dart';

final int SAMPLE_RATE = nes_sample_rate();

class SoundPlayer {
  StreamController<Uint8List> _dataStream = StreamController();
  StreamSubscription<Uint8List> _dataStreamSubscription;
  Lock _lock = Lock();

  SoundPlayer() {
    _dataStreamSubscription = _dataStream.stream.listen((data) {
      _dataStreamSubscription.pause(RawAudioPlugin.addBuffer(data));
    });

    RawAudioPlugin.newAudioPlayer(SAMPLE_RATE);
  }

  void addBuffer(Uint8List buffer) {
    _dataStream.sink.add(buffer);
  }

  void pause() async {
    _lock.synchronized(() {
      RawAudioPlugin.pause();
    });
  }

  void resume() async {
    _lock.synchronized(() {
      RawAudioPlugin.resume();
    });
  }

  void stop() async {
    RawAudioPlugin.stop();
    if (_dataStreamSubscription != null) _dataStreamSubscription.cancel();
    _dataStream.close();
    _dataStream.sink.close();
  }

  void dispose() {
    stop();
  }
}
