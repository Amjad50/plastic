import 'dart:async';
import 'dart:convert';
import 'dart:isolate';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_sound/flutter_sound.dart';
import 'package:plastic_mobile/libplastic_mobile/binding.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';
import 'package:plastic_mobile/widgets/image_canvas.dart';
import 'package:synchronized/synchronized.dart';

const int SAMPLE_RATE = 22050;

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // TODO: support different orientations, (this is temporary)
  await SystemChrome.setPreferredOrientations([DeviceOrientation.portraitUp]);
  runApp(MyApp());
}

class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flutter Demo',
      theme: ThemeData(
        primarySwatch: Colors.blue,
        visualDensity: VisualDensity.adaptivePlatformDensity,
      ),
      home: MyHomePage(title: 'Flutter Demo Home Page'),
    );
  }
}

class MyHomePage extends StatefulWidget {
  MyHomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  _MyHomePageState createState() => _MyHomePageState();
}

class _MyHomePageState extends State<MyHomePage> with WidgetsBindingObserver {
  int _counter = 0;
  ReceivePort _port;
  ui.Image _currentImg = null;
  Lock _imageDrawingLock = Lock();
  FlutterSoundPlayer _mPlayer = FlutterSoundPlayer();
  bool _mPlayerIsInited = false;

  @override
  void didChangeAppLifecycleState(ui.AppLifecycleState state) {
    switch (state) {
      case AppLifecycleState.resumed:
        print("resume");
        nes_request(NesRequestType.Resume);
        break;
      case AppLifecycleState.inactive:
        print("inactive");
        break;
      case AppLifecycleState.paused:
        print("pause");
        nes_request(NesRequestType.Pause);
        break;
      case AppLifecycleState.detached:
        print("detach");
        nes_request(NesRequestType.Exit);
        _port.close();
        break;
    }
  }

  void _nesHandler(dynamic msg) {
    if (msg is Uint8List && msg.isNotEmpty) {
      Uint8List msgList = msg.sublist(1);

      switch (msg.first) {
        case NesResponseType.Log:
          print(Utf8Decoder().convert(msgList));
          break;
        case NesResponseType.Image:
          ui.decodeImageFromPixels(
              msgList, TV_WIDTH, TV_HEIGHT, ui.PixelFormat.bgra8888, (img) {
            setState(() {
              _currentImg = img;
            });
          });
          break;
        case NesResponseType.AudioBuffer:
          if (_mPlayerIsInited && _mPlayer != null && !_mPlayer.isStopped) {
            _mPlayer.foodSink.add(FoodData(msgList));
          }
          break;
        case NesResponseType.SavesPresent:
          print("got saves");
          print(msgList.toList());
          break;
        case NesResponseType.ExitResponse:
          _port.close();
          print("exiting...");
          break;
        default:
          print("Got unknown type message: $msg");
      }
    } else {
      print("Got unknown type message: $msg");
    }
  }

  void _imgDrawer() {
    // allow only one entry
    _imageDrawingLock.synchronized(() async {
      while (true) {
        await Future.delayed(Duration(microseconds: 1000000 ~/ 60));
        nes_request(NesRequestType.GetImage);
      }
    });
  }

  void _clickHandler() async {
    Isolate.spawn(run_nes, _port.sendPort);
    _imgDrawer();
    _play();
  }

  void _play() async {
    if (_mPlayerIsInited && _mPlayer.isStopped) {
      print("player starting");
      await _mPlayer.startPlayerFromStream(
        codec: Codec.pcm16,
        numChannels: 1,
        sampleRate: SAMPLE_RATE,
      );
      if (_mPlayer != null) {
        // We must not do stopPlayer() directely //await stopPlayer();
        _mPlayer.foodSink.add(FoodEvent(() async {
          //await _mPlayer.stopPlayer();
          //setState(() {});
          print("food sink callback");
        }));
      }
    }
  }

  Future<void> _stopPlayer() async {
    if (_mPlayer != null) await _mPlayer.stopPlayer();
  }

  @override
  void initState() {
    super.initState();

    _port = ReceivePort();
    _port.listen(_nesHandler);
    setup_ffi();

    WidgetsBinding.instance.addObserver(this);

    // Be careful : openAudioSession return a Future.
    // Do not access your FlutterSoundPlayer or FlutterSoundRecorder before the completion of the Future
    _mPlayer.openAudioSession().then((value) {
      setState(() {
        _mPlayerIsInited = true;
      });
    });
  }

  @override
  void dispose() {
    nes_request(NesRequestType.Exit);
    _port.close();

    WidgetsBinding.instance.removeObserver(this);

    // maybe need await?
    _stopPlayer();
    _mPlayer.closeAudioSession();
    _mPlayer = null;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    double _width = 0;
    double _height = 0;
    Size size = MediaQuery.of(context).size;
    if (size.width < size.height) {
      _width = size.width;
      _height = _width / TV_WIDTH * TV_HEIGHT;
    } else {
      _height = size.height;
      _width = _height / TV_HEIGHT * TV_WIDTH;
    }

    return Scaffold(
      appBar: AppBar(
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            // drawing screen
            Container(
              width: _width,
              height: _height,
              child: CustomPaint(
                painter: ImagePainter(_currentImg),
              ),
            ),
            RaisedButton(
              onPressed: () async {
                FilePickerResult result = await FilePicker.platform.pickFiles();

                if (result != null) {
                  nes_request(NesRequestType.LoadRom, result.files.first.path);
                }
              },
              child: Text("open game"),
            ),
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _clickHandler,
        tooltip: 'Start Nes',
        child: Icon(Icons.add),
      ),
    );
  }
}
