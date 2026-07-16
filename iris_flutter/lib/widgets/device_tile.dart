import 'package:flutter/material.dart';

import '../services/iris_service.dart';

/// Widget displaying a discovered device with IP, port, platform, and services
class DeviceTile extends StatelessWidget {
  final DeviceInfo device;
  final VoidCallback? onTap;

  const DeviceTile({super.key, required this.device, this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Row(
            children: [
              // Platform icon
              Container(
                width: 44, height: 44,
                decoration: BoxDecoration(
                  color: theme.colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(device.platformIcon, size: 24,
                    color: theme.colorScheme.onPrimaryContainer),
              ),
              const SizedBox(width: 14),
              // Info
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(device.deviceName,
                        style: const TextStyle(
                            fontSize: 15, fontWeight: FontWeight.w600)),
                    const SizedBox(height: 4),
                    Row(children: [
                      // IP:port
                      Icon(Icons.lan, size: 13, color: Colors.grey[500]),
                      const SizedBox(width: 4),
                      Flexible(
                        child: Text(
                          '${device.primaryIp}:${device.tcpPort}',
                          style: TextStyle(
                              fontSize: 12, color: Colors.grey[500]),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ]),
                    const SizedBox(height: 3),
                    // Platform + services
                    Wrap(
                      spacing: 6,
                      runSpacing: 3,
                      children: [
                        _chip(device.platform, theme.colorScheme.secondary),
                        for (final svc in device.services)
                          _chip(_serviceLabel(svc), theme.colorScheme.tertiary),
                      ],
                    ),
                  ],
                ),
              ),
              Icon(Icons.chevron_right, color: Colors.grey[600], size: 22),
            ],
          ),
        ),
      ),
    );
  }

  String _serviceLabel(String svc) {
    switch (svc) {
      case 'screen_share': return 'Screen';
      case 'file_transfer': return 'Files';
      default: return svc;
    }
  }

  Widget _chip(String label, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(label,
          style: TextStyle(
              fontSize: 10, color: color, fontWeight: FontWeight.w500)),
    );
  }
}
