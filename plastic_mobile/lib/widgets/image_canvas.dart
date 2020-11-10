import 'dart:ui' as ui;

import 'package:flutter/material.dart';

class ImagePainter extends CustomPainter {
  final ui.Image img;

  ImagePainter(this.img);

  @override
  void paint(Canvas canvas, Size size) {
    Paint paint = Paint();
    if (img != null) {
      canvas.drawImage(img, Offset(0, 0), paint);
    }
    canvas.save();
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant ImagePainter oldDelegate) =>
      oldDelegate.img != img;
}
