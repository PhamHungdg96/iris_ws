import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/iris_service.dart';
import '../widgets/device_tile.dart';
import 'screen_share_screen.dart';
import 'file_transfer_screen.dart';

/// Main screen: device list + port configuration
class DiscoveryScreen extends StatefulWidget {
  const DiscoveryScreen({super.key});

  @override
  State<DiscoveryScreen> createState() => _DiscoveryScreenState();
}

class _DiscoveryScreenState extends State<DiscoveryScreen> {
  Timer? _refreshTimer;
  final _portController = TextEditingController(text: '21001');

  @override
  void initState() {
    super.initState();
    _refreshTimer = Timer.periodic(
      const Duration(seconds: 3),
      (_) => _refreshDevices(),
    );
    WidgetsBinding.instance.addPostFrameCallback((_) => _refreshDevices());
  }

  void _refreshDevices() {
    context.read<IrisService>().refreshDevices();
  }

  @override
  void dispose() {
    _refreshTimer?.cancel();
    _portController.dispose();
    super.dispose();
  }

  void _applyPort() {
    final port = int.tryParse(_portController.text.trim());
    if (port != null && port > 0 && port < 65536) {
      context.read<IrisService>().restartWithPort(port);
    }
  }

  void _showDeviceActions(DeviceInfo device) {
    showModalBottomSheet(
      context: context,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Center(
              child: Container(
                width: 40, height: 4,
                decoration: BoxDecoration(
                    color: Colors.grey[600],
                    borderRadius: BorderRadius.circular(2)),
              ),
            ),
            const SizedBox(height: 20),
            Row(children: [
              Icon(device.platformIcon, size: 28),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(device.deviceName,
                        style: const TextStyle(
                            fontSize: 18, fontWeight: FontWeight.bold)),
                    Text('${device.primaryIp}:${device.tcpPort}',
                        style: TextStyle(color: Colors.grey[400])),
                  ],
                ),
              ),
            ]),
            const SizedBox(height: 24),
            const Divider(),
            const SizedBox(height: 8),
            ListTile(
              leading: const Icon(Icons.screen_share),
              title: const Text('View Screen'),
              subtitle: const Text('Watch remote screen via UDP stream'),
              onTap: () {
                Navigator.pop(ctx);
                _connectAndNavigate(device, ScreenShareScreen(device: device));
              },
            ),
            ListTile(
              leading: const Icon(Icons.folder),
              title: const Text('Share Files'),
              subtitle: const Text('Transfer files via TCP'),
              onTap: () {
                Navigator.pop(ctx);
                _connectAndNavigate(device, FileTransferScreen(device: device));
              },
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }

  Future<void> _connectAndNavigate(DeviceInfo device, Widget screen) async {
    final service = context.read<IrisService>();
    final address = '${device.ipAddresses.first}:${device.tcpPort}';
    final connected = await service.connectToDevice(address);
    if (!mounted) return;
    if (connected) {
      Navigator.push(context, MaterialPageRoute(builder: (_) => screen));
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Failed to connect to ${device.deviceName}'),
          backgroundColor: Colors.red,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      body: CustomScrollView(
        slivers: [
          // ── App Bar ──
          SliverAppBar(
            expandedHeight: 200,
            floating: false,
            pinned: true,
            flexibleSpace: FlexibleSpaceBar(
              title: const Text('IRIS Share',
                  style: TextStyle(fontWeight: FontWeight.bold)),
              background: Container(
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                    colors: [theme.colorScheme.primary, theme.colorScheme.tertiary],
                  ),
                ),
                child: Center(
                  child: Icon(Icons.share, size: 64,
                      color: Colors.white.withValues(alpha: 0.3)),
                ),
              ),
            ),
          ),

          // ── This Device + Port Config ──
          SliverToBoxAdapter(
            child: Consumer<IrisService>(
              builder: (context, service, _) {
                if (!service.isInitialized) {
                  return const Padding(
                    padding: EdgeInsets.all(32),
                    child: Center(child: CircularProgressIndicator()),
                  );
                }
                return Card(
                  margin: const EdgeInsets.all(16),
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(children: [
                          Icon(Icons.desktop_windows,
                              color: theme.colorScheme.primary),
                          const SizedBox(width: 12),
                          const Text('This Device',
                              style: TextStyle(
                                  fontSize: 16, fontWeight: FontWeight.w600)),
                          const Spacer(),
                          _statusBadge(service.isRunning),
                        ]),
                        const SizedBox(height: 8),
                        Text('ID: ${service.deviceId}',
                            style: theme.textTheme.bodySmall),
                        const SizedBox(height: 12),
                        // Port configuration
                        Row(children: [
                          const Icon(Icons.settings_ethernet, size: 18,
                              color: Colors.grey),
                          const SizedBox(width: 8),
                          const Text('TCP Port:',
                              style: TextStyle(fontSize: 13)),
                          const SizedBox(width: 8),
                          SizedBox(
                            width: 80,
                            child: TextField(
                              controller: _portController,
                              decoration: const InputDecoration(
                                border: OutlineInputBorder(),
                                isDense: true,
                                contentPadding: EdgeInsets.symmetric(
                                    horizontal: 8, vertical: 8),
                              ),
                              keyboardType: TextInputType.number,
                              style: const TextStyle(fontSize: 13),
                            ),
                          ),
                          const SizedBox(width: 8),
                          SizedBox(
                            height: 32,
                            child: FilledButton.icon(
                              onPressed: _applyPort,
                              icon: const Icon(Icons.refresh, size: 16),
                              label: const Text('Apply', style: TextStyle(fontSize: 12)),
                              style: FilledButton.styleFrom(
                                padding: const EdgeInsets.symmetric(horizontal: 12),
                              ),
                            ),
                          ),
                        ]),
                      ],
                    ),
                  ),
                );
              },
            ),
          ),

          // ── Discovered Devices Header ──
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
              child: Row(children: [
                Icon(Icons.devices, size: 20, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Discovered Devices',
                    style: theme.textTheme.titleMedium
                        ?.copyWith(fontWeight: FontWeight.w600)),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.refresh, size: 20),
                  onPressed: _refreshDevices,
                  tooltip: 'Refresh',
                ),
              ]),
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
                      child: Column(children: [
                        Icon(Icons.search_off, size: 48,
                            color: Colors.grey[600]),
                        const SizedBox(height: 16),
                        Text('No devices found',
                            style: TextStyle(
                                color: Colors.grey[500], fontSize: 16)),
                        const SizedBox(height: 8),
                        Text(
                          'Run another IRIS instance on the same network',
                          textAlign: TextAlign.center,
                          style: TextStyle(color: Colors.grey[600], fontSize: 13),
                        ),
                      ]),
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
          const SliverToBoxAdapter(child: SizedBox(height: 80)),
        ],
      ),
    );
  }

  Widget _statusBadge(bool online) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: online
            ? Colors.green.withValues(alpha: 0.2)
            : Colors.orange.withValues(alpha: 0.2),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Text(
        online ? 'Online' : 'Starting...',
        style: TextStyle(
          color: online ? Colors.green : Colors.orange,
          fontSize: 12,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}
