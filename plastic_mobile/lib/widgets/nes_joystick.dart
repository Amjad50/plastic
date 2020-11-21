import 'package:control_pad/views/joystick_view.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';
import 'package:plastic_mobile/widgets/nes_controller_button.dart';

class NesJoyStick extends StatelessWidget {
  final BoxConstraints constraints;
  final NesKeyHandler onPress;
  final NesKeyHandler onRelease;

  // up down left right
  List<bool> _arrows = [false, false, false, false];
  final _buttons = [
    NesControllerKey.Up,
    NesControllerKey.Down,
    NesControllerKey.Left,
    NesControllerKey.Right,
  ];
  final _arrowStates = [
    [3, 0],
    [3],
    [3, 1],
    [1],
    [2, 1],
    [2],
    [2, 0],
    [0],
  ];

  NesJoyStick(
      {Key key,
      this.constraints,
      @required this.onPress,
      @required this.onRelease})
      : assert(onPress != null),
        assert(onRelease != null),
        super(key: key);

  @override
  Widget build(BuildContext context) {
    return JoystickView(
      onDirectionChanged: (angle, distance) {
        int index = ((angle - 22.5) / 45).floor();
        if (index == -1) {
          index = 7;
        }

        if (distance > 0.6) {
          var indecies = _arrowStates[index];
          var new_state = [false, false, false, false];

          for (int i in indecies) {
            new_state[i] = true;
          }

          // need update
          if (new_state != _arrows) {
            onRelease(NesControllerKey.Up);
            onRelease(NesControllerKey.Down);
            onRelease(NesControllerKey.Left);
            onRelease(NesControllerKey.Right);

            for (int i in indecies) {
              onPress(_buttons[i]);
            }
          }

          _arrows = new_state;
        } else {
          onRelease(NesControllerKey.Up);
          onRelease(NesControllerKey.Down);
          onRelease(NesControllerKey.Left);
          onRelease(NesControllerKey.Right);
        }
      },
      size: constraints.biggest.shortestSide - 16,
    );
  }
}
