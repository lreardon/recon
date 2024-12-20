# frozen_string_literal: true

#
#
# The Explorer is initialized from our persistent data.
# The Explorer utilizes the persistent data to explore new unary and nullary expressions.
# The Explorer evaluates new nullary expressions and saves the results to our persistent data.
# The top-level method of the Explorer is `explore`, which iterates through the exploration process.
# Occasionally, the Explorer invokes `save` to save its progress to our persistent data.

require 'json'
require 'gruff'
require 'benchmark'

require_relative 'errors/malformed_nullary_string_error'
require_relative 'models/progress'
require_relative 'scripts/check_candidates'

# A class for building new unary and nullary expressions.
class Explorer
	include Benchmark
	attr_accessor :depth
	attr_reader :nullaries_chain, :unaries_chain, :evaluations

	SAVE_MODULUS = 20_000

	EXPLORING_READY = :exploring_ready
	EXPLORING_NULLARIES__NEW_NULLARIES__ALL_UNARIES = :exploring_nullaries_new_nullaries_all_unaries
	EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES = :exploring_nullaries_old_nullaries_new_unaries
	EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES = :exploring_unaries_new_nullaries_all_unaries
	EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES = :exploring_unaries_old_nullaries_new_unaries
	EXPLORING_DONE = :exploring_done

	def initialize(
		progress:,
		chains:
	)
		@depth = progress.depth
		@exploration_state = EXPLORING_READY
		@nullary_index = progress.nullary_index
		@unary_index = progress.unary_index

		@nullaries_chain = chains[:nullaries]
		@unaries_chain = chains[:unaries]

		@save_ticker = 0
	end

	# def nullaries
	# 	@nullaries_chain[0..@depth - 1].flatten
	# end

	# def unaries
	# 	@unaries_chain[0..@depth - 1].flatten
	# end

	def explore
		@save_ticker = 0
		old_nullaries = @nullaries_chain[0..@depth - 2].flatten
		new_nullaries = @nullaries_chain[@depth - 1]

		old_unaries = @unaries_chain[0..@depth - 2].flatten
		new_unaries = @unaries_chain[@depth - 1]
		unaries = old_unaries + new_unaries

		if @exploration_state == EXPLORING_READY
			# Ensure candidate nullaries and unaries files exist
			File.write(candidate_nullaries_tmp_file, '') unless File.exist?(candidate_nullaries_tmp_file)
			File.write(candidate_unaries_tmp_file, '') unless File.exist?(candidate_unaries_tmp_file)
			# Ensure nullaries and unaries chains have the correct length to handle the current depth
			@nullaries_chain.append([]) unless @nullaries_chain.index_in_range?(@depth)
			@unaries_chain.append([]) unless @unaries_chain.index_in_range?(@depth)
			@exploration_state = EXPLORING_NULLARIES__NEW_NULLARIES__ALL_UNARIES
		end

		while @exploration_state != EXPLORING_READY
			puts @exploration_state.upcase

			if @exploration_state == EXPLORING_NULLARIES__NEW_NULLARIES__ALL_UNARIES
				puts new_nullaries.length * unaries.length
				new_nullaries.each do |nullary|
					unaries.each do |unary|
						record_resulting_candidate_nullary(unary, nullary)
						maybe_process_candidate_batches
					end
				end
				@exploration_state = EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES
				next
			end

			if @exploration_state == EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES
				puts old_nullaries.length * new_unaries.length
				old_nullaries.each do |nullary|
					new_unaries.each do |unary|
						record_resulting_candidate_nullary(unary, nullary)
						maybe_process_candidate_batches
					end
				end
				@exploration_state = EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES
				next

			end

			if @exploration_state == EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES
				puts new_nullaries.length * unaries.length
				new_nullaries.each do |nullary|
					unaries.each do |unary|
						record_resulting_candidate_unaries(unary, nullary)
						maybe_process_candidate_batches
					end
				end
				@exploration_state = EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES
				next

			end

			if @exploration_state == EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES
				puts old_nullaries.length * new_unaries.length
				old_nullaries.each do |nullary|
					new_unaries.each do |unary|
						record_resulting_candidate_unaries(unary, nullary)
						maybe_process_candidate_batches
					end
				end
				@exploration_state = EXPLORING_DONE
				next
			end

			next unless @exploration_state == EXPLORING_DONE

			process_candidate_batches

			@depth += 1

			@exploration_state = EXPLORING_READY
		end
	end

	def evaluate_nullary(nullary)
		if nullary == '1'
			evaluation = 1
			return evaluation
		end
		if nullary.start_with?('^')
			evaluation = evaluate_unary('^(*)').call(evaluate_nullary(nullary[2..-2]))
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

		current_value
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

	def record_resulting_candidate_unaries(unary, nullary)
		first_intermediate = "R(%,#{nullary},&)"
		second_intermediate = "R(%,&,#{nullary})"
		unary_frozen = unary.sub('*', '#')
		first_new_unary = first_intermediate.sub('%', unary_frozen).sub('&', '*')
		second_new_unary = second_intermediate.sub('%', unary_frozen).sub('&', '*')

		File.open(candidate_unaries_tmp_file, 'a') do |f|
			f.puts(first_new_unary)
			f.puts(second_new_unary)
		end
	end

	def record_resulting_candidate_nullary(unary, nullary)
		candidate_nullary = unary.sub('*', nullary)
		File.open(candidate_nullaries_tmp_file, 'a') do |f|
			f.puts(candidate_nullary)
		end
	end

	def maybe_process_candidate_batches
		@save_ticker += 1
		process_candidate_batches if (@save_ticker % SAVE_MODULUS).zero?
	end

	def process_candidate_batches
		check_candidates_against_fst
		update_fst
		update_chains
		record_progress
		# clear_candidate_files
		puts ''
		puts ''
	end

	def check_candidates_against_fst
		Benchmark.benchmark(CAPTION, 40, FORMAT) do |x|
			x.report('CHECK NULLARIES AGAINST FST') { check_candidate_nullaries_against_fst }
			x.report('CHECK UNARIES AGAINST FST') { check_candidate_unaries_against_fst }
		end
	end

	def update_fst
		# At this point in the exploration process, we've filled out a new batch of candidate nullaries and/or unaries.
		# If we have nullaries, we'll need to evaluate them.
		# Then we need to update the fst objects.
		# Finally, and this should be the easiest, we need to update the chains on our explorer.
		# Something was wrong with the way we were updating the chains - it was resulting in bad looping in the exploration process.
		# That said, the current implementation is resulting in a lot of redundant work.

		new_nullaries_with_evaluations = evaluate_nullaries_from_new_nullaries_txt
		write_hash_to_file(new_nullaries_with_evaluations, new_nullaries_evaluated_file)

		new_unaries_length = File.readlines(new_unaries_tmp_file).size

		Benchmark.benchmark(CAPTION, 40, FORMAT) do |x|
			x.report("MERGING #{new_nullaries_with_evaluations.size} NEW NULLARIES INTO FST") { `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/merge_new_nullaries_evaluated_json_into_evaluations_fst` } if new_nullaries_with_evaluations.size.positive?
			x.report("MERGING #{new_unaries_length} NEW UNARIES INTO FST") { `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/merge_new_unaries_into_unaries_fst` } if new_unaries_length.positive?
		end
	end

	def update_chains
		@nullaries_chain[@depth].concat(File.readlines(new_nullaries_tmp_file).map(&:chomp))
		@unaries_chain[@depth].concat(File.readlines(new_unaries_tmp_file).map(&:chomp))
		save_chains
	end

	def clear_candidate_files
		Dir.foreach(candidate_dir) do |file_name|
			next if ['.', '..'].include?(file_name)

			file_path = File.join(candidate_dir, file_name)
			File.open(file_path, 'w').close
		end
	end

	def save_chains
		File.write(File.join(__dir__, 'chains.json'), JSON.pretty_generate({ unaries: @unaries_chain, nullaries: @nullaries_chain }))
	end

	def write_hash_to_file(hash, file)
		File.write(file, hash.to_json)
	end

	def record_progress
		Progress.new(
			depth: @depth,
			exploration_state: @exploration_state,
			nullary_index: @nullary_index,
			unary_index: @unary_index
		).save
	end

	def evaluate_nullaries_from_new_nullaries_txt
		new_nullaries = File.readlines(candidate_nullaries_tmp_file).map(&:chomp)

		# TODO: This step will surely lag before long. We need to evaluate nullaries in batches.
		evaluations = {}
		if new_nullaries.length.positive?
			Benchmark.benchmark(CAPTION, 40, FORMAT) do |x|
				x.report("EVALUATING #{new_nullaries.length} NULLARIES") do
					evaluations = new_nullaries.each_with_object({}).with_index do |(nullary, hash), index|
						puts "#{index}/#{new_nullaries.length}"
						evaluation_time = Benchmark.realtime do
							hash[nullary] = evaluate_nullary(nullary)
						end
						puts "Evaluation of #{nullary} took #{evaluation_time} seconds." if evaluation_time > 1
					end
				end
			end
		end

		evaluations
	end

	def candidate_dir
		File.join(ENV.fetch('PROJECT_ROOT', nil), 'candidates')
	end

	def candidate_nullaries_tmp_file
		File.join(candidate_dir, 'nullaries.txt')
	end

	def candidate_unaries_tmp_file
		File.join(candidate_dir, 'unaries.txt')
	end

	def new_nullaries_tmp_file
		File.join(candidate_dir, 'new_nullaries.txt')
	end

	def new_unaries_tmp_file
		File.join(candidate_dir, 'new_unaries.txt')
	end

	def new_nullaries_evaluated_file
		File.join(candidate_dir, 'new_nullaries_evaluated.json')
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

class Array
	def index_in_range?(index)
		index >= 0 && index < length
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
