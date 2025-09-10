# Collatz Conjecture Analysis

Let's try to prove or disprove the Collatz conjecture together. Let's start by defining the conjecture:

> **Collatz Conjecture**: For any positive integer n:
> - If n is even, divide by 2
> - If n is odd, multiply by 3 and add 1
> 
> With enough repetition, do all positive integers converge to 1?

## Graph Representation

This can be viewed as a directed graph with a known cycle:

$$4 \rightarrow 2 \rightarrow 1 \rightarrow 4$$

We can then label the edges on this graph to give different labels for the halving and the triple + 1 operations. I'll use right arrows (→) to represent halving and up arrows (↑) to represent tripling plus one.

$$4 \xrightarrow{D} 2 \xrightarrow{D} 1 \xrightarrow{U} 4$$

## Walk Trees

Now let's define a new type of Topological Graph Query language called **walk trees**. Walk trees can be used to select portions of an edge-labeled graph with a certain topology.

Walk trees can be defined in many ways, but their simplest form would be a tree generation algorithm which is evaluated in a similar way to how the tree of a CFG is generated.

So if we had rules like:

- $D \rightarrow D$
- $D \rightarrow U$

This walk "tree" would represent any linear walk of N edges with label D and exactly one edge with label U. We'll name this particular walk tree **"double stack"** as it represents the stacks of numbers in Collatz which double ad infinitum.

Generally, this would select walks like:

$$(\text{even}) \xrightarrow{D} (\text{even}) \xrightarrow{D} (\text{even}) \xrightarrow{D} \cdots (\text{odd}) \xrightarrow{U} (\text{even})$$

More generally:

$$n \cdot 2^{\infty} (\text{even}) \cdots (n \cdot 2^3 \text{ even}) \xrightarrow{D} (n \cdot 2^2 \text{ even}) \xrightarrow{D} (n \cdot 2^1 \text{ even}) \xrightarrow{D} (n \text{ odd}) \xrightarrow{U} (3n+1 \text{ even})$$

## Condensed Collatz Graph

Let's now make a graph of the walk trees that match the "double stack" walk tree.

Each of these walk trees covers exactly one odd number and an infinite number of even numbers.

We can iterate through all of the odd numbers by simply counting up:

$$1, 3, 5, 7, \ldots$$

And thus iterate through all of the walk trees in our graph.

We can also represent this iteration as an iteration of:

$$2a + 1 \text{ starting at } 0$$

When we do this, we can then represent the set of all even numbers "to the left" of our odd number (as well as our odd number) with the expression $(2a+1) \cdot 2^k$ as well as the even number pointed to by that odd number.

Just as when iterating from 0 we can iterate through the odd numbers using the expression $2a+1$, when iterating through the left-hand side even numbers (those that are pointed to from an odd number) using the expression $6a+4$. For example, $6 \cdot 0 + 4 \xrightarrow{U} 2 \cdot 0 + 1$ represents the connection between 4 and 1.

Earlier I told you that we wish to build a graph of the "double stack" walk trees. So far I have shown that these "double stack" walk trees can be iterated over for every odd number, thus defining the set of such walk trees. But I have so far not shown you the edges between such walk trees. These edges can be represented by the relationship:

$$(2a + 1) \cdot 2^k = 6b + 4$$

where $a$ and $b$ are the indexes of the given walk tree.

We create the edges this way because if an odd number points to an even number, then that even number is going to be of the form $6a+4$. We know this because $3n+1$ in the Collatz conjecture is how we get from odd numbers to even numbers, and if $n = 2a+1$ then $3(2a+1)+1$ happens to be $6a+4$.

Lets look at a quick table of such edges to better visualize this.

