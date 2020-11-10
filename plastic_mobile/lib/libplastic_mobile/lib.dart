import 'dart:ffi';

import 'dart:io';
import 'dart:isolate';

import 'package:ffi/ffi.dart';

import 'binding.dart';

final DynamicLibrary _dl = _open();
DynamicLibrary _open() {
  if (Platform.isAndroid) return DynamicLibrary.open('libplastic_mobile.so');
  throw UnsupportedError('This platform is not supported.');
}

final _nativeLib = NativeLibrary(_dl);

/// Must be called before anything
void setup_ffi() {
  _nativeLib.store_dart_post_cobject(NativeApi.postCObject.cast());
}

void run_nes(SendPort port) {
  _nativeLib.run_nes(port.nativePort);
}

void nes_request(int event, [dynamic data = 0]) {
  Pointer<Utf8> dataToSend;

  switch (data.runtimeType) {
    case int:
      dataToSend = Pointer.fromAddress(data);
      break;
    case String:
      dataToSend = Utf8.toUtf8(data);
      break;
    default:
      dataToSend = nullptr;
  }

  _nativeLib.nes_request(event, dataToSend.cast());
}
