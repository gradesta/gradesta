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

When we do this, we can then represent all of the even numbers "to the left" of our odd number (as well as our odd number) with the expression $(2a+1) \cdot 2^k$. That said, the starting edge in our walk tree is U and that is actually an edge from the odd number to one other even number. So we have one even number, then an edge labeled U, and then an odd number, and then a bunch of edges labeled → that all point from even numbers in each walk tree.

Just as when iterating from 0 we can iterate through the odd numbers using the expression $2a+1$, when iterating through the left-hand side even numbers we can use the expression $6a+4$. For example, $6 \cdot 0 + 4 \xrightarrow{U} 2 \cdot 0 + 1$ represents the connection between 4 and 1.

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

We have now defined a condensed Collatz graph consisting of subwalks of the Collatz graph. The interesting thing about grouping the walks like this is that the topology of a graph of walks happens to exactly match the topology of a graph. If you can walk to a walk, then you can walk that walk to wherever that walk goes.

## Walks of the Condensed Graph

We can go onwards to larger walks like:

$$6a + 4 = (2b+1) \cdot 2^k$$
$$6b + 4 = (2c+1) \cdot 2^l$$
$$6c + 4 = (2d+1) \cdot 2^m$$
$$6d + 4 = (2a+1) \cdot 2^n$$

where $a \neq b \neq c \neq d$

and we eventually get an equation like the following:

We have two series of variables $x_0, x_1, \ldots, x_n$ where all the $x$'s are distinct non-zero positive integers. And another series $k_0, k_1, \ldots, k_n$ where the $k$'s are positive non zero integers but don't need to be distinct. $k_i$ is non zero becaues it represents the number of even steps between odd numbers and there must be at least one such step in the graph.

The equation we need to prove solvability for is:

$$6x_0 + 4 = (2x_1+1) \cdot 2^{k_0}$$
$$6x_1 + 4 = (2x_2+1) \cdot 2^{k_1}$$
$$6x_2 + 4 = (2x_3+1) \cdot 2^{k_2}$$
$$\vdots$$
$$6x_{n-1} + 4 = (2x_n+1) \cdot 2^{k_{n-1}}$$
$$6x_n + 4 = (2x_0+1) \cdot 2^{k_n}$$

This may seem abstract and unconvincing so lets make it more concrete by trying to punch some values in here. 

I'll start with walking from $x_0 = 0$ as this is the only case in which we get a loop. So from $x_0=0$

We have:

$$6\cdot 0 + 4 = (2\cdot x_1+1)\cdot 2^{k_0}$$

Now obviously the only possible value for $x_1$ is 0. There is always only one possible value in this direction.

Lets start with $x_0 = 4$ and go for a walk. $x_0=4$ represents starting anywhere on the walk tree with the odd number 9 in it and walking down the Collatz graph. From this walk tree we can get to exactly one other walk tree.

$$6\cdot 4 + 4 = (2\cdot x_1+1)\cdot 2^{k_1}$$

$x_1=3$ and $k_1=2$ here clearly. Thus we've gotten to the walk tree with a 7 in it.

$$6\cdot 3 + 4 = (2\cdot x_2+1)\cdot 2^{k_2}$$

$x_2=5$, $k_2=1$.

$$6\cdot 5 + 4 = (2\cdot x_3+1)\cdot 2^{k_3}$$

$x_3=8$, $k_3=1$

$$6\cdot 8 + 4 = (2\cdot x_4+1)\cdot 2^{k_4}$$

$x_4=6$, $k_4=3$

$$6\cdot 6 + 4 = (2\cdot x_5+1)\cdot 2^{k_5}$$

$x_5=2$, $k_5=3$

$$6\cdot 2 + 4 = (2\cdot x_6+1)\cdot 2^{k_6}$$

$x_6=0$, $k_6=4$

$$6\cdot 0 + 4 = (2\cdot x_7+1)\cdot 2^{k_7}$$

$x_7=0$, $k_7=2$

Now obviously at this point we are stuck. No matter how many times we iterate from here we will never get $x_n=4$ which is where we started. Because there simply is no loop in the Collatz graph from 9 to 1. But we want to prove that this equation is unsolvable not just in finite cases that we test, but in all cases for all positive whole number values of x and n.


Solving for $x_{n+1}$
----------------------

Starting from our equation:

$$6x_{n} + 4 = (2x_{n+1}+1) \cdot 2^{k_n}$$

We devide by $2^{k_n}$

$$
\frac{6x_{n} + 4}{2^{k_n}} = 2x_{n+1} + 1
$$

Then we subtract by 1.

$$
\frac{6x_{n} + 4}{2^{k_n}} - 1 = 2x_{n+1}
$$

And devide by 2

$$
\frac{6x_{n} + 4}{2^{k_n+1}} - 0.5 = x_{n+1}
$$

And reduce the fraction.

$$
\frac{3x_{n} + 2}{2^{k_n}} - 0.5 = x_{n+1}
$$

