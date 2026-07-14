import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

/// FFI type definitions for the IRIS core native library.
///
/// Maps to functions exported by iris_core/src/ffi.rs

// ── Native function typedefs ───────────────────────────────────────

// iris_init(device_name: *const c_char, platform: *const c_char, download_dir: *const c_char) -> *mut c_char
typedef IrisInitNative = Pointer<Utf8> Function(
  Pointer<Utf8> deviceName,
  Pointer<Utf8> platform,
  Pointer<Utf8> downloadDir,
);
typedef IrisInitDart = Pointer<Utf8> Function(
  Pointer<Utf8> deviceName,
  Pointer<Utf8> platform,
  Pointer<Utf8> downloadDir,
);

// iris_start() -> *mut c_char
typedef IrisStartNative = Pointer<Utf8> Function();
typedef IrisStartDart = Pointer<Utf8> Function();

// iris_get_devices() -> *mut c_char
typedef IrisGetDevicesNative = Pointer<Utf8> Function();
typedef IrisGetDevicesDart = Pointer<Utf8> Function();

// iris_connect(address: *const c_char) -> *mut c_char
typedef IrisConnectNative = Pointer<Utf8> Function(Pointer<Utf8> address);
typedef IrisConnectDart = Pointer<Utf8> Function(Pointer<Utf8> address);

// iris_encode_frame(rgba: *const u8, len: u32, w: u32, h: u32, encoding: u8) -> *mut c_char
typedef IrisEncodeFrameNative = Pointer<Utf8> Function(
  Pointer<Uint8> rgbaData,
  Uint32 dataLen,
  Uint32 width,
  Uint32 height,
  Uint8 encoding,
);
typedef IrisEncodeFrameDart = Pointer<Utf8> Function(
  Pointer<Uint8> rgbaData,
  int dataLen,
  int width,
  int height,
  int encoding,
);

// iris_decode_frame(base64: *const c_char, w: u32, h: u32, encoding: u8) -> *mut c_char
typedef IrisDecodeFrameNative = Pointer<Utf8> Function(
  Pointer<Utf8> base64Data,
  Uint32 width,
  Uint32 height,
  Uint8 encoding,
);
typedef IrisDecodeFrameDart = Pointer<Utf8> Function(
  Pointer<Utf8> base64Data,
  int width,
  int height,
  int encoding,
);

// iris_get_version() -> *mut c_char
typedef IrisGetVersionNative = Pointer<Utf8> Function();
typedef IrisGetVersionDart = Pointer<Utf8> Function();

// iris_free_string(ptr: *mut c_char)
typedef IrisFreeStringNative = Void Function(Pointer<Utf8> ptr);
typedef IrisFreeStringDart = void Function(Pointer<Utf8> ptr);

// ─── Bindings class ────────────────────────────────────────────────

class IrisBindings {
  final DynamicLibrary _lib;

  late final IrisInitDart init;
  late final IrisStartDart start;
  late final IrisGetDevicesDart getDevices;
  late final IrisConnectDart connect;
  late final IrisEncodeFrameDart encodeFrame;
  late final IrisDecodeFrameDart decodeFrame;
  late final IrisGetVersionDart getVersion;
  late final IrisFreeStringDart _freeString;

  IrisBindings._(this._lib) {
    init = _lib
        .lookupFunction<IrisInitNative, IrisInitDart>('iris_init');
    start = _lib
        .lookupFunction<IrisStartNative, IrisStartDart>('iris_start');
    getDevices = _lib
        .lookupFunction<IrisGetDevicesNative, IrisGetDevicesDart>(
            'iris_get_devices');
    connect = _lib
        .lookupFunction<IrisConnectNative, IrisConnectDart>('iris_connect');
    encodeFrame = _lib
        .lookupFunction<IrisEncodeFrameNative, IrisEncodeFrameDart>(
            'iris_encode_frame');
    decodeFrame = _lib
        .lookupFunction<IrisDecodeFrameNative, IrisDecodeFrameDart>(
            'iris_decode_frame');
    getVersion = _lib
        .lookupFunction<IrisGetVersionNative, IrisGetVersionDart>(
            'iris_get_version');
    _freeString = _lib
        .lookupFunction<IrisFreeStringNative, IrisFreeStringDart>(
            'iris_free_string');
  }

  /// Load the native library based on platform
  static IrisBindings load() {
    final lib = _openNativeLibrary();
    return IrisBindings._(lib);
  }

  static DynamicLibrary _openNativeLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libiris_core.so');
    } else if (Platform.isLinux) {
      return DynamicLibrary.open('libiris_core.so');
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libiris_core.dylib');
    } else if (Platform.isWindows) {
      return DynamicLibrary.open('iris_core.dll');
    } else {
      throw UnsupportedError(
          'Unsupported platform: ${Platform.operatingSystem}');
    }
  }

  /// Helper: call an FFI function returning a string and free the native memory
  String _callStringFn(Pointer<Utf8> Function() fn) {
    final ptr = fn();
    final result = ptr.toDartString();
    _freeString(ptr);
    return result;
  }

  String _callStringFn1<T>(Pointer<Utf8> Function(T) fn, T arg) {
    final ptr = fn(arg);
    final result = ptr.toDartString();
    _freeString(ptr);
    return result;
  }

  void dispose() {
    // Native library is managed by the OS
  }
}
