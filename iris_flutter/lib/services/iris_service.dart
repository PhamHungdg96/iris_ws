import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:network_info_plus/network_info_plus.dart';
import 'package:path_provider/path_provider.dart';

import '../bridge/iris_api.dart';

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

  /// Initialize the native IRIS engine via flutter_rust_bridge
  Future<void> initialize() async {
    try {
      // Get device info
      String hostname = 'Unknown Device';
      try {
        final info = NetworkInfo();
        final wifiName = await info.getWifiName();
        hostname = wifiName ?? Platform.localHostname;
      } catch (_) {
        hostname = Platform.localHostname;
      }

      final platform = Platform.operatingSystem;
      final dir = await getApplicationDocumentsDirectory();
      final downloadDir = '${dir.path}/iris_downloads';
      await Directory(downloadDir).create(recursive: true);

      // Call native init via flutter_rust_bridge
      debugPrint('IRIS: Calling irisInit...');
      final resultJson = irisInit(
        deviceName: hostname,
        platform: platform,
        downloadDir: downloadDir,
      );
      final result = jsonDecode(resultJson) as Map<String, dynamic>;

      if (result['success'] == true) {
        _deviceId = result['device_id'] as String;
        _isInitialized = true;
        debugPrint('IRIS: irisInit OK, device_id=$_deviceId');

        // Start services
        _start();
      } else {
        _error = result['error'] as String? ?? 'Unknown error';
      }

      notifyListeners();
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS init error: $e');
      notifyListeners();
    }
  }

  void _start() {
    if (!_isInitialized) return;

    try {
      debugPrint('IRIS: Calling irisStart...');
      final resultJson = irisStart();
      debugPrint('IRIS: irisStart returned: $resultJson');
      final result = jsonDecode(resultJson) as Map<String, dynamic>;
      _isRunning = result['success'] == true;
      if (!_isRunning) {
        _error = result['error'] as String?;
      }
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS start error: $e');
      notifyListeners();
    }
  }

  /// Refresh the list of discovered devices
  void refreshDevices() {
    if (!_isRunning) return;

    try {
      final devicesJson = irisGetDevices();
      final List<dynamic> list = jsonDecode(devicesJson) as List<dynamic>;
      _devices = list
          .map((d) => DeviceInfo.fromJson(d as Map<String, dynamic>))
          .toList();
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS get devices error: $e');
      notifyListeners();
    }
  }

  /// Connect to a remote device
  Future<bool> connectToDevice(String address) async {
    try {
      final resultJson = irisConnect(address: address);
      final result = jsonDecode(resultJson) as Map<String, dynamic>;
      return result['success'] == true;
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS connect error: $e');
      notifyListeners();
      return false;
    }
  }

  /// Test connection to an address without full handshake
  Future<String?> testConnection(String host, int port) async {
    try {
      final socket = await Socket.connect(
        host,
        port,
        timeout: const Duration(seconds: 3),
      );
      socket.destroy();
      return null; // success, no error
    } catch (e) {
      return e.toString();
    }
  }

  /// Restart transports on a specific TCP port
  void restartWithPort(int tcpPort) {
    try {
      final resultJson = irisRestartWithPort(tcpPort);
      debugPrint('IRIS restart: $resultJson');
      _isRunning = true;
      notifyListeners();
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS restart error: $e');
      notifyListeners();
    }
  }

  /// Send encoded frame data to a target via UDP.
  bool udpSend(String target, String encodedB64) {
    try {
      final resultJson = irisUdpSend(target, encodedB64);
      final result = jsonDecode(resultJson) as Map<String, dynamic>;
      return result['success'] == true;
    } catch (e) {
      debugPrint('UDP send error: $e');
      return false;
    }
  }

  /// Get the latest received UDP frame (null if none).
  Map<String, dynamic>? udpReceiveLatest() {
    try {
      final resultJson = irisUdpReceiveLatest();
      final result = jsonDecode(resultJson) as Map<String, dynamic>;
      if (result['data'] != null) return result;
      return null;
    } catch (e) {
      return null;
    }
  }

  /// Capture a frame from the primary display
  Map<String, dynamic>? captureFrame() {
    try {
      final resultJson = irisCaptureFrame();
      return jsonDecode(resultJson) as Map<String, dynamic>;
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS capture error: $e');
      notifyListeners();
      return null;
    }
  }

  /// Encode a raw RGBA frame for sending
  String encodeFrame(List<int> rgbaData, int width, int height,
      {FrameEncoding encoding = FrameEncoding.h264}) {
    return irisEncodeFrame(
      rgbaData: rgbaData,
      width: width,
      height: height,
      encodingType: encoding.value,
    );
  }

  /// Decode received frame data
  Map<String, dynamic>? decodeFrame(String base64Data, int width, int height,
      {FrameEncoding encoding = FrameEncoding.h264}) {
    try {
      final resultJson = irisDecodeFrame(
        base64Data: base64Data,
        width: width,
        height: height,
        encodingType: encoding.value,
      );
      return jsonDecode(resultJson) as Map<String, dynamic>;
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS decode error: $e');
      notifyListeners();
      return null;
    }
  }

  /// Get display dimensions
  Map<String, dynamic>? getDisplayDimensions() {
    try {
      final resultJson = irisDisplayDimensions();
      return jsonDecode(resultJson) as Map<String, dynamic>;
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS display error: $e');
      notifyListeners();
      return null;
    }
  }

  /// Get protocol/version info
  Map<String, dynamic> getVersionInfo() {
    try {
      final json = irisGetVersion();
      return jsonDecode(json) as Map<String, dynamic>;
    } catch (e) {
      return {};
    }
  }

  /// List shareable files
  List<Map<String, dynamic>> listFiles(String dirPath) {
    try {
      final resultJson = irisListFiles(dirPath: dirPath);
      final result = jsonDecode(resultJson) as Map<String, dynamic>;
      if (result['success'] == true) {
        return (result['files'] as List<dynamic>)
            .cast<Map<String, dynamic>>();
      }
      return [];
    } catch (e) {
      _error = e.toString();
      debugPrint('IRIS list files error: $e');
      notifyListeners();
      return [];
    }
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
  final int tcpPort;
  final int udpPort;
  final List<String> services;

  DeviceInfo({
    required this.deviceId,
    required this.deviceName,
    required this.platform,
    required this.ipAddresses,
    this.tcpPort = 21001,
    this.udpPort = 21000,
    this.services = const [],
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
      tcpPort: json['tcp_port'] as int? ?? 21001,
      udpPort: json['udp_port'] as int? ?? 21000,
      services: (json['services'] as List<dynamic>?)
              ?.map((e) => e.toString())
              .toList() ??
          [],
    );
  }

  /// Get an appropriate icon based on platform
  IconData get platformIcon {
    switch (platform.toLowerCase()) {
      case 'windows':
        return Icons.desktop_windows;
      case 'macos':
        return Icons.laptop_mac;
      case 'android':
        return Icons.android;
      case 'linux':
        return Icons.computer;
      default:
        return Icons.devices;
    }
  }

  /// Get the first IP address for display
  String get primaryIp =>
      ipAddresses.isNotEmpty ? ipAddresses.first : 'Unknown';
}
