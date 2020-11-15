import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';

class ImagePainter extends CustomPainter {
  final ui.Image img;

  ImagePainter(this.img);

  @override
  void paint(Canvas canvas, Size size) {
    Paint paint = Paint();
    if (img != null) {
      canvas.scale(size.width / TV_WIDTH);
      canvas.drawImage(img, Offset.zero, paint);
    } else {
      paint.color = Colors.black38;
      canvas.drawRect(Offset.zero & size, paint);
    }
    canvas.save();
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant ImagePainter oldDelegate) =>
      oldDelegate.img != img;
}
