/// Support for doing something awesome.
///
/// More dartdocs go here.
library dart;

export 'src/dart_base.dart';

import 'dart:ffi';

import 'generated_bindings.dart';

void name() {
  var r = Rune(DynamicLibrary.open('rune'));
  int capability_type = 0;
  Pointer<NativeFunction<Void Function(Pointer<Void> p1)>> create_capability =
      nullptr;
  Pointer<NativeFunction<Void Function(Pointer<Void> p1)>> free = nullptr;
  Pointer<Image> image = nullptr;
  Pointer<Void> user_data = nullptr;
  r.rune_image_register_capability_handler(
      image, capability_type, user_data, create_capability, free);
}

// TODO: Export any libraries intended for clients of this package.
