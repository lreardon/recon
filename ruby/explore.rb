# frozen_string_literal: true

require_relative 'explorer'

json_file_path = File.join(__dir__, 'explorer_data.json')

if File.exist?(json_file_path)
		explorer_data_json = JSON.parse(File.read(json_file_path))
		$e =	Explorer.new(
				evaluations: GDBM.new('evaluations.db'),
				depth: explorer_data_json['depth'],
				nullaries_chain: explorer_data_json['nullaries_chain'],
				unaries_chain: explorer_data_json['unaries_chain'],
				r: explorer_data_json['r']
		)
else
		$e = Explorer.new(evaluations: GDBM.new('evaluations.db')).save
end