Now we know $x_{n+1}$ must be a whole number, so we're going to want to end up with the fraction resolving to $x_{n+1}.5$

If the nominator is odd then $k_n$ must be 1. If it is even, we must keep on dividing by two untill we get to a whole number.

Lets look at how this looks in binary.

Imagine $x_n$ is 5

```
0101
```

We multiply by 3 and add 2. This is best visualized as simply 3 binary addition operations.

```
 00101
 00101
 00101
+00010
-----

```

```
 00101
 00101
=01010
 00101
=01111
+00010
-----
=10001
```

Now in this case our result is odd. So $k$ must be 1.

In this case, to get $x_{n+1}$ we simply snip off the last 1 to get:

$$
1000\sout{1}\\
x_{n+1}=1000
$$

Or 8.

Now lets do this again with our 8.

```
 01000
 01000
 01000
+00010
-----
=11010
```

In this case, $k = 2$. If $x_n$ is odd, $k = 1$. If $x_n$ is even, $k \geq 2$.

$$
110\sout{10}\\
x_{n+1}=110
$$

```
 00110
 00110
 00110
=10010
+00010
-----
=10100
```

This is an interesting case in that the $+2$ step pushed one of the ones farther to the left thus increasing the value of $k$ from $2$ to $3$, so now we strike out 3 binary digits from the left.

$$
10\sout{100}\\
x_{n+1}=10
$$

Here we gained two digits on the right, and lost 3 digits on the left.

We can understand the loop in the Collatz Cycle as follows.

```
 0000
 0000
 0000
+0010
-----
 0010
```

$$
x_n=0\\
0\sout{10}\\
x_{n+1}=0
$$

The possibility of endless growth or loops
------------------------------------------

If we can somehow figure out the growth on the right and prove that it is always less than culling on the right over time, then we will have proven the Collatz conjecture.

So far we have a variable $k$ for culling on the right. I'd like to define a new variable $g$ for growth on the left. $g$ is equal to the number of extra digits we gain on the left before culling. I will also define a variable $l$ which is the length of the binary number in digits. So $0100$ has an $l$ of 3.

```
 00100
 00100
 00100
+00010
-----
=01110
```

Has a $g$ of 1 and a $k$ of 2. $l+g-k=2$ in this case.

I want this proof to be simple enough that ordinary people can understand it, so I am going use a table to compare possible values for $g$ and $k$ and see if I can prove that the cumulative value $\sum{g}<\sum{k}$.

Obviously over a single step $g$ can be greater than $k$ but what about over 4 steps?

What are the possible values of $g$?

If the first two digits are:

|0011|0010|
|----|----|
|1001|0110|
| g=2| g=1|

Even if we were to carry from below. these results don't seem to change:

|0011|0010|with 1 carry|
|----|----|-|
|1010|0111||
| g=2| g=1||

|0011|0010|with 2 carries|
|----|----|-|
|1011|1000||
| g=2| g=2||

So $g$ is always either 1 or 2.

Furthermore, we know that if $g$ is 2 than the next $g$ is 1. If $g$ is 1 then the next $g$ is 2 or ocasionally 1 is there is a double carry.

Finally. Two carries can only come from the +2 step pushing 1's all the way from the right. This means that if there are two carries, $k$ will either be 1 (if $x_n$ is odd), or it will be $l+g$ (thus landing us at $x_{n+1}=0$).

$k$ is always somewhere between 1 and $l+g$

$$1\leq k \leq l+g$$

Right now we can confidently say that:

$$
\sum_{i=1}^{n} g_i \leq \frac n 2 \cdot 3
$$

Lets make a table for the right side. This table is simpler because there are no carries.

|step |0000|0001|0010|0011|0100|0101|0110|0111|1000|1001|1010|1011|1111|
|-----|----|----|----|----|----|----|----|----|----|----|----|----|----|
|    1|0010|0101|1000|1011|1110|0001|0100|0111|0010|0100|1000|0010|1111|
|    k|   2|   1|   4|   1|   2|   1|   3|   1|   2|   3|   4|   2|   1|
|start|??00|?010|????|?101|??11|?000|???0|?011|??00|???0|????|??00|?111|
|    2|??10|?000|????|?001|??11|?010|???0|?101|??10|???0|????|??10|?111|
|    k|   2|  ≥3|  ≥1|   1|   1|   2|  ≥2|   1|   2|  ≥2|  ≥1|   2|   1|
|start|????|????|????|??00|???1|???1|????|??10|????|????|????|????|??11|
|    3|????|????|????|??10|???1|???1|????|??00|????|????|????|????|??01|
|    k|  ≥1|  ≥1|  ≥1|   2|   1|   1|  ≥1|  ≥3|  ≥1|  ≥1|  ≥1|  ≥1|   1|
|total|  ≥5|  ≥5|  ≥6|   4|   4|   4|  ≥6|  ≥5|  ≥5|  ≥6|  ≥6|  ≥5|   3|

If we were to then fill the ? marks with the least favorable column here (straight ones `1111`) the Collatz conjecture would not hold. We would grow faster than we ate.

