import 'package:flutter/material.dart';
import 'dart:math';

import 'package:flutter/rendering.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';
import 'package:plastic_mobile/widgets/nes_controller_button.dart';
import 'package:provider/provider.dart';

extension NesControllerConverter on NesControllerKey {
  int get nativeKeyIndex {
    return NesControllerKey.values.indexOf(this);
  }
}

class NesController extends StatelessWidget {
  final NesKeyHandler onPress;
  final NesKeyHandler onRelease;
  const NesController(
      {@required this.onPress, @required this.onRelease, Key key})
      : assert(onPress != null),
        assert(onRelease != null),
        super(key: key);

  Widget _buildABButtons() {
    return Container(
      color: Colors.black87,
      child: FittedBox(
        fit: BoxFit.contain,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            for (var k in [
              [NesControllerKey.B, "B"],
              [NesControllerKey.A, "A"],
            ])
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8.0),
                child: NesControllerButton(
                  nesKey: k[0],
                  builder: (clicked) => FloatingActionButton(
                    onPressed: () {},
                    child: Text(k[1]),
                    backgroundColor: clicked ? Colors.redAccent : Colors.red,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildStartSelectButtons(
    BuildContext context,
    BoxConstraints constraints,
  ) {
    List<Widget> children = [
      for (var k in [
        [NesControllerKey.Select, "Select"],
        [NesControllerKey.Start, "Start"],
      ])
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8.0),
          child: NesControllerButton(
            nesKey: k[0],
            builder: (clicked) => RaisedButton(
              onPressed: () {},
              child: Text(k[1]),
              color: clicked ? Colors.grey : Colors.black,
            ),
          ),
        ),
    ];

    Widget child;

    if (constraints.maxWidth >
        Theme.of(context).buttonTheme.minWidth * 2 + 16) {
      child = Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: children,
      );
    } else {
      child = Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: children,
      );
    }

    return ButtonTheme(
      colorScheme: ColorScheme.dark(),
      child: Container(
        color: Colors.black87,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Container(
              decoration: ShapeDecoration(
                color: Colors.white,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(15),
                ),
              ),
              child: child,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildTwoArrowButtons(NesControllerKey key1, NesControllerKey key2) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        NesControllerButton(
          nesKey: key1,
          builder: (clicked) => RaisedButton(
            shape: BeveledRectangleBorder(
              side: BorderSide(color: Colors.white),
              borderRadius: BorderRadius.horizontal(right: Radius.circular(20)),
            ),
            onPressed: () {},
            color: clicked ? Colors.grey : Colors.black,
          ),
        ),
        Padding(padding: EdgeInsets.only(left: 16)),
        NesControllerButton(
          nesKey: key2,
          builder: (clicked) => RaisedButton(
            shape: BeveledRectangleBorder(
              side: BorderSide(color: Colors.white),
              borderRadius: BorderRadius.horizontal(left: Radius.circular(20)),
            ),
            onPressed: () {},
            color: clicked ? Colors.grey : Colors.black,
          ),
        ),
      ],
    );
  }

  Widget _buildArrowButtons(BuildContext _context, BoxConstraints constraints) {
    double size = constraints.maxWidth > constraints.maxHeight
        ? constraints.maxHeight
        : constraints.maxWidth;

    return ButtonTheme(
      minWidth: size / 3,
      child: Container(
        color: Colors.black87,
        child: Stack(
          alignment: AlignmentDirectional.center,
          children: [
            Transform.rotate(
              angle: pi / 2,
              child: _buildTwoArrowButtons(
                NesControllerKey.Up,
                NesControllerKey.Down,
              ),
            ),
            _buildTwoArrowButtons(NesControllerKey.Left, NesControllerKey.Right)
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Provider(
      create: (_context) => NesKeyHandlerProvider(onPress, onRelease),
      builder: (context, child) {
        return Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Expanded(
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Expanded(
                      child: LayoutBuilder(
                        builder: _buildArrowButtons,
                      ),
                    ),
                    Expanded(
                      child: LayoutBuilder(
                        builder: _buildStartSelectButtons,
                      ),
                    ),
                    Expanded(child: _buildABButtons()),
                  ],
                ),
              )
            ],
          ),
        );
      },
    );
  }
}
