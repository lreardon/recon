import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';

/// Root of the recon project. Override with the PROJECT_ROOT environment
/// variable when launching; defaults to the standard checkout location.
final String projectRoot =
    Platform.environment['PROJECT_ROOT'] ?? '/Users/landho/git/recon';

final String releaseDir = '$projectRoot/rust/evaluations/target/release';

void main() {
  runApp(const DashboardApp());
}

class DashboardApp extends StatelessWidget {
  const DashboardApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Recon Dashboard',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorSchemeSeed: Colors.indigo,
        brightness: Brightness.light,
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        colorSchemeSeed: Colors.indigo,
        brightness: Brightness.dark,
        useMaterial3: true,
      ),
      home: const DashboardPage(),
    );
  }
}

class DashboardPage extends StatelessWidget {
  const DashboardPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Recon Dashboard')),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 760),
          child: ListView(
            padding: const EdgeInsets.all(16),
            children: const [
              ProgressCard(),
              SizedBox(height: 16),
              EvaluateCard(),
              SizedBox(height: 16),
              MinFormCard(),
            ],
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Progress
// ---------------------------------------------------------------------------

class ProgressCard extends StatefulWidget {
  const ProgressCard({super.key});

  @override
  State<ProgressCard> createState() => _ProgressCardState();
}

class _ProgressCardState extends State<ProgressCard> {
  Map<String, dynamic>? _progress;
  Map<String, dynamic>? _lengthProgress;
  Map<String, dynamic>? _manifest;
  DateTime? _progressModified;

  /// Largest N such that every value in [0, N) has a definitively minimal form
  /// (its shortest known form is within the exhaustive length watermark).
  int? _minimalCertainBelow;
  String? _error;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _refresh();
    _timer = Timer.periodic(const Duration(seconds: 2), (_) => _refresh());
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _refresh() async {
    try {
      final progressFile = File('$projectRoot/state/progress.json');
      final lengthFile = File('$projectRoot/state/length_progress.json');
      final manifestFile = File('$projectRoot/fst/MANIFEST.json');

      final progress = progressFile.existsSync()
          ? jsonDecode(await progressFile.readAsString())
          : null;
      final lengthProgress = lengthFile.existsSync()
          ? jsonDecode(await lengthFile.readAsString())
          : null;
      final manifest = manifestFile.existsSync()
          ? jsonDecode(await manifestFile.readAsString())
          : null;
      final newestModified = [progressFile, lengthFile]
          .where((f) => f.existsSync())
          .map((f) => f.lastModifiedSync())
          .fold<DateTime?>(
              null, (a, b) => a == null || b.isAfter(a) ? b : a);

      final watermark =
          (lengthProgress?['complete_through_length'] as num?)?.toInt();
      final minimalCertainBelow = watermark == null
          ? null
          : await _minimalCertainBelowValue(watermark);

      if (!mounted) return;
      setState(() {
        _progress = progress as Map<String, dynamic>?;
        _lengthProgress = lengthProgress as Map<String, dynamic>?;
        _manifest = manifest as Map<String, dynamic>?;
        _progressModified = newestModified;
        _minimalCertainBelow = minimalCertainBelow;
        _error = null;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = e.toString());
    }
  }

  /// Scan the shortest-known-form index and return the largest N such that
  /// every value in [0, N) has a shortest known form no longer than the
  /// exhaustive watermark — i.e. a form that is provably minimal. The first
  /// value that is either missing or only known through a longer (provisional)
  /// form marks the boundary.
  Future<int?> _minimalCertainBelowValue(int watermark) async {
    final file = File('$projectRoot/fst/shortest_by_value.tsv');
    if (!file.existsSync()) return null;
    final lengthByValue = <int, int>{};
    for (final line in await file.readAsLines()) {
      final tab = line.indexOf('\t');
      if (tab < 0) continue;
      final value = int.tryParse(line.substring(0, tab));
      if (value == null) continue;
      // Expression is the ASCII text after the tab; its char count is its length.
      lengthByValue[value] = line.length - tab - 1;
    }
    var n = 0;
    while (true) {
      final len = lengthByValue[n];
      if (len == null || len > watermark) break;
      n++;
    }
    return n;
  }

  String _phaseLabel(String? phase) {
    switch (phase) {
      case 'exploring_ready':
        return 'ready to explore';
      case 'exploring_new_nullaries_all_unaries':
        return 'phase 1: new nullaries × all unaries';
      case 'exploring_old_nullaries_new_unaries':
        return 'phase 2: old nullaries × new unaries';
      default:
        return phase ?? 'unknown';
    }
  }

  @override
  Widget build(BuildContext context) {
    final progress = _progress;
    final evaluations = _manifest?['evaluations'] as Map<String, dynamic>?;
    final unaries = _manifest?['unaries'] as Map<String, dynamic>?;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Text('Exploration progress',
                    style: Theme.of(context).textTheme.titleMedium),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.refresh, size: 20),
                  tooltip: 'Refresh now',
                  onPressed: _refresh,
                ),
              ],
            ),
            const SizedBox(height: 8),
            if (_error != null)
              Text(_error!,
                  style: TextStyle(
                      color: Theme.of(context).colorScheme.error))
            else if (progress == null && _lengthProgress == null)
              const Text('No exploration state yet — run `recon` to start.')
            else
              Wrap(
                spacing: 32,
                runSpacing: 12,
                children: [
                  if (_lengthProgress != null) ...[
                    _stat('Length sweep',
                        'stratum ${_lengthProgress!['length']} · item ${_lengthProgress!['item']}'),
                    _stat(
                        'All forms found below length',
                        '${((_lengthProgress!['complete_through_length'] as num).toInt()) + 1}',
                        tooltip:
                            'Largest N such that every form of length less than N '
                            'has been enumerated.'),
                  ],
                  if (_minimalCertainBelow != null)
                    _stat('Minimal form proven below', '$_minimalCertainBelow',
                        tooltip:
                            'Largest N such that every value less than N has a '
                            'definitively minimal form (its shortest known form '
                            'is within the exhaustive length watermark).'),
                  if (progress != null) ...[
                    _stat('Depth mode', 'depth ${progress['depth']}'),
                    _stat('Depth phase', _phaseLabel(
                        progress['exploration_phase'] as String?)),
                  ],
                  if (evaluations != null)
                    _stat('Evaluations', '${evaluations['entries']}'),
                  if (unaries != null)
                    _stat('Unaries', '${unaries['entries']}'),
                  if (_progressModified != null)
                    _stat('Last commit',
                        _progressModified!.toLocal().toString().substring(0, 19)),
                ],
              ),
          ],
        ),
      ),
    );
  }

  Widget _stat(String label, String value, {String? tooltip}) {
    final column = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label,
            style: Theme.of(context)
                .textTheme
                .labelSmall
                ?.copyWith(color: Theme.of(context).colorScheme.outline)),
        Text(value, style: Theme.of(context).textTheme.bodyLarge),
      ],
    );
    if (tooltip == null) return column;
    return Tooltip(message: tooltip, child: column);
  }
}

