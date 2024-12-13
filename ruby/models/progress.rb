# frozen_string_literal: true

class Progress
	attr_reader :depth, :nullary_index, :unary_index

	def initialize(depth: 1, exploration_state: :exploration_ready, nullary_index: 0, unary_index: 0)
		@depth = depth
		@nullary_index = nullary_index
		@unary_index = unary_index
		@exploration_state = exploration_state
	end

	def self.from_json(json)
		Progress.new(
			depth: json[:depth],
			nullary_index: json[:nullary_index],
			unary_index: json[:unary_index],
			exploration_state: json[:exploration_state]
		)
	end

	def to_json(*_args)
		{
			depth: @depth,
			nullary_index: @nullary_index,
			unary_index: @unary_index,
			exploration_state: @exploration_state
		}
	end

	def save
		pretty_json = JSON.pretty_generate(to_json)
		puts pretty_json
		File.write(File.join(__dir__, 'progress.json'), pretty_json)
		self
	end
end