| a | LHS | LHS-binary | RHS | RHS-binary |
|---|-----|------------|-----|------------|
| 0 | 1   | 1          | 4   | 100        |
| 1 | 3   | 11         | 10  | 1010       |
| 2 | 5   | 101        | 16  | 1000       |
| 3 | 7   | 111        | 22  | 10110      |
| 4 | 9   | 1001       | 28  | 1110       |
| 5 | 11  | 1011       | 34  | 100010     |
| 6 | 13  | 1101       | 40  | 101000     |
| 7 | 15  | 1111       | 46  | 101110     |
| 8 | 17  | 10001      | 52  | 110100     |
| 9 | 19  | 10011      | 58  | 111000     |
| 10| 21  | 10101      | 64  | 1000000    |

You can see that going from left to right we directly have the relationship 3n+1.

We can then use the binary representation to find indirect paths between the walk trees. Like our 10th walk tree points to the 0th walk tree because the binary representation of 64 1000000 looks like the binary representation of 1 if we cut all the zeros off the end.

And the 8th walk tree points to the 6th becaues 52 is 110100 in binary wich looks like 13 (1101) without the ending zeros.

You can also use this table to verify the original equation:

$$(2a + 1) \cdot 2^k = 6b + 4$$

## Graph Topology

We have now defined a condensed Collatz graph consisting of subwalks of the Collatz graph. The interesting thing about grouping the walks like this is that the topology of a graph of walks happens to exactly match the topology of a graph. If you can walk to a walk, then you can walk that walk to wherever that walk goes. This means that if we create a graph of walks, and can prove that graph of walks has no loops. (And that the walks themselves have no loops), then we have proven that the graph we were walking also has no loops.

## Walks of the Condensed Graph

We can go onwards to larger walks like:

$$6a + 4 = (2b+1) \cdot 2^k$$
$$6b + 4 = (2c+1) \cdot 2^l$$
$$6c + 4 = (2d+1) \cdot 2^m$$
$$6d + 4 = (2a+1) \cdot 2^n$$

where $a \neq b \neq c \neq d$

This may seem abstract and unconvincing so lets make it more concrete by trying to punch some values in here. 

I'll start with walking from $x_0 = 0$ as this is the only case in which we get a loop. So from $x_0=0$

We have:

$$6\cdot 0 + 4 = (2\cdot x_1+1)\cdot 2^{k_0}$$

Now obviously the only possible value for $x_1$ is 0. There is always only one possible value in this direction.

Lets start with $x_0 = 4$ and go for a walk. $x_0=4$ represents starting anywhere on the walk tree with the odd number 9 in it and walking down the Collatz graph. From this walk tree we can get to exactly one other walk tree.

$$6\cdot 4 + 4 = (2\cdot x_1+1)\cdot 2^{k_1}$$

$x_1=3$ and $k_1=2$ here clearly. Thus we've gotten to the walk tree with a 7 in it.

$$6\cdot 3 + 4 = (2\cdot x_2+1)\cdot 2^{k_2}$$

$x_2=5$, $k_2=1$, $odd n=11$.

$$6\cdot 5 + 4 = (2\cdot x_3+1)\cdot 2^{k_3}$$

$x_3=8$, $k_3=1$, $odd n=17$

$$6\cdot 8 + 4 = (2\cdot x_4+1)\cdot 2^{k_4}$$

$x_4=6$, $k_4=3$, $odd n=13$

$$6\cdot 6 + 4 = (2\cdot x_5+1)\cdot 2^{k_5}$$

$x_5=2$, $k_5=3$, $odd n=5$

$$6\cdot 2 + 4 = (2\cdot x_6+1)\cdot 2^{k_6}$$

$x_6=0$, $k_6=4$, $odd n=1$

$$6\cdot 0 + 4 = (2\cdot x_7+1)\cdot 2^{k_7}$$

$x_7=0$, $k_7=2$, $odd n=1$

Now obviously at this point we are stuck. No matter how many times we iterate from here we will never get $x_n=4$ which is where we started. Because there simply is no loop in the Collatz graph from 9 to 1. But we want to prove that the above looping equation is unsolvable not just in finite cases that we test, but in all cases for all positive whole number values of x and n.

