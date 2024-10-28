# frozen_string_literal: true

require 'gdbm'
require 'json'
require 'gruff'
require 'sfst'

require_relative 'errors/malformed_nullary_string_error'
require_relative 'models/progress'

# A class for building new unary and nullary expressions.
class Explorer
	attr_accessor :depth
	attr_reader :nullaries_chain, :unaries_chain, :evaluations

	SAVE_MODULUS = 5000

	EXPLORING_READY = 0
	EXPLORING_NULLARIES__NEW_NULLARIES__ALL_UNARIES = 1
	EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES = 2
	EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES = 3
	EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES = 4
	EXPLORING_DONE = 5

	def initialize(
		evaluations:,
		efficient_evaluations:,
		progress:,
		chains:
	)
		@depth = progress.depth
		@exploration_state = EXPLORING_READY
		@nullary_index = progress.nullary_index
		@unary_index = progress.unary_index

		@nullaries_chain = chains[:nullaries]
		@unaries_chain = chains[:unaries]

		@evaluations = evaluations
		@evaluations_compiler = SFST::Compiler.new
		@efficient_evaluations = efficient_evaluations
		@save_ticker = 0
	end

	def nullaries
		@nullaries_chain.flatten
	end

	def save_progress
		progress_json = {
			depth: @depth,
			nullary_index: @nullary_index,
			unary_index: @unary_index
		}

		File.write(File.join(__dir__, 'progress.json'), progress_json.to_json)
	end

	def unaries
		@unaries_chain.flatten
	end

	def explore
		@save_ticker = 0
		new_nullaries = @nullaries_chain.last
		old_nullaries = @nullaries_chain[0..-2].flatten

		new_unaries = @unaries_chain.last
		# old_unaries = @unaries_chain[0..-2].flatten

		while @exploration_state != EXPLORING_DONE
			if @exploration_state == EXPLORING_READY
				@exploration_state = EXPLORING_NULLARIES__NEW_NULLARIES__ALL_UNARIES
				next
			end

			if @exploration_state == EXPLORING_NULLARIES__NEW_NULLARIES__ALL_UNARIES
				new_nullaries[@nullary_index..].each do |nullary|
					unaries[@unary_index..].each do |unary|
						commit_resulting_nullary_if_new(unary, nullary)
						maybe_save_progress

						@unary_index += 1
					end
					@nullary_index += 1
				end
				@exploration_state = EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES
				@nullary_index = 0
				@unary_index = 0
				next
			end

			if @exploration_state == EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES
				old_nullaries[@nullary_index..].each do |nullary|
					new_unaries[@unary_index..].each do |unary|
						commit_resulting_nullary_if_new(unary, nullary)
						maybe_save_progress

						@unary_index += 1
					end
					@nullary_index += 1
				end
				@exploration_state = EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES
				@nullary_index = 0
				@unary_index = 0
				next

			end

			if @exploration_state == EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES
				new_nullaries[@nullary_index..].each do |nullary|
					unaries[@unary_index..].each do |unary|
						commit_resulting_unaries_if_new(unary, nullary)
						maybe_save_progress

						@unary_index += 1
					end
					@nullary_index += 1
				end
				@exploration_state = EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES
				@nullary_index = 0
				@unary_index = 0
				next

			end

			next unless @exploration_state == EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES

			old_nullaries[@nullary_index..].each do |nullary|
				new_unaries[@unary_index..].each do |unary|
					commit_resulting_unaries_if_new(unary, nullary)
					maybe_save_progress

					@unary_index += 1
				end
				@nullary_index += 1
			end
			@exploration_state = EXPLORING_DONE
			@nullary_index = 0
			@unary_index = 0
			next
		end

		return unless @exploration_state == EXPLORING_DONE

		@exploration_state = EXPLORING_READY
		@depth += 1
	end

	def save_chains
		data = {
			nullaries_chain: @nullaries_chain,
			unaries_chain: @unaries_chain
		}

		File.write(File.join(__dir__, 'explorer_data.json'), data.to_json)
	end

	def evaluate_candidate_nullaries(depth)
		candidate_nullaries_file = File.open(File.join(__dir__, "candidate_nullaries_#{depth}.txt"))

		until candidate_nullaries_file.zero?
			nullary = file.gets.chomp
			next if @evaluations.include?(nullary)

			evaluation = evaluate_nullary(nullary)

			@evaluations[nullary] = evaluation.to_s

			equivalent_nullaries = @evaluations.select { |_, v| v == evaluation.to_s }.keys
			min_discovered_length = equivalent_nullaries.map(&:length).min

			@efficient_evaluations[nullary] = evaluation.to_s if evaluation.length < min_discovered_length

			File.write(File.join(__dir__, "candidate_nullaries_#{depth}.txt"), candidate_nullaries_file.readlines.shift.join("\n"))
		end
	end

	def evaluate_nullary(nullary, raw: false)
		puts "\nEvaluating nullary: #{nullary}\n\n"
		if raw == false && @evaluations.include?(nullary)
			puts 'Previously evaluated nullary'

			return @evaluations[nullary].to_i
		end

		if nullary == '1'
			evaluation = 1
			@evaluations[nullary] = evaluation.to_s
			return evaluation
		end
		if nullary.start_with?('^')
			evaluation = evaluate_unary('^(*)').call(evaluate_nullary(nullary[2..-2]))
			@evaluations[nullary] = evaluation.to_s
			return evaluation
		end

		raise MalformedNullaryStringError(nullary) unless nullary.start_with?('R')

		arguments = nullary[2..-2].split_into_top_level_arguments
		arguments[0] = unfreeze_unary(arguments[0])

		unary_evaluated = evaluate_unary(arguments[0])
		base_evaluated = evaluate_nullary(arguments[1])
		countdown_evaluated = evaluate_nullary(arguments[2])

		current_value = base_evaluated
		current_countdown = countdown_evaluated
		while current_countdown.positive?
			current_value = unary_evaluated.call(current_value)
			current_countdown -= 1
		end

		evaluation = current_value

		@evaluations[nullary] = evaluation.to_s
		evaluation
	end

	def evaluate_unary(unary)
		# puts "Evaluating unary: #{unary}"
		return ->(n) { n } if unary == '*'

		if unary.start_with?('^')
			unary_innards = unary[2..-2]
			# puts 'Innards'
			# puts unary_innards
			innards_evaluated = evaluate_unary(unary_innards)
			return lambda { |n|
											innards_evaluated.call(n) + 1
										}
		end

		raise MalformedNullaryStringError(nullary) unless unary.start_with?('R')

		# puts '-- Is Recursive --'
		arguments = unary[2..-2].split_into_top_level_arguments
		arguments[0] = unfreeze_unary(arguments[0])
		# puts arguments
		# puts '-' * 18

		unary_operator = evaluate_unary(arguments[0])

		if arguments[1] == '*'
			func = lambda { |n|
				initial_value = evaluate_nullary(arguments[2])
				countdown = n

				working_value = initial_value
				while countdown.positive?
					working_value = unary_operator.call(working_value)
					countdown -= 1
				end

				working_value
			}
			return func
		end

		raise "#{unary} is not a valid unary expression." unless arguments[2] == '*'

		lambda { |n|
			initial_value = n
			countdown = evaluate_nullary(arguments[1])

			working_value = initial_value
			while countdown.positive?
				working_value = unary_operator.call(working_value)
				countdown -= 1
			end

			working_value
		}
	end

	def unfreeze_unary(unary)
		return '*' if unary == '#'
		return "^(#{unfreeze_unary(unary[2..-2])})" if unary.start_with?('^')
		raise "String #{unary} does not correspond to a unary expression" unless unary.start_with?('R')

		arguments = unary[2..-2].split_into_top_level_arguments
		# puts '-' * 16
		# puts 'Unary top-level arguments:'
		# puts arguments
		# puts '-' * 16
		first_nullary_input_argument = arguments[1]
		second_nullary_input_argument = arguments[2]
		return "R(#{arguments[0]},*,#{second_nullary_input_argument})" if first_nullary_input_argument == '#'
		raise "String #{unary} does not correspond to a unary expression" unless second_nullary_input_argument == '#'

		"R(#{arguments[0]},#{first_nullary_input_argument},*)"
	end

	def document_nullary(representation)
		quantity = evaluate_nullary(representation)
		f = File.open('results.csv', 'a')
		f.write("\n")
		f.write("#{representation}, #{quantity}")
	end

	def document_nullaries
		nullaries.each do |nullary|
			document_nullary(nullary)
			catch MalformedNullaryStringError
			continue
		end
	end

	def create_evaluation_visualization
		values = evaluations.values.map(&:to_i)
		# lengths = evaluations.keys.map(&:length)

		x = values.uniq
		y = []
		c = []

		x.each do |i|
			e = evaluations.select { |_, v| v.to_i == i }
			puts e.first[0]
			min_length = e.map(&:first).map(&:length).min
			num_expressions = e.length
			y.append(min_length)
			c.append(num_expressions)
		end

		g = Gruff::Scatter.new(800)

		g.minimum_x_value = 0
		g.maximum_x_value = x.max
		g.minimum_value = 0
		g.maximum_value = y.max

		g.x_axis_increment = 1
		g.y_axis_increment = 10

		g.theme_37signals
		g.circle_radius = 2

		x.each do |j|
			jy = y[x.index(j)]
			jc = c[x.index(j)]
			g.data jc.to_s, j, [jy], interpolate_color(jc)
		end

		g.hide_legend = true
		g.title = 'Evaluations'

		g.write('evaluations.png')
	end
