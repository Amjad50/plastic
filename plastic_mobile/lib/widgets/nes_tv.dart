import 'package:flutter/material.dart';
import 'package:plastic_mobile/providers/frame_provider.dart';
import 'package:plastic_mobile/widgets/image_canvas.dart';

import 'package:provider/provider.dart';

class NesTV extends StatefulWidget {
  NesTV({Key key}) : super(key: key);

  @override
  _NesTVState createState() => _NesTVState();
}

class _NesTVState extends State<NesTV> {
  @override
  Widget build(BuildContext context) {
    final provider = context.watch<FrameProvider>();

    return CustomPaint(
      willChange: true,
      painter: ImagePainter(provider.image),
    );
  }
}

// class NesTV extends StatelessWidget {
//   const NesTV({Key key}) : super(key: key);

//   @override
//   Widget build(BuildContext context) {
//     // final provider = Provider.of(context).watch<FrameProvider>();

//     // return CustomPaint(
//     //   willChange: true,
//     //   painter: ImagePainter(provider.image),
//     // );
//     return Container();
//   }
// }
