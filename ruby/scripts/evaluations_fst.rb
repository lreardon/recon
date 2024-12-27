require 'benchmark'

include Benchmark

def load_evaluations_fst
	puts `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/load_evaluations_fst`
end

def query_evaluations_fst(nullary)
	res = nil
	output = ''
	# Benchmark.benchmark(CAPTION, 40, FORMAT) do |x|
		# output = x.report("Query Evaluations for #{nullary}") {
			output = `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/query_evaluations_fst "#{nullary}"`
		# }
		# x.report("Parse output") {
			res = Integer.try_parse(output)
		# }
	# end
	puts "Query for #{nullary} turns up #{res}"
	res
end