class Expression:
	def __init__(self, expression):
		self.expression = expression
	
	@property
	def isRecursive(self):
		if self.expression[0] == '*':
				return True

class ExpressionParser:
	def __init__(self):
		pass
	
	def indexOfFirstIndeterminate(self, expression: Expression):
		# Base cases
		if expression.expression == '0':
			return None		
		if expression.expression == '#':
			return 0
		
		# Successor case
		if expression.expression.startswith('^'):
			assert expression.expression[0:2] == '^('
			assert expression.expression[-1:] == ')'
			internals = expression.expression[2:-1]
			indexOfIndeterminateInInternals = self.indexOfFirstIndeterminate(Expression(internals))
			return 2 + indexOfIndeterminateInInternals

		# Recursive case			
		if expression.isRecursive:
			print(expression.expression)
			assert expression.expression[0:2] == '*('
			assert expression.expression[-1:] == ')'
			workingString = expression.expression[2:-1]

			# Operator
			indexOfFirstOpenParen = workingString.index('(')
			n = indexOfFirstOpenParen
			while n < len(workingString):
				prefix = workingString[0:n+1]
				if prefix.count('(') == prefix.count(')'):
					break
				n += 1

			operatorString = prefix	
			workingString = workingString[len(operatorString):]


			assert workingString.startswith(',')
			workingString = workingString[1:]

			# First quantity
			if workingString.startswith('0'):
				firstQuantityString = '0'
				workingString = workingString[1:]
			elif workingString.startswith('#'):
				firstQuantityString = '#'
				workingString = workingString[1:]
			else:
				indexOfFirstOpenParen = workingString.index('(')
				n = indexOfFirstOpenParen
				while n < len(workingString):
					prefix = workingString[0:n+1]
					if prefix.count('(') == prefix.count(')'):
						break
					n += 1
				firstQuantityString = prefix
				workingString = workingString[len(firstQuantityString):]

			indexOfFirstIndeterminateInFirstQuantityString = self.indexOfFirstIndeterminate(Expression(firstQuantityString))
			if indexOfFirstIndeterminateInFirstQuantityString is not None:
				return 2 + len(operatorString) + 1 + indexOfFirstIndeterminateInFirstQuantityString			

			# Second quantity
			assert workingString.startswith(',')
			secondQuantityString = workingString[1:]

			indexOfIndeterminateInSecondQuantityString = self.indexOfFirstIndeterminate(Expression(secondQuantityString))
			if indexOfIndeterminateInSecondQuantityString is not None:
				return 2 + len(operatorString) + 1 + len(firstQuantityString) + 1 + indexOfIndeterminateInSecondQuantityString

			return None
		
e = ExpressionParser()

i = e.indexOfFirstIndeterminate(Expression("*(*(*(^,#,#),#,#),#,#)"))
print(i)