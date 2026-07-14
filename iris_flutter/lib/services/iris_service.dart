import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:network_info_plus/network_info_plus.dart';
import 'package:path_provider/path_provider.dart';

import '../ffi/iris_bindings.dart';

/// High-level service wrapping the Rust FFI bindings.
/// Provides reactive state for the Flutter UI via ChangeNotifier.
class IrisService extends ChangeNotifier {
  // ── State ──
  List<DeviceInfo> _devices = [];
  bool _isInitialized = false;
  bool _isRunning = false;
  String _deviceId = '';
  String? _error;

  // Getters
  List<DeviceInfo> get devices => List.unmodifiable(_devices);
  bool get isInitialized => _isInitialized;
  bool get isRunning => _isRunning;
  String get deviceId => _deviceId;
  String? get error => _error;

  // ── Core Bindings (lazy) ──
  IrisBindings? _bindings;

  /// Initialize the native IRIS engine
  Future<void> initialize() async {
    try {
      // Load native library
      _bindings = IrisBindings.load();

      // Get device info
      final info = NetworkInfo();
      String? hostname;
      try {
        hostname = await info.getWifiName() ?? 'Unknown Device';
      } catch (_) {
        hostname = Platform.localHostname;
      }

      final platform = Platform.operatingSystem;
      final dir = await getApplicationDocumentsDirectory();
      final downloadDir = '${dir.path}/iris_downloads';
      await Directory(downloadDir).create(recursive: true);

      // Call native init
      final resultJson = _bindings!.init(
        hostname,
        platform,
        downloadDir,
      );
      final result = jsonDecode(resultJson) as Map<String, dynamic>;

      if (result['success'] == true) {
        _deviceId = result['device_id'] as String;
        _isInitialized = true;

        // Start services
        _start();
      } else {
        _error = result['error'] as String? ?? 'Unknown error';
      }

      notifyListeners();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }

  void _start() {
    if (_bindings == null || !_isInitialized) return;

    try {
      final resultJson = _bindings!.start();
      final result = jsonDecode(resultJson) as Map<String, dynamic>;
      _isRunning = result['success'] == true;
      if (!_isRunning) {
        _error = result['error'] as String?;
      }
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }

  /// Refresh the list of discovered devices
  void refreshDevices() {
    if (_bindings == null || !_isRunning) return;

    try {
      final devicesJson = _bindings!.getDevices();
      final List<dynamic> list = jsonDecode(devicesJson) as List<dynamic>;
      _devices = list
          .map((d) => DeviceInfo.fromJson(d as Map<String, dynamic>))
          .toList();
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      notifyListeners();
    }
  }

  /// Connect to a remote device
  Future<bool> connectToDevice(String address) async {
    if (_bindings == null) return false;

    try {
      final resultJson = _bindings!.connect(address);
      final result = jsonDecode(resultJson) as Map<String, dynamic>;
      return result['success'] == true;
    } catch (e) {
      _error = e.toString();
      notifyListeners();
      return false;
    }
  }

  /// Encode a raw RGBA frame for sending
  String encodeFrame(Uint8List rgbaData, int width, int height,
      {FrameEncoding encoding = FrameEncoding.h264}) {
    if (_bindings == null) return '';

    return _bindings!.encodeFrame(
      rgbaData,
      rgbaData.length,
      width,
      height,
      encoding.value,
    );
  }

  /// Decode received frame data
  Uint8List? decodeFrame(String base64Data, int width, int height,
      {FrameEncoding encoding = FrameEncoding.h264}) {
    if (_bindings == null) return null;

    final resultJson = _bindings!.decodeFrame(
        base64Data, width, height, encoding.value);
    final result = jsonDecode(resultJson) as Map<String, dynamic>;

    if (result['success'] == true) {
      final data = base64Decode(result['data'] as String);
      return Uint8List.fromList(data);
    }
    return null;
  }

  /// Get protocol/version info
  Map<String, dynamic> getVersionInfo() {
    if (_bindings == null) return {};
    final json = _bindings!.getVersion();
    return jsonDecode(json) as Map<String, dynamic>;
  }

  @override
  void dispose() {
    _bindings?.dispose();
    super.dispose();
  }
}

/// Encoding type for screen frames
enum FrameEncoding {
  rawRgba(0),
  jpeg(1),
  lz4(2),
  h264(3);

  final int value;
  const FrameEncoding(this.value);
}

/// Represents a discovered device on the LAN
class DeviceInfo {
  final String deviceId;
  final String deviceName;
  final String platform;
  final List<String> ipAddresses;

  DeviceInfo({
    required this.deviceId,
    required this.deviceName,
    required this.platform,
    required this.ipAddresses,
  });

  factory DeviceInfo.fromJson(Map<String, dynamic> json) {
    return DeviceInfo(
      deviceId: json['device_id'] as String? ?? '',
      deviceName: json['device_name'] as String? ?? 'Unknown',
      platform: json['platform'] as String? ?? 'unknown',
      ipAddresses: (json['ip_addresses'] as List<dynamic>?)
              ?.map((e) => e.toString())
              .toList() ??
          [],
    );
  }

  /// Get icon based on platform
  IconData get platformIcon {
    switch (platform.toLowerCase()) {
      case 'windows':
        return Icons.desktop_windows;
      case 'macos':
        return Icons.laptop_mac;
      case 'android':
        return Icons.phone_android;
      case 'ios':
        return Icons.phone_iphone;
      default:
        return Icons.devices;
    }
  }
}
