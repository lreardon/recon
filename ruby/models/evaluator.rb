require "#{ENV.fetch('PROJECT_ROOT')}/ruby/extensions/string"
require "#{ENV.fetch('PROJECT_ROOT')}/ruby/extensions/array"

class Evaluator
	attr_accessor :mem_evals

	def initialize
		Benchmark.benchmark(CAPTION, 40, FORMAT) do |x|
			x.report('Loading Evaluations...') {
				puts `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/load_evaluations_fst`
			}
		end

		@mem_evals = {}
	end

	def query_evaluations_fst(nullary)
		output = `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/query_evaluations_fst "#{nullary}"`
		Integer.try_parse(output)
	end

	def evaluate_nullary(nullary)
		if @mem_evals.key?(nullary)
			return @mem_evals[nullary]
		end

		rec_dep = recursive_depth(nullary)
		already_evaluated = nil
		if rec_dep > 3
			already_evaluated = query_evaluations_fst(nullary)
		end			
		
		if already_evaluated.is_a?(Integer)
			evaluation = already_evaluated

		elsif nullary == '1'
			evaluation = 1

		elsif nullary.start_with?('^')
			evaluation = evaluate_unary('^(*)').call(evaluate_nullary(nullary[2..-2]))

		elsif nullary.start_with?('R')

			arguments = nullary[2..-2].split_into_top_level_arguments

			arguments[0] = unfreeze_unary(arguments[0])

			unary_evaluated = evaluate_unary(arguments[0])
			base_evaluated = evaluate_nullary(arguments[1])
			countdown_evaluated = evaluate_nullary(arguments[2])
			
			# puts "Unary - #{arguments[0]}"
			# puts "Base - #{base_evaluated}"
			# puts "Countdown - #{countdown_evaluated}"

			current_value = base_evaluated
			current_countdown = countdown_evaluated

			while current_countdown.positive?
				current_value = unary_evaluated.call(current_value)
				# puts "--- tick --- #{current_countdown}"
				# puts "    #{current_value}"

				current_countdown -= 1
			end

			evaluation = current_value
			# puts "    #{nullary} ==comp== #{already_evaluated}"

		else
			raise MalformedNullaryStringError(nullary)
		end
		
		@mem_evals[nullary] = evaluation unless @mem_evals.key?(nullary)

		return evaluation
	end

	def evaluate_unary(unary)
		return ->(n) { n } if unary == '*'

		if unary.start_with?('^')
			unary_innards = unary[2..-2]
			innards_evaluated = evaluate_unary(unary_innards)
			return lambda { |n|
											innards_evaluated.call(n) + 1
										}
		end

		raise MalformedNullaryStringError(nullary) unless unary.start_with?('R')

		arguments = unary[2..-2].split_into_top_level_arguments
		arguments[0] = unfreeze_unary(arguments[0])

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
		first_nullary_input_argument = arguments[1]
		second_nullary_input_argument = arguments[2]
		return "R(#{arguments[0]},*,#{second_nullary_input_argument})" if first_nullary_input_argument == '#'
		raise "String #{unary} does not correspond to a unary expression" unless second_nullary_input_argument == '#'

		"R(#{arguments[0]},#{first_nullary_input_argument},*)"
	end

	def recursive_depth(nullary)
		return 0 if nullary == '1'
		
		if nullary.start_with?('^')
			return recursive_depth(nullary[2..-2])
		end

		raise MalformedNullaryStringError(nullary) unless nullary.start_with?('R')

		arguments = nullary[2..-2].split_into_top_level_arguments
		arguments[0] = unfreeze_unary(arguments[0])

		# unary_depth = recursive_depth(arguments[0])
		base_depth = recursive_depth(arguments[1])
		countdown_depth = recursive_depth(arguments[2])

		1 + [base_depth, countdown_depth].max
	end
end