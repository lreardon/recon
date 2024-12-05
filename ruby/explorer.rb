# frozen_string_literal: true

#
#
# The Explorer is initialized from our persistent data.
# The Explorer utilizes the persistent data to explore new unary and nullary expressions.
# The Explorer evaluates new nullary expressions and saves the results to our persistent data.
# The top-level method of the Explorer is `explore`, which iterates through the exploration process.
# Occasionally, the Explorer invokes `save` to save its progress to our persistent data.

require 'gdbm'
require 'json'
require 'gruff'

require_relative 'errors/malformed_nullary_string_error'
require_relative 'models/progress'

# A class for building new unary and nullary expressions.
class Explorer
	attr_accessor :depth
	attr_reader :nullaries_chain, :unaries_chain, :evaluations

	SAVE_MODULUS = 5000

	EXPLORING_READY = :exploring_ready
	EXPLORING_NULLARIES__NEW_NULLARIES__ALL_UNARIES = :exploring_nullaries_new_nullaries_all_unaries
	EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES = :exploring_nullaries_old_nullaries_new_unaries
	EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES = :exploring_unaries_new_nullaries_all_unaries
	EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES = :exploring_unaries_old_nullaries_new_unaries
	EXPLORING_DONE = :exploring_done

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
		@efficient_evaluations = efficient_evaluations
		@save_ticker = 0
	end

	def nullaries
		@nullaries_chain.flatten
	end

	def unaries
		@unaries_chain.flatten
	end

	def save_progress
		progress_json = {
			depth: @depth,
			exploration_phase: @exploration_state,
			nullary_index: @nullary_index,
			unary_index: @unary_index
		}
		File.write(File.join(__dir__, 'progress.json'), progress_json.to_json)

		# What we really need to do here is document our recent evaluations and merge them into our full evaluations, as fst objects.
		# First step is to find our most recent evaluations.
	end

	def explore
		@save_ticker = 0
		new_nullaries = @nullaries_chain.last
		old_nullaries = @nullaries_chain[0..-2].flatten

		new_unaries = @unaries_chain.last
		# old_unaries = @unaries_chain[0..-2].flatten #? Not explicitly needed, as we pair new nullaries against all unaries

		while @exploration_state != EXPLORING_DONE
			if @exploration_state == EXPLORING_READY
				# Ensure candidate nullaries and unaries files exist
				File.write(candidate_nullaries_file(@depth), '') unless File.exist?(candidate_nullaries_file(@depth))
				File.write(candidate_unaries_file(@depth), '') unless File.exist?(candidate_unaries_file(@depth))
				File.write(candidate_nullaries_tmp_file, '') unless File.exist?(candidate_nullaries_tmp_file)
				File.write(candidate_unaries_tmp_file, '') unless File.exist?(candidate_unaries_tmp_file)
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

		# (0..@depth).each do |i|
		# 	File.open(candidate_nullaries_file(i)) do |f|
		# 		f.each do |l|
		# 			evaluate_nullary(l)
		# 			# puts l
		# 		end
		# 	end
		# end

		evaluate_candidate_nullaries(@depth)
		@exploration_state = EXPLORING_READY
		@depth += 1
	end

	# def save_chains
	# 	data = {
	# 		nullaries_chain: @nullaries_chain,
	# 		unaries_chain: @unaries_chain
	# 	}

	# 	File.write(File.join(__dir__, 'explorer_data.json'), data.to_json)
	# end

	def evaluate_candidate_nullaries(depth)
		candidate_nullaries_file = File.open(candidate_nullaries_file(depth))

		until File.empty?(candidate_nullaries_file)
			nullary = candidate_nullaries_file.gets
			next if nullary.nil?
			next if @evaluations.include?(nullary)

			evaluation = evaluate_nullary(nullary)

			@evaluations[nullary] = evaluation.to_s

			equivalent_nullaries = @evaluations.select { |_, v| v == evaluation.to_s }
			min_discovered_length = equivalent_nullaries.map(&:length).min

			@efficient_evaluations[nullary] = evaluation.to_s if evaluation.length < min_discovered_length

			File.write(candidate_nullaries_file(depth), candidate_nullaries_file.readlines.shift.join("\n"))
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

	def commit_candidate_nullary_if_new(nullary)
		puts nullary
		# TODO: I think this isn't robust enough.
		return if @evaluations.include?(nullary)

		File.open(candidate_nullaries_file(@depth), 'a') do |f|
			f.puts(nullary)
		end
	end

	def record_nullary_in_tmp_if_new(nullary)
		return if check_nullaries_fst_for_nullary(nullary)

		File.open(candidate_nullaries_tmp_file, 'a') do |f|
			return nil if f.readlines.include?(nullary)

			f.puts(nullary)
		end
	end

	def record_unary_in_tmp_if_new(unary)
		return if check_unaries_fst_for_unary(unary)

		File.open(candidate_unaries_tmp_file, 'a') do |f|
			return nil if f.readlines.include?(unary)

			f.puts(unary)
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
		resulting_nullary = unary.sub('*', nullary)
		commit_candidate_nullary_if_new(resulting_nullary)
	end

	def maybe_save_progress
		@save_ticker += 1
		save_progress if (@save_ticker % SAVE_MODULUS).zero?
	end

	def candidate_dir
		File.join(__dir__, 'candidates')
	end

	def candidate_nullaries_dir
		File.join(candidate_dir, 'nullaries')
	end

	def candidate_unaries_dir
		File.join(candidate_dir, 'unaries')
	end

	def candidate_nullaries_file(depth)
		File.join(candidate_nullaries_dir, "cn_#{depth}.txt")
	end

	def candidate_unaries_file(depth)
		File.join(candidate_unaries_dir, "cu_#{depth}.txt")
	end

	def candidate_nullaries_tmp_file
		File.join(candidate_dir, 'tmp', 'nullaries.txt')
	end

	def candidate_unaries_tmp_file
		File.join(candidate_dir, 'tmp', 'nullaries.txt')
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