// ---------------------------------------------------------------------------
// Shared: a query box that runs a binary and shows the result
// ---------------------------------------------------------------------------

class QueryResult {
  final String text;
  final bool isError;
  const QueryResult(this.text, {this.isError = false});
}

Future<QueryResult> runTool(String binary, List<String> args) async {
  final path = '$releaseDir/$binary';
  if (!File(path).existsSync()) {
    return QueryResult(
      'Binary not found: $path\nBuild it with: cargo build --release',
      isError: true,
    );
  }
  try {
    final result = await Process.run(
      path,
      args,
      environment: {'PROJECT_ROOT': projectRoot},
    );
    final stdout = (result.stdout as String).trim();
    final stderr = (result.stderr as String).trim();
    if (result.exitCode != 0) {
      return QueryResult(stderr.isEmpty ? 'No result.' : stderr,
          isError: true);
    }
    return QueryResult(stdout);
  } catch (e) {
    return QueryResult(e.toString(), isError: true);
  }
}

class QueryCard extends StatefulWidget {
  final String title;
  final String hint;
  final String buttonLabel;
  final Future<QueryResult> Function(String input) run;

  const QueryCard({
    super.key,
    required this.title,
    required this.hint,
    required this.buttonLabel,
    required this.run,
  });

  @override
  State<QueryCard> createState() => _QueryCardState();
}

