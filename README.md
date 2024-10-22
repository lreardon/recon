# <span style="color:red;">R</span><span style="color:red;">e</span>cursive <span style="color:blue;">C</span>omplexity <span style="color:orange;">O</span>f <span style="color:green;">N</span>umbers

## Motivation

The natural numbers are traditionally expressed in an implicit base system, as a string of characters, each character indicating a multiple of an implicit power of the implicit base, with implicit subsequent array reduction via addition. Example:

###### <span style="color:green;">2</span><span style="color:blue;">3</span><span style="color:red;">4</span> <span style="color:orange;">(base 10)
###### = <span style="color:red;">4 * <span style="color:orange;">10</span>^0</span> + <span style="color:blue;">3 * <span style="color:orange;">10</span>^1</span> + <span style="color:green;">2 * <span style="color:orange;">10</span>^2</span> 
###### = <span style="color:red;">4 * 1</span> + <span style="color:blue;">3 * 10</span> + <span style="color:green;">2 * 100</span>
###### = <span style="color:red;">4</span> + <span style="color:blue;">30</span> + <span style="color:green;">200</span> = 234

Bases 10 and 2 being the most widely used today, though other bases have been used throughout history (20 - Mayas; 60 -Babylonians; 12 - Egyptians).

This <b>traditional encoding</b> is logarithmic in the quantity of the number, however, much instruction is left implicit - we rely on iterated exponentiation (which is iterated multiplication, which itself is iterated addition, which itself is iterated succession), and we finally iterate addition over the resultants of the digits.

This suggests the question - what is the minimum string required to communicate a quantity, with the minimum implicit instructions.

For a minimal encoding of the positive numbers, we visit Peano Arithmetic, wherein an atomic 0-ary expression (sometimes interpreted as zero, but which we will call <b>1</b>) and an atomic 1-ary function (which we will call <b>+</b>) may be applied to any 0-ary expression to obtain a <b>next</b> 0-ary expression (which we can further denote or interpret as the rest of the positive natural numbers). We can draw an equivalence through quanity of the strings of our different encodings. Example:

###### 2 := +1
###### 3 := +2 = ++1
###### ...
###### 13 := ++++++++++++1

The <b>Peano encoding</b>, however, is so minimal as to be trivial, the representation of each quantity being exactly as large as the quantity itself. It should be that any finite brain would be severely limited in its ability to operate and organize quanitites so represented.

Noting that the representative failure of the Peano encoding stems from the fact that iterated succession is not condensed, we are motivated to invent a symbolic representation for the recursive application of a 1-ary expression upon a 0-ary expression. Such an expression requires a further piece of data, namely a 0-ary expression representing the number of times which the 1-ary expression should be iterated, which we will call the <b>(recursive) counter</b>. We will denote the <b>recursive operator</b> by <b>*</b>. We may require parentheses, though they are not necessary to parse a well-formed string, and thus they will count towardsthe length of a given number's <b>recursive representation</b>. As a convention, we will adopt the following order of arguments for a recursive expression: *abS is to be interpreted as "perform S b times upon a".

###### Note -- It miiiight be possible to dispense with the * altogether, by interpreting abS as perform S b times on a, with the convention of parsing from the right. But I have to think more about this. An advantage of explicitly including the * is that execution can begin from any 1 in the string.

Example:
###### *+++++1++1+++ = *(6)(3)+++ = (+++)(+++)(+++)6 = ... = 15

Note above that the recursive expression above has length 14, whereas the quanity of the expression is 15. While we haven't beaten the logarithmic representation, we have beat the Peano representation. As we will see, compounding the recursive expression allows us to express massive numbers with relatively little inforamtion (See Ackerman's Function).

In the recursive encoding, the complexity of a number is equal to the length of it's minimal recursive representation.

As the construction of the recursive encoding is minimal and natural, we are motivated to explore the notion of recursive complexity of numbers as a fundamental quality of their structure.

The best place to start with such a task is observation. Thus this repository contains (or will contain) scripts to explore, record, and visualize the recursive complexity of positive numbers.

If you are a researcher in this area of mathematics, and have stumbled upon this repository, I'd love to chat! You can find me in the usual places online -- linkedIn, personal website, email, etc.

