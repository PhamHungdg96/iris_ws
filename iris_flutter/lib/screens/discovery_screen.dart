import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/iris_service.dart';
import '../widgets/device_tile.dart';
import 'screen_share_screen.dart';
import 'file_transfer_screen.dart';

/// Main screen: shows discovered LAN devices and connection options
class DiscoveryScreen extends StatefulWidget {
  const DiscoveryScreen({super.key});

  @override
  State<DiscoveryScreen> createState() => _DiscoveryScreenState();
}

class _DiscoveryScreenState extends State<DiscoveryScreen> {
  Timer? _refreshTimer;

  @override
  void initState() {
    super.initState();
    // Auto-refresh device list every 3 seconds
    _refreshTimer = Timer.periodic(
      const Duration(seconds: 3),
      (_) => _refreshDevices(),
    );
    // Initial refresh after build
    WidgetsBinding.instance.addPostFrameCallback((_) => _refreshDevices());
  }

  void _refreshDevices() {
    context.read<IrisService>().refreshDevices();
  }

  @override
  void dispose() {
    _refreshTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: CustomScrollView(
        slivers: [
          // ── App Bar ──
          SliverAppBar(
            expandedHeight: 200,
            floating: false,
            pinned: true,
            flexibleSpace: FlexibleSpaceBar(
              title: const Text(
                'IRIS Share',
                style: TextStyle(fontWeight: FontWeight.bold),
              ),
              background: Container(
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                    colors: [
                      Theme.of(context).colorScheme.primary,
                      Theme.of(context).colorScheme.tertiary,
                    ],
                  ),
                ),
                child: Center(
                  child: Icon(
                    Icons.share,
                    size: 64,
                    color: Colors.white.withOpacity(0.3),
                  ),
                ),
              ),
            ),
          ),

          // ── This Device Info ──
          SliverToBoxAdapter(
            child: Consumer<IrisService>(
              builder: (context, service, _) {
                if (!service.isInitialized) {
                  return const Padding(
                    padding: EdgeInsets.all(32),
                    child: Center(child: CircularProgressIndicator()),
                  );
                }

                if (service.error != null) {
                  return Padding(
                    padding: const EdgeInsets.all(16),
                    child: Card(
                      color: Theme.of(context).colorScheme.errorContainer,
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Row(
                          children: [
                            const Icon(Icons.warning_amber),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Text(
                                service.error!,
                                style: TextStyle(
                                  color: Theme.of(context)
                                      .colorScheme
                                      .onErrorContainer,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  );
                }

                return Card(
                  margin: const EdgeInsets.all(16),
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Icon(
                              Icons.desktop_windows,
                              color: Theme.of(context).colorScheme.primary,
                            ),
                            const SizedBox(width: 12),
                            const Text(
                              'This Device',
                              style: TextStyle(
                                fontSize: 16,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                            const Spacer(),
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 10,
                                vertical: 4,
                              ),
                              decoration: BoxDecoration(
                                color: service.isRunning
                                    ? Colors.green.withOpacity(0.2)
                                    : Colors.orange.withOpacity(0.2),
                                borderRadius: BorderRadius.circular(12),
                              ),
                              child: Text(
                                service.isRunning ? 'Online' : 'Starting...',
                                style: TextStyle(
                                  color: service.isRunning
                                      ? Colors.green
                                      : Colors.orange,
                                  fontSize: 12,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'ID: ${service.deviceId}',
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
          ),

          // ── Section Header ──
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Row(
                children: [
                  Icon(
                    Icons.devices,
                    size: 20,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                  const SizedBox(width: 8),
                  Text(
                    'Discovered Devices',
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                  ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.refresh, size: 20),
                    onPressed: _refreshDevices,
                    tooltip: 'Refresh',
                  ),
                ],
              ),
            ),
          ),

          // ── Device List ──
          Consumer<IrisService>(
            builder: (context, service, _) {
              final devices = service.devices;

              if (devices.isEmpty) {
                return SliverToBoxAdapter(
                  child: Padding(
                    padding: const EdgeInsets.all(48),
                    child: Center(
                      child: Column(
                        children: [
                          Icon(
                            Icons.search_off,
                            size: 48,
                            color: Colors.grey[600],
                          ),
                          const SizedBox(height: 16),
                          Text(
                            'No devices found',
                            style: TextStyle(
                              color: Colors.grey[500],
                              fontSize: 16,
                            ),
                          ),
                          const SizedBox(height: 8),
                          Text(
                            'Make sure other IRIS instances are running\non the same network',
                            textAlign: TextAlign.center,
                            style: TextStyle(
                              color: Colors.grey[600],
                              fontSize: 13,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                );
              }

              return SliverList(
                delegate: SliverChildBuilderDelegate(
                  (context, index) {
                    final device = devices[index];
                    return DeviceTile(
                      device: device,
                      onTap: () => _showDeviceActions(device),
                    );
                  },
                  childCount: devices.length,
                ),
              );
            },
          ),

          // Bottom padding
          const SliverToBoxAdapter(
            child: SizedBox(height: 80),
          ),
        ],
      ),
    );
  }

  void _showDeviceActions(DeviceInfo device) {
    showModalBottomSheet(
      context: context,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (context) => Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Center(
              child: Container(
                width: 40,
                height: 4,
                decoration: BoxDecoration(
                  color: Colors.grey[600],
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ),
            const SizedBox(height: 20),
            Row(
              children: [
                Icon(device.platformIcon, size: 28),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        device.deviceName,
                        style: const TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      Text(
                        device.ipAddresses.isNotEmpty
                            ? device.ipAddresses.first
                            : 'No address',
                        style: TextStyle(color: Colors.grey[400]),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 24),
            const Divider(),
            const SizedBox(height: 8),

            // Screen Share option
            ListTile(
              leading: const Icon(Icons.screen_share),
              title: const Text('View Screen'),
              subtitle: const Text('Watch remote screen via UDP stream'),
              onTap: () {
                Navigator.pop(context);
                _connectAndNavigate(
                  device,
                  ScreenShareScreen(device: device),
                );
              },
            ),

            // File Transfer option
            ListTile(
              leading: const Icon(Icons.folder),
              title: const Text('Share Files'),
              subtitle: const Text('Transfer files via TCP'),
              onTap: () {
                Navigator.pop(context);
                _connectAndNavigate(
                  device,
                  FileTransferScreen(device: device),
                );
              },
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }

  Future<void> _connectAndNavigate(
      DeviceInfo device, Widget screen) async {
    final service = context.read<IrisService>();
    final address =
        '${device.ipAddresses.first}:21001'; // TCP control port

    final connected = await service.connectToDevice(address);

    if (connected && mounted) {
      Navigator.push(
        context,
        MaterialPageRoute(builder: (_) => screen),
      );
    } else if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Failed to connect to ${device.deviceName}'),
          backgroundColor: Colors.red,
        ),
      );
    }
  }
}
