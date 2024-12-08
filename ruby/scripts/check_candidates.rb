# frozen_string_literal: true

def check_candidate_nullaries_against_fst
	puts 'Checking candidate nullaries against FST...'
	response = `#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/check_candidate_nullaries_against_fst`
	puts response
end

def check_candidate_unaries_against_fst
	puts 'Checking candidate unaries against FST...'
	command = "#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/check_candidate_unaries_against_fst"
	system(command)
end
