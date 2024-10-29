# frozen_string_literal: true

rust_binary_path = "#{ENV.fetch('PROJECT_ROOT', nil)}/rust/evaluations/target/release/merge_tmp_into_evaluations"

puts rust_binary_path

system(rust_binary_path)
