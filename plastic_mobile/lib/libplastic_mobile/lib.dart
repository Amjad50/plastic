

import 'dart:ffi';

import 'dart:io';

import 'binding.dart';

final DynamicLibrary _dl = _open();
DynamicLibrary _open() {
  if (Platform.isAndroid) return DynamicLibrary.open('libplastic_mobile.so');
  throw UnsupportedError('This platform is not supported.');
}

final _nativeLib = NativeLibrary(_dl);

/// Must be called before anything
void setup() {
  _nativeLib.store_dart_post_cobject(NativeApi.postCObject.cast());
}
