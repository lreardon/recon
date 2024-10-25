# frozen_string_literal: true

require_relative 'errors/malformed_nullary_string_error'
require 'gdbm'
require 'json'
require 'gruff'

# A class for building new unary and nullary expressions.
class Explorer
	attr_accessor :depth
	attr_reader :nullaries_chain, :unaries_chain, :evaluations

	def initialize(evaluations:, depth: 0, nullaries_chain: [['1']], unaries_chain: [['^(*)']], r: 'R(%,&,&)')
		@depth = depth
		@nullaries_chain = nullaries_chain
		@unaries_chain = unaries_chain
		@r = r
		@evaluations = evaluations
	end

	def nullaries
		@nullaries_chain.flatten
	end

	def unaries
		@unaries_chain.flatten
	end

	def explore
		start_time = Time.now

		last_unaries = @unaries_chain.last
		last_nullaries = @nullaries_chain.last

		old_unaries = @unaries_chain[0..-2].flatten
		old_nullaries = @nullaries_chain[0..-2].flatten

		new_nullaries = []

		num_candidate_nullaries = old_nullaries.length * last_nullaries.length
		puts "Will require #{num_candidate_nullaries} new candidate nullaries"

		old_nullaries.each do |nullary|
			last_unaries.each do |unary|
				new_nullaries << unary.sub('*', nullary)
			end
		end

		num_candidate_nullaries = last_nullaries.length * old_unaries.length
		puts "Will require #{num_candidate_nullaries} new candidate nullaries"

		last_nullaries.each do |nullary|
			old_unaries.each do |unary|
				new_nullaries << unary.sub('*', nullary)
			end
		end

		num_candidate_nullaries = last_nullaries.length * last_unaries.length
		puts "Will require #{num_candidate_nullaries} new candidate nullaries"

		last_nullaries.each do |nullary|
			last_unaries.each do |unary|
				new_nullaries << unary.sub('*', nullary)
			end
		end

		puts "checkpoint #{i}"

		new_unaries = []
		@nullaries_chain.flatten.each do |nullary|
			first_intermediate = @r.sub('&', nullary)
			second_intermediate = @r.sub_2nd('&', nullary)
			@unaries_chain.flatten.each do |unary|
				first_new_unary = first_intermediate.sub('%', unary.sub('*', '#')).sub('&', '*')
				second_new_unary = second_intermediate.sub('%', unary.sub('*', '#')).sub('&', '*')
				new_unaries += [first_new_unary, second_new_unary]
			end
		end

		final_new_nullaries = []
		new_nullaries.each do |new_nullary_candidate|
			final_new_nullaries << new_nullary_candidate unless @nullaries_chain.flatten.include? new_nullary_candidate
		end

		final_new_unaries = []
		new_unaries.each do |new_unary_candidate|
			final_new_unaries << new_unary_candidate unless @unaries_chain.flatten.include? new_unary_candidate
		end

		@nullaries_chain << final_new_nullaries
		@unaries_chain << final_new_unaries

		@depth += 1

		save

		Time.now - start_time
	end

	def save
		data = {
			depth: @depth,
			nullaries_chain: @nullaries_chain,
			unaries_chain: @unaries_chain,
			r: @r
		}

		File.write('explorer_data.json', data.to_json)
		# File.binwrite('explorer_data.dat', Marshal.dump(self))
	end

	def evaluate_nullaries
		nullaries.each do |nullary|
			evaluate_nullary(nullary)
		rescue MalformedNullaryStringError
			next
		end

		nil
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

		# puts 'Parsing Recursive Arguments'
		# puts '-' * 32
		arguments = nullary[2..-2].split_into_top_level_arguments
		# puts 'Found top level arguments:'
		# puts arguments
		# puts '-' * 16
		arguments[0] = unfreeze_unary(arguments[0])
		# puts "Unary:    #{arguments[0]}"
		# puts "Base:     #{arguments[1]}"
		# puts "Countdown: #{arguments[2]}"
		# puts '-' * 32
		# puts "\n"

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
