# frozen_string_literal: true

require 'debug/open_nonstop'
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/models/terminal/interactive_terminal"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/models/explorer"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/models/progress"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/scripts/evaluations_fst"

chains_file_path = "#{ENV.fetch('PROJECT_ROOT')}/ruby/models/chains.json"
progress_file_path = "#{ENV.fetch('PROJECT_ROOT')}/ruby/models/progress.json"

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

# puts "Loading evaluations..."
# Benchmark.benchmark(CAPTION, 40, FORMAT) do |x|
# 	x.report('Loaded Evaluations') {load_evaluations_fst}
# end
# puts "Done"

puts instructions

terminal = InteractiveTerminal.new(
	history_file: "#{ENV.fetch('PROJECT_ROOT')}/ruby/models/terminal/.command_history",
	max_history: 500,
	prompt: 'ruby> '
)

trap("SIGINT") do
  puts "Cleaning up..."
  # Place your cleanup code here
  exit
end

terminal.start
