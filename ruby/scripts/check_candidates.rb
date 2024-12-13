# frozen_string_literal: true

def check_candidate_nullaries_against_fst
	`#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/check_candidate_nullaries_against_fst`
end

def check_candidate_unaries_against_fst
	`#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/check_candidate_unaries_against_fst`
end
