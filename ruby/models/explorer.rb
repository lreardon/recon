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

require "#{ENV.fetch('PROJECT_ROOT')}/ruby/errors/malformed_nullary_string_error"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/models/progress"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/models/evaluator"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/scripts/check_candidates"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/extensions/string"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/extensions/array"
# require "#{ENV.fetch('PROJECT_ROOT')}/ruby/scripts/evaluations_fst"

# A class for building new unary and nullary expressions.
class Explorer
	include Benchmark
	attr_accessor :depth, :evaluator
	attr_reader :nullaries_chain, :unaries_chain, :evaluations

	SAVE_MODULUS = 10_000

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
		@evaluator = Evaluator.new
		@depth = progress.depth
		@exploration_state = EXPLORING_READY
		@nullary_index = progress.nullary_index
		@unary_index = progress.unary_index

		@nullaries_chain = chains[:nullaries]
		@unaries_chain = chains[:unaries]

		@save_ticker = 0
		@exploration_phase_length = 0
		@exploration_phase_batch = progress.exploration_phase_batch
	end

	def explore
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
			@save_ticker = 0
			phase_starting_index = @exploration_phase_batch * SAVE_MODULUS
			
			if @exploration_state == EXPLORING_NULLARIES__NEW_NULLARIES__ALL_UNARIES
				outer_loop = new_nullaries
				inner_loop = unaries

				# @exploration_phase_length = outer_loop.length * inner_loop.length

				# nullaries_starting_index = phase_starting_index // inner_loop.length
				# unaries_starting_index = phase_starting_index % inner_loop.length

				new_nullaries.each do |nullary|
					unaries.each do |unary|
						record_resulting_candidate_nullary(unary, nullary)
						maybe_process_candidate_batch
					end
				end
				process_candidate_batch
				@exploration_state = EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES
				@exploration_phase_batch = 0
				next
			end

			if @exploration_state == EXPLORING_NULLARIES__OLD_NULLARIES__NEW_UNARIES
				@exploration_phase_length = old_nullaries.length * new_unaries.length
				old_nullaries.each do |nullary|
					new_unaries.each do |unary|
						record_resulting_candidate_nullary(unary, nullary)
						maybe_process_candidate_batch
					end
				end
				process_candidate_batch
				@exploration_state = EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES
				@exploration_phase_batch = 0
				next

			end

			if @exploration_state == EXPLORING_UNARIES__NEW_NULLARIES__ALL_UNARIES
				new_nullaries.each do |nullary|
					unaries.each do |unary|
						record_resulting_candidate_unaries(unary, nullary)
						maybe_process_candidate_batch
					end
				end
				process_candidate_batch
				@exploration_state = EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES
				@exploration_phase_batch = 0
				next

			end

			if @exploration_state == EXPLORING_UNARIES__OLD_NULLARIES__NEW_UNARIES
				old_nullaries.each do |nullary|
					new_unaries.each do |unary|
						record_resulting_candidate_unaries(unary, nullary)
						maybe_process_candidate_batch
					end
				end
				process_candidate_batch
				@exploration_state = EXPLORING_DONE
				@exploration_phase_batch = 0
				next
			end

			next unless @exploration_state == EXPLORING_DONE

			# process_candidate_batch # Not necessary if we process batches at the end of each phase
			@depth += 1
			@exploration_state = EXPLORING_READY
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

	def maybe_process_candidate_batch
		@save_ticker += 1
		process_candidate_batch if (@save_ticker % SAVE_MODULUS).zero?
	end
	
	def process_candidate_batch
		check_candidates_against_fst
		update_fst
		update_chains
		record_progress
		clear_candidate_files
		@exploration_phase_batch += 1
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
		new_nullaries_with_evaluations = evaluate_nullaries_from_new_nullaries_txt
		write_hash_to_file(new_nullaries_with_evaluations, new_nullaries_evaluated_file)

		new_unaries_length = File.readlines(new_unaries_tmp_file).size

		Benchmark.benchmark(CAPTION, 40, FORMAT) do |x|
			x.report("MERGING #{new_nullaries_with_evaluations.size} NEW NULLARIES INTO FST") { `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/merge_new_nullaries_evaluated_json_into_evaluations_fst` } if new_nullaries_with_evaluations.size.positive?
			x.report("MERGING #{new_unaries_length} NEW UNARIES INTO FST") { `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/merge_new_unaries_into_unaries_fst` } if new_unaries_length.positive?
		end
	end

	def update_chains
		@nullaries_chain[@depth].concat(File.readlines(new_nullaries_tmp_file).map(&:chomp)).uniq
		@unaries_chain[@depth].concat(File.readlines(new_unaries_tmp_file).map(&:chomp)).uniq
		save_chains
	end

	def clear_candidate_files
		# File.open(candidate_nullaries_tmp_file, 'w').close
		# File.open(candidate_unaries_tmp_file, 'w').close
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
			exploration_phase_batch: @exploration_phase_batch,
			nullary_index: @nullary_index,
			unary_index: @unary_index,
		).save
	end

	def evaluate_nullaries_from_new_nullaries_txt
		new_nullaries = File.readlines(candidate_nullaries_tmp_file).map(&:chomp).uniq
		evaluations = {}
		if new_nullaries.length.positive?
					evaluations = new_nullaries.each_with_object({}).with_index do |(nullary, hash), index|
						evaluation_time = Benchmark.realtime do
							puts "Depth #{@depth} -- #{@exploration_state} -- #{(@exploration_phase_batch * SAVE_MODULUS) + index + 1} of at most #{@exploration_phase_length} -- #{nullary}"
							hash[nullary] = evaluate(nullary)
						end
						# puts "Evaluation of #{nullary} took #{evaluation_time} seconds -- #{hash[nullary]}"
						# puts ""
					end
		end

		evaluations
	end

	def evaluate(nullary)
		@evaluator.evaluate_nullary(nullary)
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

	def double_iter_with_start_index(outer_array:, inner_array:, starting_index:)
		outer_array_start_index = starting_index.div(inner_array.length)
		inner_array_start_index = starting_index % inner_array.length

		outer_array[outer_array_start_index..-1].each do |outer_element|
			inner_array[inner_array_start_index..-1].each do |inner_element|
				yield outer_element, inner_element
			end
			inner_array_start_index = 0
		end

		nil
	end

	# def print_x_to_99(x)
	# 	double_iter_with_start_index(outer_array: (0...10).to_a, inner_array: (0...10).to_a, starting_index: x) do |tens, ones|
	# 		puts 10 * tens + ones
	# 	end
	# end
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

class Integer
  def self.try_parse(str)
    Integer(str)
  rescue ArgumentError, TypeError
    nil
  end
end