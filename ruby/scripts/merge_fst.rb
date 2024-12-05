# frozen_string_literal: true

rust_merge_fst_binary_path = "#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/merge_tmp_into_evaluations"

puts rust_merge_fst_binary_path

system(rust_merge_fst_binary_path)
