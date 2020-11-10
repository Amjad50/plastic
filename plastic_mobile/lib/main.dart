import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:plastic_mobile/libplastic_mobile/binding.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';
import 'package:plastic_mobile/widgets/image_canvas.dart';
import 'package:synchronized/synchronized.dart';

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
      Uint8List msgList = Uint8List.sublistView(msg, 1, msg.length);

      switch (msg.first) {
        case NesResponseType.Log:
          print(Utf8Decoder().convert(msgList));
          break;
        case NesResponseType.Image:
          ui.decodeImageFromPixels(msgList, 256, 240, ui.PixelFormat.bgra8888,
              (img) {
            setState(() {
              _currentImg = img;
            });
          });
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

  @override
  void initState() {
    super.initState();

    _port = ReceivePort();
    _port.listen(_nesHandler);
    setup_ffi();

    WidgetsBinding.instance.addObserver(this);
  }

  void _clickHandler() async {
    Isolate.spawn(run_nes, _port.sendPort);
    _imgDrawer();
  }

  @override
  void dispose() {
    nes_request(NesRequestType.Exit);
    _port.close();

    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
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
              width: 256,
              height: 240,
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
