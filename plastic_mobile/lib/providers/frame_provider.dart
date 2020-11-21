import 'package:flutter/widgets.dart';

import 'dart:ui' as ui;

class FrameProvider extends ChangeNotifier {
  ui.Image _image;

  ui.Image get image {
    return _image;
  }

  set image(ui.Image value) {
    _image = value;
    notifyListeners();
  }
}