## Walks that loop

If we wanted to create a system of equations which described a loop in the Collatz graph we would do the following:

We have two series of variables $x_0, x_1, \ldots, x_n$ where all the $x$'s are distinct non-zero positive integers. And another series $k_0, k_1, \ldots, k_n$ where the $k$'s are positive non zero integers but don't need to be distinct. $k_i$ is non zero becaues it represents the number of even steps between odd numbers and there must be at least one such step in the graph.

The equation we need to prove solvability for is:

$$6x_0 + 4 = (2x_1+1) \cdot 2^{k_0}$$
$$6x_1 + 4 = (2x_2+1) \cdot 2^{k_1}$$
$$6x_2 + 4 = (2x_3+1) \cdot 2^{k_2}$$
$$\vdots$$
$$6x_{n-1} + 4 = (2x_n+1) \cdot 2^{k_{n-1}}$$
$$6x_n + 4 = (2x_0+1) \cdot 2^{k_n}$$

Solving for loops
-----------------

### Loops with one odd

Obviously there is only one loop (the trivial one we know of) with a single step as this resolves to

$$6x_0 + 4 = (2x_0+1) \cdot 2^{k_0}$$

Which when re-written as

$$x_0 = \frac{(2x_0+1) \cdot 2^{k_0} - 4}{6}$$

Can be easilly analized and who's only solution is $x_0=0, k=2$.

### N loops

Lets return to our original problem statement:

$$6x_0 + 4 = (2x_1+1) \cdot 2^{k_0}$$
$$6x_1 + 4 = (2x_2+1) \cdot 2^{k_1}$$
$$6x_2 + 4 = (2x_3+1) \cdot 2^{k_2}$$
$$\vdots$$
$$6x_{n-1} + 4 = (2x_n+1) \cdot 2^{k_{n-1}}$$
$$6x_n + 4 = (2x_0+1) \cdot 2^{k_n}$$

$x_n$ are all positive distinct integers greater than zero. $k_n$ are all positive integers greater than 0.

Lets ignore the restrictions on $x_n$ and indeed just limit $k_n$ to be positive integers.

We can rewrite this system of equations by solving for $x_i$ iteratively. So we solve for $x_1$.

$$x_1 = \frac{6x_0 + 4}{2^{k_0+1}} - \frac{1}{2}$$

And We can now eliminate $x_1$ in the next equation as $x_1$ is now defined in terms of $x_0$.

$$6(\frac{6x_0 + 4}{2^{k_0+1}} - \frac{1}{2}) + 4 = (2x_2+1) \cdot 2^{k_1}$$

And now we solve for $x_2$

$$
x_2 = \frac{6(\frac{6x_0 + 4}{2^{k_0+1}} - \frac{1}{2}) + 4}{2^{k_1+1}} - \frac{1}{2}
$$

This will create a kind of telescoping recursive fraction.

One important thing to note is that if we take any set of constant values for $k$ we can plot this as a function:

$$
f(x_0) = \frac{6(\frac{6x_0 + 4}{2^{k_0+1}} - \frac{1}{2}) + 4}{2^{k_1+1}} - \frac{1}{2}
$$

And no matter how many times we add extra steps this will always be a straight line.

A straight line can always be defined in the form:

$$
f(x) = \frac{a}{b} + \frac{rise}{run} \cdot x
$$

In order to find a loop in this structure we would want the input of this function to equal the output. So we find ourselves solving for x in:

$$
x = \frac{a}{b} + \frac{rise}{run} \cdot x
$$

If $x$ is a whole number and the $k$ s are also whole numbers we will have found ourselves a cycle in the structure.

Lets try to solve for a 4 odd loop and we will see that this easilly translates to an N odd cycle.

$$
x = \frac{6\left(\frac{6\left(\frac{6\left(\frac{6x + 4}{2^{k_0+1}} - \frac{1}{2}\right) + 4}{2^{k_1+1}} - \frac{1}{2}\right) + 4}{2^{k_2+1}} - \frac{1}{2}\right) + 4}{2^{k_3+1}} - \frac{1}{2}
$$

