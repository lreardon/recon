# frozen_string_literal: true

require 'readline'
require 'reline'
require 'io/console'
require 'colorize' # For syntax highlighting
require 'coderay'  # For more advanced syntax highlighting

class InteractiveTerminal
	def initialize(options = {})
		@history_file = options[:history_file] || File.join(Dir.home, 'models/terminal/.command_history')
		@max_history = options[:max_history] || 1000
		@prompt = options[:prompt] || '> '
		@binding = binding

		# Initialize our environment
		setup_completion
		load_history
		setup_signal_handlers
	end

	def setup_completion
		# Set up tab completion for Ruby keywords, methods, and local variables
		Reline.completion_proc = proc do |word|
			# Get local variables
			local_vars = @binding.local_variables.map(&:to_s)

			# Get Ruby keywords
			ruby_keywords = %w[class def end if else elsif unless case when while until do for break next]

			# Get methods from common classes
			common_methods = Object.methods + String.instance_methods

			# Combine all possible completions and filter based on the current word
			completions = (local_vars + ruby_keywords + common_methods.map(&:to_s)).uniq
			completions.grep(/^#{Regexp.escape(word)}/)
		end

		# Enable multiline editing
		# Reline.auto_indent = true
	end

	def load_history
		return unless File.exist?(@history_file)

		File.readlines(@history_file).each do |line|
			Readline::HISTORY.push(line.chomp)
		end
	end

	def save_history
		# Save only the last @max_history commands
		history = Readline::HISTORY.to_a.last(@max_history)
		File.write(@history_file, history.join("\n"))
	end

	def setup_signal_handlers
		# Handle Ctrl+C gracefully
		Signal.trap('INT') do
			puts "\nUse 'exit' to quit"
			print @prompt
		end
	end

	def highlight_syntax(code)
		# Use CodeRay for syntax highlighting
		CodeRay.scan(code, :ruby).terminal
	rescue StandardError
		# Fallback to simple string if highlighting fails
		code
	end

	def start
		# puts 'Welcome to Interactive Ruby Terminal!'
		# puts 'Features available:'
		# puts '  • Arrow keys for navigation'
		# puts '  • Command history (persisted across sessions)'
		# puts '  • Tab completion for Ruby keywords and methods'
		# puts '  • Syntax highlighting'
		# # puts '  • Multi-line editing (press Enter twice to execute)'
		# puts '  • Ctrl+C to cancel current input'
		# puts "Type 'exit' or 'quit' to end the session"
		# puts

		# Main REPL loop
		loop do
			# Read
			input = Reline.readline(@prompt, true)
			break if input.nil? || %w[exit quit].include?(input.downcase.strip)

			# Highlight the input for better readability
			puts highlight_syntax(input)

			# Evaluate
			begin
				result = @binding.eval(input)
				# Print with color formatting
				puts "=> #{highlight_syntax(result)}"
			rescue Exception => e
				puts "Error: #{e.message}".red
				puts e.backtrace.first(5).join("\n").red if e.backtrace
			end
		end

		# Clean up
		save_history
		puts "\nGoodbye! Terminal session ended."
	end
end
