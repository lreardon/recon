class String
	def sub_2nd(char, replacement)
		dup.tap { |s| s[s.index(char, s.index(char) + 1)] = replacement if s.index(char, s.index(char) + 1) }
	end

	def split_into_top_level_arguments
		arguments = []
		current_argument = 0

		depth = 0
		each_char do |char|
			arguments.append('') until arguments.length > current_argument
			if depth.zero? && char == ','
				current_argument += 1
				next
			end
			arguments[current_argument] += char
			depth += 1 if char == '('
			depth -= 1 if char == ')'
		end

		arguments
	end
end