end

# Override String with useful helpers.
class String
	def sub_2nd(char, replacement)
		dup.tap { |s| s[s.index(char, s.index(char) + 1)] = replacement if s.index(char, s.index(char) + 1) }
	end

	def split_into_top_level_arguments
		arguments = []
		current_argument = 0

		depth = 0
		each_char do |char|
			arguments.append('') until arguments.length > current_argument
			if depth.zero? && char == ','
				current_argument += 1
				next
			end
			arguments[current_argument] += char
			depth += 1 if char == '('
			depth -= 1 if char == ')'
		end

		arguments
	end
end

def interpolate_color(value)
	# Special case for 0 to ensure it's blue
	return '#0000FF' if value.zero?

	# Logarithmic scaling with base 100
	normalized = Math.log(value + 1) / Math.log(10_000)
	normalized = [0.0, [1.0, normalized].min].max # Clamp between 0 and 1

	# RGB values for blue (0,0,255) to red (255,0,0)
	r = (255 * normalized).round
	b = (255 * (1 - normalized)).round

	format('#%02X%02X%02X', r, 0, b)
end

def commit_candidate_nullary_if_new(nullary)
	puts nullary
	return if @evaluations.include?(nullary)

	File.open(candidate_nullaries_file(@depth), 'a') do |f|
		f.puts(nullary)
	end
end

def commit_candidate_unary_if_new(unary)
	return if @evaluations.include?(unary)

	File.open(candidate_unaries_file(@depth), 'a') do |f|
		f.puts(unary)
	end
end

def commit_resulting_unaries_if_new(unary, nullary)
	first_intermediate = "R(%,#{nullary},&)"
	second_intermediate = "R(%,&,#{nullary})"
	unary_frozen = unary.sub('*', '#')
	first_new_unary = first_intermediate.sub('%', unary_frozen).sub('&', '*')
	second_new_unary = second_intermediate.sub('%', unary_frozen).sub('&', '*')
	commit_candidate_unary_if_new(first_new_unary)
	commit_candidate_unary_if_new(second_new_unary)
end

def commit_resulting_nullary_if_new(unary, nullary)
	commit_candidate_nullary_if_new(unary.sub('*', nullary))
end

def maybe_save_progress
	@save_ticker += 1
	save_progress if (@save_ticker % SAVE_MODULUS).zero?
end

def candidate_unaries_file(depth)
	File.join(__dir__, "candidate_unaries_#{depth}.txt")
end

def candidate_nullaries_file(depth)
	File.join(__dir__, "candidate_nullaries_#{depth}.txt")
end
