import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'services/iris_service.dart';
import 'screens/discovery_screen.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();

  final irisService = IrisService();
  // Initialize IRIS core (async, but we start the UI immediately)
  irisService.initialize();

  runApp(
    ChangeNotifierProvider.value(
      value: irisService,
      child: const IrisApp(),
    ),
  );
}

class IrisApp extends StatelessWidget {
  const IrisApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'IRIS Share',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF6C63FF),
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
        fontFamily: 'Roboto',
      ),
      home: const DiscoveryScreen(),
    );
  }
}
