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

### Two loops

$$6x_0 + 4 = (2x_1+1) \cdot 2^{k_0}$$
$$6x_1 + 4 = (2x_0+1) \cdot 2^{k_1}$$

Multiply terms:

$$(6x_0 + 4)(6x_1 + 4) = (2x_1+1)(2x_0+1) \cdot 2^{k_0+k_1}$$

Simplify ↓

$$(3x_0 + 2)(3x_1 + 2) = (2x_0+1)(2x_1+1) \cdot 2^{k_0+k_1-2}$$

Spread the terms:

$$m = k_0+k_1-2$$

$$(3x_0 + 2)(3x_1 + 2) = (2x_0\cdot 2^{\frac{m}{2}}+2^{\frac{m}{2}})(2x_1\cdot 2^{\frac{m}{2}}+2^{\frac{m}{2}})$$

Here we see that if $m$ is less than or equal to 1 then the right side will be lower. If it is 2 it will be higher. Thus this equation is unsolvable. Therefore there are no loops in Collatz with only two odds.

### Three loops

$$6x_0 + 4 = (2x_1+1) \cdot 2^{k_0}$$
$$6x_1 + 4 = (2x_2+1) \cdot 2^{k_1}$$
$$6x_2 + 4 = (2x_0+1) \cdot 2^{k_2}$$

Multiply the terms:

$$(6x_0 + 4)(6x_1 + 4)(6x_2 + 4)=(2x_1+1)(2x_2+1)(2x_0+1) \cdot 2^{k_0+k_1+k_2}$$

Simplify ↓

$$(3x_0 + 2)(3x_1 + 2)(3x_2 + 2)=(2x_0+1)(2x_1+1)(2x_2+1) \cdot 2^{k_0+k_1+k_2-3}$$

Simplify more explicitly ↓

$$m = k_0+k_1+k_2-3$$

$$(3x_0 + 2)(3x_1 + 2)(3x_2 + 2)=(2x_0\cdot 2^{\frac{m}{3}}+2^{\frac{m}{3}})(2x_1\cdot 2^{\frac{m}{3}}+2^{\frac{m}{3}})(2x_2\cdot 2^{\frac{m}{3}}+2^{\frac{m}{3}})$$

If $\frac{m}{3} <= \frac{1}{3}$ then the LHS is higher. If $\frac{m}{3} >= \frac{3}{3}$ then the RHS is higher. However deeper analysis is needed for the case of $\frac{m}{3} >= \frac{2}{3}$ as this resolves to $2^\frac{2}{3} = 1.587...$ or 

$$(3x_0 + 2)(3x_1 + 2)(3x_2 + 2)=(3.174...x_0+1.587...)(3.174...x_1+1.587...)(3.174...x_2+1.587...)$$

And without knowing the values for $x_i$ its not clear which side is greater or less (or if they could perhaps be equal). Lets just imagine for a second though that all $x_i=2$:

$$(6 + 2)(6 + 2)(6 + 2)=(6.34...+1.587...)(6.34...x_1+1.587...)(6.34+1.587...)$$

$512 > 500$

And for $x_i=3$:

$1331 < 1372$

What if we make one of the $x_i=2$?

$1331 > 980$

$x_0=2, x_1=3, x_2=4$

$1331 > 1260$

$x_0=2, x_1=3, x_2=5$

$1331 > 1540$

Lets try to loosen our limitations on $x_i$ and really solve for what these $x_i$ could be including on-integer solutions but first lets convert our equation into a polynomial.

