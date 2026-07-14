import 'package:flutter/material.dart';

import '../services/iris_service.dart';

/// Screen for viewing a remote device's shared screen.
class ScreenShareScreen extends StatefulWidget {
  final DeviceInfo device;

  const ScreenShareScreen({super.key, required this.device});

  @override
  State<ScreenShareScreen> createState() => _ScreenShareScreenState();
}

class _ScreenShareScreenState extends State<ScreenShareScreen> {
  bool _isReceiving = false;
  String _statusText = 'Connecting...';
  double _fps = 0.0;

  @override
  void initState() {
    super.initState();
    _startReceiving();
  }

  Future<void> _startReceiving() async {
    setState(() {
      _isReceiving = true;
      _statusText = 'Waiting for stream...';
    });

    // In production:
    // 1. Send StartScreenShare via TCP
    // 2. Receive UDP port assignment
    // 3. Bind local UDP socket for incoming frames
    // 4. Reassemble FrameChunks into complete frames
    // 5. Decode and display

    // For now, simulate connection established
    await Future.delayed(const Duration(seconds: 1));
    if (mounted) {
      setState(() {
        _statusText = 'Streaming from ${widget.device.deviceName}';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.black87,
        title: Text(widget.device.deviceName),
        actions: [
          // FPS counter
          Center(
            child: Container(
              margin: const EdgeInsets.only(right: 12),
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color: Colors.green.withOpacity(0.2),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(
                '${_fps.toStringAsFixed(0)} FPS',
                style: const TextStyle(
                  color: Colors.green,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
          // Quality / codec selector
          PopupMenuButton<String>(
            icon: const Icon(Icons.tune),
            tooltip: 'Encoding',
            onSelected: (value) {
              // Switch encoding type
            },
            itemBuilder: (context) => [
              const PopupMenuItem(value: 'h264', child: Text('H.264 (Best)')),
              const PopupMenuItem(value: 'jpeg', child: Text('JPEG (Good)')),
              const PopupMenuItem(value: 'lz4', child: Text('LZ4 (Fast)')),
              const PopupMenuItem(value: 'raw', child: Text('Raw (LAN)')),
            ],
          ),
        ],
      ),
      body: Stack(
        fit: StackFit.expand,
        children: [
          // ── Screen Display Area ──
          // In production: rendered frame from decoded RGBA data
          Center(
            child: Container(
              decoration: BoxDecoration(
                color: Colors.grey[900],
                border: Border.all(
                  color: Colors.grey[800]!,
                  width: 0.5,
                ),
              ),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    Icons.desktop_windows,
                    size: 80,
                    color: Colors.grey[700],
                  ),
                  const SizedBox(height: 16),
                  Text(
                    _isReceiving ? _statusText : 'Stream paused',
                    style: TextStyle(
                      color: Colors.grey[500],
                      fontSize: 16,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Screen sharing from ${widget.device.deviceName}',
                    style: TextStyle(
                      color: Colors.grey[600],
                      fontSize: 13,
                    ),
                  ),
                ],
              ),
            ),
          ),

          // ── Control overlay (bottom) ──
          Positioned(
            bottom: 0,
            left: 0,
            right: 0,
            child: Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.bottomCenter,
                  end: Alignment.topCenter,
                  colors: [
                    Colors.black.withOpacity(0.8),
                    Colors.transparent,
                  ],
                ),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  // Pause/Resume
                  IconButton.filled(
                    onPressed: _togglePause,
                    icon: Icon(
                      _isReceiving ? Icons.pause : Icons.play_arrow,
                    ),
                    style: IconButton.styleFrom(
                      backgroundColor:
                          Theme.of(context).colorScheme.primary,
                    ),
                  ),
                  const SizedBox(width: 16),
                  // Request keyframe
                  IconButton.filled(
                    onPressed: () {
                      // Send KeyFrameRequest via UDP
                    },
                    icon: const Icon(Icons.refresh),
                    tooltip: 'Request keyframe',
                  ),
                  const SizedBox(width: 16),
                  // Fullscreen
                  IconButton.filled(
                    onPressed: () {
                      // Enter fullscreen mode
                    },
                    icon: const Icon(Icons.fullscreen),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  void _togglePause() {
    setState(() {
      _isReceiving = !_isReceiving;
      _statusText = _isReceiving
          ? 'Streaming from ${widget.device.deviceName}'
          : 'Paused';
    });
  }
}
