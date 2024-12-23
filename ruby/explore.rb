# frozen_string_literal: true

require 'debug/open_nonstop'
require_relative 'terminal'
require_relative 'models/explorer'
require_relative 'models/progress'

chains_file_path = File.join(__dir__, 'chains.json')
progress_file_path = File.join(__dir__, 'progress.json')

progress = Progress.from_json(JSON.parse(File.read(progress_file_path), symbolize_names: true))
chains = JSON.parse(File.read(chains_file_path), symbolize_names: true)

# puts chains

$e = if File.exist?(chains_file_path) && File.exist?(progress_file_path)
						Explorer.new(
							progress:,
							chains:
						)
					else
						Explorer.new(
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
