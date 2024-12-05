# frozen_string_literal: true

check_tmp_nullaries_against_fst = "#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/check_tmp_nullaries_against_fst"

system(check_tmp_nullaries_against_fst)
