# frozen_string_literal: true

require_relative 'terminal'
require_relative 'explorer'
require_relative 'models/progress'

chains_file_path = File.join(__dir__, 'chains.json')
progress_file_path = File.join(__dir__, 'progress.json')

progress = Progress.from_json(JSON.parse(File.read(progress_file_path), symbolize_names: true))
chains = JSON.parse(File.read(chains_file_path), symbolize_names: true)

puts chains

$e = if File.exist?(chains_file_path) && File.exist?(progress_file_path)
						Explorer.new(
							evaluations: GDBM.new('evaluations/evaluations.db'),
							efficient_evaluations: GDBM.new('evaluations/efficient_evaluations.db'),
							progress:,
							chains:
						)
					else
						Explorer.new(
							evaluations: GDBM.new('evaluations/evaluations.db'),
							efficient_evaluations: GDBM.new('evaluations/efficient_evaluations.db'),
							progress: Progress.new,
							chains:
						).save
					end

# Print instructions to the terminal
instructions = <<~INSTRUCTIONS

 ============================================
 |                                          |
 |          Welcome to Explore!             |
 |                                          |
 |  Access the explorer via the global      |
 |  variable $e                             |
 |                                          |
 ============================================

INSTRUCTIONS

puts instructions

terminal = InteractiveTerminal.new(
	history_file: '.custom_history',
	max_history: 500,
	prompt: 'ruby> '
)
terminal.start