Lets convert this to a standard linear equation solving for $a$, $b$, $rise$ and $run$.

$$\frac{rise}{run} = \frac{2^4 \cdot 3^4}{2^{k_0+k_1+k_2+k_3}}$$

$$\frac{a}{b} = \frac{2^3 \cdot 3^3(4-2^{k_0-1})}{2^{k_0+k_1+k_2+k_3}} + \frac{2^2 \cdot 3^2(4-2^{k_1-1})}{2^{k_1+k_2+-k_3}} + \frac{2 \cdot 3(4-2^{k_2-1})}{2^{k_2+k_3}} + \frac{4-2^{k_3-1}}{2^{k_3}}$$

Lets split that constant term up even further:

$$\frac{a}{b} = \frac{2^5 \cdot 3^3}{2^{k_0+k_1+k_2+k_3}}-\frac{3^3 \cdot 2^{k_0+2}}{2^{k_0+k_1+k_2+k_3}} + \frac{2^4 \cdot 3^2}{2^{k_1+k_2+-k_3}}-\frac{3^2 \cdot 2^{k_1+1}}{2^{k_1+k_2+-k_3}} + \frac{2^3 \cdot 3} {2^{k_2+k_3}}-\frac{2^{k_2}\cdot 3}{2^{k_2+k_3}} + \frac{2^2}{2^{k_3}}-\frac{2^{k_3-1}}{2^{k_3}}$$

I'd like to claim here, that if $\frac{a}{b}$ is a whole number it must be divisible by 3 and 2 and nothing else.

We'll now define

$$C = \frac{a}{b}$$

And lets rewrite or equation as:

$$
x = C + \frac{rise}{run}\cdot x
$$

And lets solve for $x$.

$$
x - \frac{rise}{run}\cdot x = C
$$

$$
x(1 - \frac{rise}{run}) = C 
$$

$$
x\frac{run - rise}{run} = C 
$$

$$
x(run - rise) = C \cdot run 
$$

$$
x = \frac {C \cdot run}{run - rise} 
$$

Now $x$ needs to be positive. So $C$ must be negative and $run$ must be less than $rise$ or $C$ must be postive and $run$ must be greater than $rise$.

We can instantly rule out the second case as the definition of $rise$ and $run$ is:

$$\frac{rise}{run} = \frac{2^4 \cdot 3^4}{2^{k_0+k_1+k_2+k_3}}$$

And I happen to know that the average value of $k_i$ cannot be greater than 2.

So we are left with the former case.

Now if $C$ is not a whole number $x$ will not be a whole number. If $C$ is a whole number then it is divisible only by $2$ and $3$. Thus if we can prove that $run - rise$ is divisible by some other number than $2$ and $3$ than we can prove that $x$ is not a whole number.

Lets look at this in mod 3.

We know that rise is $2^n \cdot 3^n$ and run is $2^l$.

$$
2^l ≡ (−1)^l (mod 3)
$$

$$
3^y ≡ 0 (mod 3)
$$

If we write our equation $run - rise$ in mod 3 we get:

$$
0-(-1)^l (mod 3)
$$

And we can be sure that the denominator is never divisible by 3 and $x$ is never a whole number.

Collatz in $2^k$ agnostic arithmatic
------------------------------------

$2^k$ agnostic arithmetic is a special kind of arithmatic where each number represents a set of all numbers that are represented by the equation $(2a+1)2^k$ where $a$ is an integer greater than or equal to zero. 

$2^k$ agnostic arithmatic is best represnted using binary representation. A number might look like:

`101`

or

`1111`.

or just `1` which in $2^k$ agnostic arithmatic represents 1, 2, 4, 8 and any other number that fits the pattern $n2^k$.

Trailing zeros however, are illegal/ignored.

