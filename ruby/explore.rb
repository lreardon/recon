# frozen_string_literal: true

require_relative 'explorer'
require_relative 'models/progress'

chains_file_path = File.join(__dir__, 'chains.json')
progress_file_path = File.join(__dir__, 'progress.json')

progress = Progress.from_json(JSON.parse(File.read(progress_file_path), symbolize_names: true))
chains = JSON.parse(File.read(chains_file_path), symbolize_names: true)

puts chains

$e = if File.exist?(chains_file_path) && File.exist?(progress_file_path)
						Explorer.new(
							evaluations: GDBM.new('evaluations.db'),
							efficient_evaluations: GDBM.new('efficient_evaluations.db'),
							chains:,
							progress:
						)
					else
						Explorer.new(
							evaluations: GDBM.new('evaluations.db'),
							efficient_evaluations: GDBM.new('efficient_evaluations.db'),
							progress: Progress.new,
							chains:
						).save
					end
