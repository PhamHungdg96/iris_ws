import 'package:flutter/material.dart';

import '../services/iris_service.dart';

/// Screen for file transfer with a connected device.
/// Lists available files and manages transfer progress.
class FileTransferScreen extends StatefulWidget {
  final DeviceInfo device;

  const FileTransferScreen({super.key, required this.device});

  @override
  State<FileTransferScreen> createState() => _FileTransferScreenState();
}

class _FileTransferScreenState extends State<FileTransferScreen> {
  final List<_TransferItem> _activeTransfers = [];
  bool _isLoadingFiles = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Files - ${widget.device.deviceName}'),
        actions: [
          IconButton(
            icon: const Icon(Icons.folder_open),
            onPressed: _pickAndSendFile,
            tooltip: 'Send file',
          ),
        ],
      ),
      body: Column(
        children: [
          // ── Device Info Bar ──
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(16),
            color: Theme.of(context).colorScheme.surfaceContainerHighest,
            child: Row(
              children: [
                Icon(widget.device.platformIcon),
                const SizedBox(width: 12),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Connected to ${widget.device.deviceName}',
                      style: const TextStyle(fontWeight: FontWeight.w600),
                    ),
                    Text(
                      widget.device.ipAddresses.isNotEmpty
                          ? widget.device.ipAddresses.first
                          : '',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ],
            ),
          ),

          // ── Download Directory Info ──
          ListTile(
            leading: const Icon(Icons.download),
            title: const Text('Received files'),
            subtitle: const Text('Downloads/iris_downloads'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () {
              // Open download directory
            },
          ),
          const Divider(),

          // ── Transfer List ──
          Expanded(
            child: _activeTransfers.isEmpty
                ? Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          Icons.cloud_upload_outlined,
                          size: 64,
                          color: Colors.grey[600],
                        ),
                        const SizedBox(height: 16),
                        Text(
                          'No active transfers',
                          style: TextStyle(
                            color: Colors.grey[500],
                            fontSize: 16,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'Tap the folder icon to send a file',
                          style: TextStyle(
                            color: Colors.grey[600],
                            fontSize: 13,
                          ),
                        ),
                      ],
                    ),
                  )
                : ListView.builder(
                    padding: const EdgeInsets.all(8),
                    itemCount: _activeTransfers.length,
                    itemBuilder: (context, index) {
                      final item = _activeTransfers[index];
                      return _TransferCard(item: item);
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Future<void> _pickAndSendFile() async {
    // In production, use file_picker package:
    // final result = await FilePicker.platform.pickFiles();
    // Then send via TCP using IrisBindings

    // Demo: add a simulated transfer
    setState(() {
      _activeTransfers.add(_TransferItem(
        fileName: 'example_document.pdf',
        totalSize: 2_500_000,
        progress: 0.0,
        status: TransferStatus.transferring,
      ));
    });

    // Simulate progress
    _simulateTransfer(_activeTransfers.last);
  }

  void _simulateTransfer(_TransferItem item) async {
    for (var i = 0; i <= 100; i += 5) {
      await Future.delayed(const Duration(milliseconds: 150));
      if (!mounted) return;
      setState(() {
        item.progress = i / 100.0;
        if (i >= 100) {
          item.status = TransferStatus.completed;
        }
      });
    }
  }
}

enum TransferStatus { transferring, completed, failed }

class _TransferItem {
  final String fileName;
  final int totalSize;
  double progress;
  TransferStatus status;

  _TransferItem({
    required this.fileName,
    required this.totalSize,
    required this.progress,
    required this.status,
  });
}

class _TransferCard extends StatelessWidget {
  final _TransferItem item;

  const _TransferCard({required this.item});

  @override
  Widget build(BuildContext context) {
    final isComplete = item.status == TransferStatus.completed;

    return Card(
      margin: const EdgeInsets.symmetric(vertical: 4),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  isComplete ? Icons.check_circle : Icons.upload_file,
                  color: isComplete ? Colors.green : Colors.blue,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        item.fileName,
                        style: const TextStyle(fontWeight: FontWeight.w500),
                        overflow: TextOverflow.ellipsis,
                      ),
                      Text(
                        _formatSize(item.totalSize),
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                ),
                Text(
                  '${(item.progress * 100).toStringAsFixed(0)}%',
                  style: TextStyle(
                    fontWeight: FontWeight.bold,
                    color: isComplete ? Colors.green : null,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            LinearProgressIndicator(
              value: item.progress,
              backgroundColor: Colors.grey[800],
              color: isComplete ? Colors.green : Colors.blue,
            ),
          ],
        ),
      ),
    );
  }

  String _formatSize(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
  }
}
