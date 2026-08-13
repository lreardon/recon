import 'package:flutter_test/flutter_test.dart';

import 'package:recon_dashboard/main.dart';

void main() {
  testWidgets('dashboard renders its three cards', (tester) async {
    await tester.pumpWidget(const DashboardApp());

    expect(find.text('Exploration progress'), findsOneWidget);
    expect(find.text('Evaluate a form'), findsOneWidget);
    expect(find.text('Minimum form for a value'), findsOneWidget);
  });
}