class _QueryCardState extends State<QueryCard> {
  final _controller = TextEditingController();
  QueryResult? _result;
  bool _busy = false;

  Future<void> _submit() async {
    final input = _controller.text.trim();
    if (input.isEmpty || _busy) return;
    setState(() => _busy = true);
    final result = await widget.run(input);
    if (!mounted) return;
    setState(() {
      _result = result;
      _busy = false;
    });
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final result = _result;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(widget.title,
                style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: TextField(
                    controller: _controller,
                    style: const TextStyle(fontFamily: 'Menlo', fontSize: 14),
                    decoration: InputDecoration(
                      hintText: widget.hint,
                      border: const OutlineInputBorder(),
                      isDense: true,
                    ),
                    onSubmitted: (_) => _submit(),
                  ),
                ),
                const SizedBox(width: 12),
                FilledButton(
                  onPressed: _busy ? null : _submit,
                  child: _busy
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2))
                      : Text(widget.buttonLabel),
                ),
              ],
            ),
            if (result != null) ...[
              const SizedBox(height: 12),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: Theme.of(context)
                      .colorScheme
                      .surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: SelectableText(
                  result.text,
                  style: TextStyle(
                    fontFamily: 'Menlo',
                    fontSize: 14,
                    color: result.isError
                        ? Theme.of(context).colorScheme.error
                        : null,
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// The two query boxes
// ---------------------------------------------------------------------------

class EvaluateCard extends StatelessWidget {
  const EvaluateCard({super.key});

  @override
  Widget build(BuildContext context) {
    return QueryCard(
      title: 'Evaluate a form',
      hint: 'e.g. R(^(#),0,^(^(0)))',
      buttonLabel: 'Evaluate',
      run: (input) => runTool('evaluate_recursive_form', [input]),
    );
  }
}

class MinFormCard extends StatelessWidget {
  const MinFormCard({super.key});

  @override
  Widget build(BuildContext context) {
    return QueryCard(
      title: 'Minimum form for a value',
      hint: 'e.g. 5',
      buttonLabel: 'Look up',
      run: (input) async {
        if (int.tryParse(input) == null) {
          return const QueryResult('Enter a non-negative integer.',
              isError: true);
        }
        final result = await runTool('min_form', [input, '-v']);
        if (result.isError) {
          if (result.text == 'No result.') {
            return const QueryResult(
                'No known form evaluates to this value yet.',
                isError: true);
          }
          return result;
        }
        return QueryResult(_formatMinForm(result.text));
      },
    );
  }
}

/// Format `min_form -v` output — a tab-separated
/// `expression \t length \t status (exhaustive through length N)` line — into a
/// readable block that states whether the form is definitively minimal or only
/// the shortest found so far.
String _formatMinForm(String raw) {
  final parts = raw.split('\t');
  if (parts.length < 3) return raw;
  final expression = parts[0];
  final length = parts[1];
  final status = parts[2]; // e.g. "definitive (exhaustive through length 50)"

  String verdict;
  String? note = RegExp(r'exhaustive through length \d+').firstMatch(status)?.group(0);
  if (status.startsWith('definitive')) {
    verdict = '✓ Definitively minimal';
  } else if (status.startsWith('provisional')) {
    verdict = '≈ Minimal found so far (not yet proven minimal)';
  } else {
    // "conditional (...)" or anything unexpected — show the raw detail as-is.
    verdict = status;
    note = null;
  }

  final buffer = StringBuffer()
    ..writeln(expression)
    ..writeln()
    ..write('length $length · $verdict');
  if (note != null) buffer.write('\n$note');
  return buffer.toString();
}
