import 'dart:async';

import 'package:flutter/material.dart';
import 'package:plastic_mobile/libplastic_mobile/lib.dart';
import 'package:provider/provider.dart';

typedef NesKeyHandler = void Function(NesControllerKey);
typedef NesClickBuilder = Widget Function(bool);

class NesKeyHandlerProvider {
  final NesKeyHandler onPress;
  final NesKeyHandler onRelease;

  NesKeyHandlerProvider(this.onPress, this.onRelease)
      : assert(onPress != null),
        assert(onRelease != null);
}

class NesControllerButton extends StatefulWidget {
  final NesClickBuilder builder;
  final NesControllerKey nesKey;

  NesControllerButton({
    Key key,
    this.nesKey,
    @required this.builder,
  })  : assert(builder != null),
        super(key: key);

  @override
  _NesControllerButtonState createState() => _NesControllerButtonState();
}

class _NesControllerButtonState extends State<NesControllerButton> {
  bool _clicked = false;
  bool _holding = false;
  // adds a delay before releasing the button. (for fast tapping).
  Completer completer = Completer();

  @override
  Widget build(BuildContext context) {
    NesKeyHandlerProvider provider = context.watch<NesKeyHandlerProvider>();

    void _press(_) {
      setState(() {
        _clicked = true;
      });
      completer = Completer();
      Future.delayed(Duration(milliseconds: 10))
          .then((value) => completer.complete());
      provider.onPress(widget.nesKey);
    }

    void _release(_) async {
      await completer.future;
      setState(() {
        _clicked = false;
      });
      provider.onRelease(widget.nesKey);
    }

    return GestureDetector(
      behavior: HitTestBehavior.translucent,
      child: IgnorePointer(child: widget.builder(_clicked)),
      onTapUp: _release,
      // FIXME: would be better if we support long press
      // onLongPressEnd: _release,
      // onLongPressStart: _press,
      onTapDown: _press,
      onTapCancel: () {
        _release(null);
      },
    );
  }
}
