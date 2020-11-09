import 'dart:convert';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:plastic_mobile/libplastic_mobile/binding.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';

void main() {
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

class _MyHomePageState extends State<MyHomePage> {
  int _counter = 0;
  ReceivePort _port;

  void _nesHandler(dynamic msg) {
    if (msg is Uint8List && msg.isNotEmpty) {
      switch (msg.first) {
        case NesResponseType.Log:
          print(Utf8Decoder().convert(msg.getRange(1, msg.length).toList()));
          break;
        case NesResponseType.Image:
          print("got image");
          print(msg.getRange(1, msg.length));
          break;
        case NesResponseType.SavesPresent:
          print("got saves");
          print(msg.getRange(1, msg.length));
          break;
        case NesResponseType.ExitResponse:
          _port.close();
          print("exiting...");
          break;
      }
    } else {
      print("Got unknown type message: $msg");
    }
  }

  @override
  void initState() {
    super.initState();

    _port = ReceivePort();
    _port.listen(_nesHandler);
    setup_ffi();
  }

  void _clickHandler() {
    Isolate.spawn(run_nes, _port.sendPort);
  }

  @override
  void dispose() {
    nes_request(NesRequestType.Exit);
    _port.close();
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
            Text("NES tester"),
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
