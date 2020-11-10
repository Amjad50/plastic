import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';

class ImagePainter extends CustomPainter {
  final ui.Image img;

  ImagePainter(this.img);

  @override
  void paint(Canvas canvas, Size size) {
    Paint paint = Paint();
    canvas.scale(size.width / TV_WIDTH);
    if (img != null) {
      canvas.drawImage(img, Offset.zero, paint);
    }
    canvas.save();
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant ImagePainter oldDelegate) =>
      oldDelegate.img != img;
}
