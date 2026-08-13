# <span style="color:red;">R</span><span style="color:red;">e</span>cursive <span style="color:blue;">C</span>omplexity <span style="color:orange;">O</span>f <span style="color:green;">N</span>umbers

## Motivation

The natural numbers are traditionally expressed in an implicit base system, as a string of characters, each character indicating a multiple of an implicit power of the implicit base, with implicit subsequent array reduction via addition. For example:

###### <span style="color:green;">2</span><span style="color:blue;">3</span><span style="color:red;">4</span> <span style="color:orange;">(base 10)

###### = <span style="color:red;">4 _ <span style="color:orange;">10</span>^0</span> + <span style="color:blue;">3 _ <span style="color:orange;">10</span>^1</span> + <span style="color:green;">2 \* <span style="color:orange;">10</span>^2</span>

###### = <span style="color:red;">4 _ 1</span> + <span style="color:blue;">3 _ 10</span> + <span style="color:green;">2 \* 100</span>

###### = <span style="color:red;">4</span> + <span style="color:blue;">30</span> + <span style="color:green;">200</span> = 234

Bases 10 and 2 are the most widely used today, though other bases have been used throughout history. E.g. 20 (Mayas) 60 (Babylonians), and 12 (Egyptians).

The length of this <b>traditional encoding</b> is logarithmic in the quantity of the number, however, much instruction is left implicit - we rely on iterated exponentiation, which is iterated multiplication, which itself is iterated addition, which itself is iterated <b>succession</b> in the sense of Peano arithmetic, and we finally iterate addition again over the resultants of the exponentiation operation on each digit.

This suggests the question - what is the minimum string length required to communicate a quantity, with the minimum of implicit instructions (syntax).

For a syntactically minimal encoding of the positive numbers, we visit Peano Arithmetic, wherein an atomic 0-ary expression (which we will call <b>0</b>) and an atomic 1-ary function (which we will call <b>^</b>) may be applied to any 0-ary expression to obtain a <b>next</b> 0-ary expression (which we can further denote or interpret as the rest of the positive natural numbers). We can draw an equivalence through quanity of the strings of our different encodings. Example:

###### 2 := +1

###### 3 := +2 = ++1

###### ...

###### 13 := ++++++++++++1

The <b>Peano encoding</b>, however, is so minimal as to be trivial, the representation of each quantity being exactly as large as the quantity itself. It should be that any finite brain would be severely limited in its ability to operate and organize quanitites so represented.

Noting that the representative failure of the Peano encoding stems from the fact that iterated succession is uncondensable, we are motivated to invent a symbolic representation for the recursive application of a 1-ary expression upon a 0-ary expression. Such an expression requires a further piece of data, namely a 3-ary expression representing the <b>number</b> of times that <b>some 1-ary expression</b> should be iterated, with <b>some 0-ary expression</b> as a base. We will call the <b>(recursive) counter</b>. We will denote the <b>recursive operator</b> by <b>R</b> or <b>*</b>. We may require parentheses, though they ought not be necessary to parse a well-formed string, and thus they ought not count towards the length of a given number's <b>recursive representation</b> (at any rate, including them would result in at most a 3x blowup off of the minimum representation). As a convention, we will adopt the following order of arguments for a recursive expression: *abS is to be interpreted as "perform S b times upon a". With parentheses, we might instead write R(S,a,b).

###### Note -- It miiiight be possible to dispense with the _ altogether, by interpreting abS as perform S b times on a, with the convention of parsing from the right. But I have to think more about this. An advantage of explicitly including the _ is that execution can begin from any 1 in the string.

Example:

###### _+++++1++1+++ = _(6)(3)+++ = (+++)(+++)(+++)6 = ... = 15

Note above that the recursive expression above has length 14, whereas the quanity of the expression is 15. While we haven't beaten the logarithmic representation, we have beat the Peano representation. As we will see, compounding the recursive expression allows us to express massive numbers with relatively little inforamtion (See Ackerman's Function).

In the recursive encoding, the complexity of a number is equal to the length of it's minimal recursive representation.

As the construction of the recursive encoding is minimal and natural, we are motivated to explore the notion of recursive complexity of numbers as a fundamental quality of their structure.

The best place to start with such a task is observation. Thus this repository contains (or will contain) scripts to explore, record, and visualize the recursive complexity of positive numbers.

If you are a researcher in this area of mathematics, and have stumbled upon this repository, I'd love to chat! You can find me in the usual places online -- linkedIn, personal website, email, etc.

# Recent Updates (8/9/2026)

Recent thought on the matter has led me to believe that begining with 0 rather than 1 is more natural. For one thing, it makes expressions like x \* y more natural, as you start with the base of 0 an proceed from there. I am going to change the explore and evaluation scripts to reflect this.

# Project Structure

Exploration scripts are carried out by rust functions. The source of truth for found expressions is contained in .fst files (under `fst/`), and the search's position is checkpointed in `state/`, so exploration can be stopped and resumed at any time without losing or repeating work.

# Using the `recon` command

`recon` is the search engine. Each time you run it, it generates new expressions, works out the number each one evaluates to, and saves everything it learns. You can stop it whenever you like (Ctrl-C is fine) and the next run picks up exactly where it left off.

If you use [direnv](https://direnv.net), the `.envrc` file already puts `recon` on your PATH and points it at this project — just run it from anywhere inside the repo:

```bash
recon                # explore a little (one round)
recon --units 40     # explore more (forty rounds)
```

By default, `recon` searches **shortest expressions first**: it works through every possible expression of length 1, then length 2, and so on. Each `--units` round finishes one expression length completely. The payoff of going in this order is certainty — once a round finishes, you know that no shorter expression exists for any number found so far. The "exhaustive through length N" line it prints tells you how far that guarantee currently reaches.

There is also an older style of search that grows expressions by how deeply they nest rather than by how long they are:

```bash
recon --mode depth --units 2
```

Feel free to switch back and forth between the two — they share everything they learn, so no work is ever lost or repeated when you swap.

A few practical flags:

- `--max-items 50000` — stop after a set amount of work, even mid-round. Handy for dipping a toe into rounds that have grown large.
- `--batch-secs 30` — how often results are saved while running (the default is fine; everything up to the last save survives a crash or Ctrl-C).

To peek at the results from the terminal:

```bash
min_form 6           # the shortest known expression for 6
min_form 6 -v        # ...plus whether it is guaranteed shortest
min_form             # the whole table of shortest known expressions
ls_eval 20           # the first 20 recorded expressions and their values
```

# Using the dashboard

There is a small desktop app in `flutter/dashboard` that shows what the search is up to and lets you poke at the results without touching the terminal.

To launch it:

```bash
cd flutter/dashboard
flutter run -d macos
```

(Or, once built, reopen it directly: `open build/macos/Build/Products/Debug/recon_dashboard.app`.)

The dashboard has three parts:

- **Exploration progress** — how far the search has gotten, how many expressions and values are known, and when results were last saved. It refreshes itself every couple of seconds, so you can leave it open while `recon` runs and watch the numbers climb.
- **Evaluate a form** — type in any expression (for example `R(^(#),0,^(^(0)))`) and it tells you the number it evaluates to.
- **Minimum form for a value** — type in a number and it shows the shortest expression currently known for it.

The dashboard assumes the project lives at `~/git/recon`; if you keep it somewhere else, launch the app with the `PROJECT_ROOT` environment variable pointing at your copy.
