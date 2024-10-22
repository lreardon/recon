# using CSV
# using DataFrames

# quantities_path = "quantities.csv"
# quantities_csv = CSV.File(quantities_path)
# quantities_df = DataFrame(quantities_csv)

# depths = observations_df[!, :depth]
# deepest_depth = maximum(depths)

# function sub_first_arg(expression::AbstractString)
#     index = findfirst("#", original_string)
    
#     if index === nothing
#         return original_string  # If '#' is not found, return the original string as is
#     end
    
#     before = original_string[1:index - 1]
#     after = original_string[index + 1:end]
    
#     return before * replacement * after
# end

# function plug_first(expression::AbstractString, quantity::AbstractString)
# 	state = ""
# 	is_operator = false
# 	for char in expression
# 		if char == '*'
# 			state = "*Expecting("
# 		end
# 		if state == "*Expecting("
# 			if char == '('
# 				state = "*ExpectingOperator"
# 			end
# 		end
# 		if state == "*ExpectingOperator"
			
# 		end
# 	for 
# 		replace("$i", "#" => quantity, count=1)
# 	end
# end

# function is_quantity(expression::AbstractString)
# 		return !occursin("#", expression)
# end







# function stringIsWellFormedQuantity(expression::AbstractString)
# 	if expression == "0"
# 		return true
# 	end

# 	if startswith(expression, "^")
# 		@assert expression[1] = '('
# 		@assert expression[end] = ')'
# 		internalString = expression[2:end-1]
# 		return stringIsWellFormedQuantity(internalString)
# 	end

# 	if startswith(expression, "*")
# 		return expressionIsWellFormedRecursion(expression)
# end

# for n in 1:4

# 	new_quantities = String[]
# 	new_indeterminates = String[]

# 	for i in indeterminates
# 		for q in quantities
# 			new_expression = replace("$i", "#" => q, count=1)
# 			if is_quantity(new_expression)
# 				push!(new_quantities, new_expression)
# 			else
# 				push!(new_indeterminates, new_expression)
# 			end
# 		end
# 	end
# 	unique!(push!(quantities, new_quantities...))


# 	for i in indeterminates
# 		new_indeterminate = "*($i,#,#)"
# 		push!(new_indeterminates, new_indeterminate)
# 	end

# 	unique!(push!(indeterminates, new_indeterminates...))
# end

# println(quantities)





# println(indeterminates)

# *(*(^(x),x,x),%,%)

# *(^(x),x,x)


# "^(#)"

# "*(^(#),#,#)"