$$(3x_0 + 2)(3x_1 + 2)(3x_2 + 2) = 2x_0\cdot 2^{\frac{m}{3}} + 2^{\frac{m}{3}})(2x_1\cdot 2^{\frac{m}{3}} + 2^{\frac{m}{3}})(2x_2\cdot 2^{\frac{m}{3}} + 2^{\frac{m}{3}}$$

Expanding both sides:

Left side:
$$
\begin{align*}
(3x_0 + 2)(3x_1 + 2)(3x_2 + 2) &= (3x_0)(3x_1)(3x_2) + (3x_0)(3x_1)2 + (3x_0)2(3x_2) + (3x_0)2 \cdot 2 \\
&\quad + 2(3x_1)(3x_2) + 2(3x_1)2 + 2(3x_2)2 + 2 \cdot 2 \cdot 2 \\
&= 27x_0x_1x_2 + 18x_0x_1 + 18x_0x_2 + 12x_0 + 18x_1x_2 + 12x_1 + 12x_2 + 8
\end{align*}
$$

Right side:
Let $a = 2^{\frac{m}{3}}$ for brevity.

$$
\begin{align*}
(2x_0 a + a)(2x_1 a + a)(2x_2 a + a) &= [2x_0 a + a][2x_1 a + a][2x_2 a + a] \\
&= (2x_0 a + a)(2x_1 a + a)(2x_2 a + a) \\
&= (2x_0 a)(2x_1 a)(2x_2 a) + (2x_0 a)(2x_1 a)a + (2x_0 a)a(2x_2 a) + (2x_0 a)a a \\
&\quad + a(2x_1 a)(2x_2 a) + a(2x_1 a)a + a a(2x_2 a) + a a a \\
&= 8x_0x_1x_2 a^3 + 4x_0x_1 a^3 + 4x_0x_2 a^3 + 2x_0 a^3 \\
&\quad + 4x_1x_2 a^3 + 2x_1 a^3 + 2x_2 a^3 + a^3 \\
&= a^3 \left(8x_0x_1x_2 + 4x_0x_1 + 4x_0x_2 + 2x_0 + 4x_1x_2 + 2x_1 + 2x_2 + 1\right)
\end{align*}
$$

So the fully expanded polynomial equation is:

$$
27x_0x_1x_2 + 18x_0x_1 + 18x_0x_2 + 12x_0 + 18x_1x_2 + 12x_1 + 12x_2 + 8 =  8x_0x_1x_2a^3 + 4x_0x_1a^3 + 4x_0x_2a^3 + 2x_0a^3 + 4x_1x_2a^3 + 2x_1a^3 + 2x_2a^3 + a^3
$$

where $a = 2^{\frac{m}{3}}$.

And filling in $a$ for this particular example:

$$
27x_0x_1x_2 + 18x_0x_1 + 18x_0x_2 + 12x_0 + 18x_1x_2 + 12x_1 + 12x_2 + 8 =  32x_0x_1x_2 + 16x_0x_1 + 16x_0x_2 + 8x_0 + 16x_1x_2 + 8x_1 + 8x_2 + 4
$$

We can then subtract by 4 on both sides and simplify a bit:

$$
4 = (32x_0x_1x_2 + 16(x_0x_1 + x_0x_2 +  x_1x_2) + 8(x_0 + x_1 + x_2)) - (27x_0x_1x_2 + 18(x_0x_1 + x_0x_2 + x_1x_2) + 12(x_0 + x_1 + x_2))
$$

So certainly the RHS must be positive and $32x_0x_1x_2 > 27x_0x_1x_2$ but $16(x_0x_1 + x_0x_2 +  x_1x_2) + 8(x_0 + x_1 + x_2) < 18(x_0x_1 + x_0x_2 + x_1x_2) + 12(x_0 + x_1 + x_2)$

So if we rewrite as:

$$
4 = 5x_0x_1x_2 - 2(x_0x_1 + x_0x_2 +  x_1x_2) - 4(x_0 + x_1 + x_2)
$$


### N loops

Take the expression

$$m = k_0+k_1+k_2...k_n-n$$

$$(3x_0 + 2)(3x_1 + 2)...(3x_n + 2)=(2x_1\cdot 2^{\frac{m}{n}}+2^{\frac{m}{n}})(2x_0\cdot 2^{\frac{m}{n}}+2^{\frac{m}{n}})...(2x_n\cdot 2^{\frac{m}{n}}+2^{\frac{m}{n}})$$

The only possible values for $\frac{m}{n}$ are those where the coeficient of the binomials on the RHS are greater than the coeficients on the LHS while the contsants on the RHS are less than those on the LHS.

The maximum value for $\frac{m}{n}$ is thus 

$$\displaystyle \lim_{\frac{m}{n} \to 1} 2^{\frac{m}{n}} = 2$$

The minimum is when

$$\displaystyle \lim_{\frac{m}{n} \to \log_2 3 - 1} 2^{\frac{m}{n} + 1} = 3$$

or equivalently, when $\frac{m}{n} \to \log_2 1.5$ so that $2^{\frac{m}{n}} \to 1.5$.

So $\log_2 1.5<\frac{m}{n}<1$

$$0.584...<\frac{m}{n}<1$$

The key to understanding the unsolvability of this equation is that the constants in the RHS binomials is at best only slightly less than the constants on the LHS. 

If this analysis is correct, then there are no loops other than the trivial loop, in the Collatz graph. We still haven't proven, however, that the sequence does not diverge.

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