## Multiplication by 3 in $2^k$ agnostic arithmatic

Since trailing zeros are ignored and multiplication by two represents a shift to the left by one digit, multiplication by two does not exist in $2^k$ agnostic arithmatic. It is, however possible to mulitply by three.

Imagine a situation where you have:

`101 * 11` That is 5 times 3.

You can do this simply by doing:

```
 101
 101
+101
----
```

The first two numbers when added together end up being the equivalent of a left shift so you end up with:

```
1010
+101
----
1111
```

Unlike multiplication by two, multiplication by 3 makes sense in this form of arithmatic.

Multiplication by 3 is the same as adding either a right shifted or left shifted version of the number to the number. It really doesn't matter which direction you shift it as the result will be the same either way.

Like it can either be:

```
 101
+ 101
```

Or

```
  101
+101
```

So that's multiplication by 3. In order to represent the Collatz tranformation we also need to add 1. But this works exactly as it does in ordinary binary. It's just important to make sure you line up the least significant digits.

You can transition between various walktrees in the condensed Collatz graph by doing 3n+1 in $2^k$ agnostic arithmatic.

```
 101
  101
+   1
-----
1
```

This represents the transition between 5 and 1. Note the lack of trailing zeros.

5 happens to be just one step from one but the sequence does tend to be longer consisting of multiple iterations of "add shift right" and +1.

If we place these binary sequences on a grid and run just the the "add shift right" operation over and over again we get something interesting. A crystaline structure of 1's and zeros with larger crystals of triangular shape sometimes included. Various crystaline and "metalic" structures appear depending on the initial binary sequence that that we enter.

If we then add the +1 operation, we see that this crystaline structure gets eaten or cut from the right to the left, except when we come across a larger dark triangular crystal at which point the structure is cut verically untill the dark triangle is consumed.

The only time we cut vertically is when we are cutting through dark crystals. Otherwise we are cutting sideways. The height of the cut crystals is always the same as their width.

We also see the crystaline structure growing slowly in the rightward direction. Basically, the whole conjecture comes down to if this cutting is faster than that growth.

We notice that adding new bits on the right to our initial sequence doesn't really effect things on the left unless there are carries. If there are carries crystals form. The possibility of a proof by induction that starts with a small sequence and grows it rightward occures to me, but there is a much easier proof of collatz.

It turns out that if you draw a vertical line in the grid where the initial 1 ocures. Then all the leftward crystal growth comes from one of two effects:

- carries
- add shift rights

Now there is at most one carry per iteration, and the add shift rights are equivalent to adding a given subsequence divided by two (rounded up). Thus the maximum binary value of each binary subsequence to the right of our imaginary virtual line is 

```
  1+1/2 (cumulatively 1)
  1+1/2 (cumulatively 2)
  1+2/2 (cumulatively 4)
  1+4/2 (cumulatively 7)
  1+7/2 (cumulatively 11)
 ...
 ```

 Which is equivalent to the subsequence to the left of our imaginary vertical line being equal to at most $3(1.5)^i-2$. We will refer to this value as $n_1$ and the value of our initial binary sequence as $n_0$.

 Now as we are cutting to the left by at least one colum any time we are not in a dark crystal, and between any dark crystals. We can calculate the maximum number of iterations before we reach this subsequence. Which is the maximum size of a dark crystal times the number of columns (which is equal to the number of bits in our initial sequence). I'm going to refer to the number of bits in our initial sequence as $l$ and for now lets just assume that the maximum crystal size is $l$ (it could be larger, up to $l+i$ but for now lets go with this reasonable estimate). If the maximum crystal size is $l$ and we end up spending $l$ iterations cutting vertically through a crystal of size $l$ then we will spend $l*l$ iterations to cut through the maximum number of crystals. Given a worse case scenario that the crystals are 1 colum appart, which is impossibly pessimistic.

 If we go with this estimate. Then the maximum value of $n_1$ is $3(1.5)^{l*l}-2$.
