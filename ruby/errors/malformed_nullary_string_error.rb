# frozen_string_literal: true

# An error for reporting that a String does not correspond to a nullary expression.
class MalformedNullaryStringError < StandardError
		attr_reader :nullary

		def initialize(nullary)
				super("String #{nullary} is not a well-formed nullary expression.")
		end
end
