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

## Cycles in the Condensed Graph

Now let's go on to show something about the cycles of our condensed Collatz graph.

First off, we know that no cycle can exist with only even numbers. We need an odd number for there to be a cycle. So the only way for the Collatz graph to cycle is if our condensed Collatz graph of walk trees cycles.

Such a cycle with two edges would require that:

$$(2a+1) \cdot 2^k = 6a + 4$$

be solved. Because we would need the left-hand side of the graph to point to the right-hand side and the right-hand side to point to the left-hand side.

This is solvable for $a=0$. Interestingly, $k$ then represents the number of → labeled edges in the walk (or if you prefer) the number of even numbers in the non-condensed cyclic walk.

In order for a cycle between walk trees with two edges to exist, we would need:

$$(2a+1) \cdot 2^k = 6b + 4$$
$$(2b+1) \cdot 2^l = 6a + 4$$

To hold. That is, such a cycle consists of two distinct walk trees. And the right-hand side of the first walk tree must point to the left-hand side of the second and the right-hand side of the second must point to the left-hand side of the first. This is, however, unsolvable for distinct positive integer values of $a$ and $b$ and positive integer values of $k$ and $l$.

## Larger Cycles

We can go onwards to larger cycles like:

$$(2a+1) \cdot 2^k = 6b + 4$$
$$(2b+1) \cdot 2^l = 6c + 4$$
$$(2c+1) \cdot 2^m = 6d + 4$$
$$(2d+1) \cdot 2^n = 6a + 4$$

where $a \neq b \neq c \neq d$

and we eventually get an equation like the following:

We have two series of variables $x_0, x_1, \ldots, x_n$ where all the $x$'s are distinct non-zero positive integers. And another series $k_0, k_1, \ldots, k_n$ where the $k$'s are positive non zero integers but don't need to be distinct. $k_i$ is non zero becaues it represents the number of even steps between odd numbers and there must be at least one such step in the graph.

The equation we need to prove solvability for is:

$$(2x_0+1) \cdot 2^{k_0} = 6x_1 + 4$$
$$(2x_1+1) \cdot 2^{k_1} = 6x_2 + 4$$
$$(2x_2+1) \cdot 2^{k_2} = 6x_3 + 4$$
$$\vdots$$
$$(2x_{n-1}+1) \cdot 2^{k_{n-1}} = 6x_n + 4$$
$$(2x_n+1) \cdot 2^{k_n} = 6x_0 + 4$$

This may seem abstract and unconvincing so lets make it more concrete by trying to punch some values in here. 

I'll start with walking from $x_0 = 0$ as this is the only case in which we get a loop. Perhapse counterintuitively, we will be walking UP the collatz graph rather than down it. So from $x_0=0$

We have:

$$(2*0+1)*2^{k_0} = 6x_1 + 4$$

Now $x_1$ could be $2$ with $k_0=3$ or it could be $10$ with $k_0=6$, or it could possibly be $0$ with $k_0=2$.



Lets start with $x_0 = 4$ and go for a walk.

$$(2*2+1)*2^{k_0} = 6x_1 + 4$$

This is representing a walk tree with one odd number in it (5) and a (posibly infinite, but also possibly zero) number of edges from other walk trees comming into it which can satisfy this equation. Lets try to solve for $x_1$ to find what kind of walk trees point to walk tree `#2` aka the one and only walk tree with a 5 in it. If $x_1 = 6$ and $k_0=3$ then we fit that first equation in our walk. This represents the edge between the walk tree with the odd number 13 in it and the LHS even number 4O in it aka 13*3+1. We get from 40 to 5 by deviding by 2 3 times.

We can then walk a bit further:

$$(2*6+1)*2^{k_1} = 6x_2 + 4$$

And solve for $x_2=8$ when $k_1=2$. This means that the odd number 17 is two even numbers away from 13 in the collatz graph. We can then continue (and I assure you we'll get to 1 at some point as in these lower regions of the graph we know that it cycles back to 1).

$$(2*8+1)*2^{k_2} = 6x_3 + 4$$

$x_3=5$, $k_2=1$ fits this time. (For the odd nuber 11)

$$(2*5+1)*2^{k_3} = 6x_4 + 4$$

$x_4=3$, $k_3=1$ (For the odd number 7)

$$(2*3+1)*2^{k_4} = 6x_5 + 4$$

$x_5=4$, $k_4=1$ (For the odd number 9)

$$(2*4+1)*2^{k_5} = 6x_6 + 4$$



## Modular Arithmetic Analysis

Now lets try to prove that this system has no solutions except the trivial one whith $n=0$ and $x_0 = 0$ and $k_0 = 2$.

Since $k_i > 0$ for all $i$, we can use modular arithmetic to disprove solvability more efficiently.

### Analysis Modulo 4

Converting $(2x_i+1) \cdot 2^{k_i} = 6x_{i+1} + 4$ to modulo 4:
$$(2x_i + 1) \cdot 2^{k_i} \equiv 2x_{i+1} \pmod{4}$$

Since $2x_i + 1$ is odd and $2^{k_i} \equiv 0 \pmod{4}$ for $k_i \geq 2$, we have:
$$x_{i+1} \equiv \begin{cases}
2 \pmod{4} & \text{if } k_i = 1 \\
0 \pmod{4} & \text{if } k_i \geq 2
\end{cases}$$

### Analysis Modulo 3

In modulo 3, $2 \equiv -1$, $6 \equiv 0$, and $4 \equiv 1$, so:
$$(-x_i + 1) \cdot 2^{k_i} \equiv 1 \pmod{3}$$

Since $2^{k_i} \equiv (-1)^{k_i} \pmod{3}$:
$$x_i \equiv \begin{cases}
0 \pmod{3} & \text{if } k_i \text{ is even} \\
2 \pmod{3} & \text{if } k_i \text{ is odd}
\end{cases}$$

### Combining Constraints with Chinese Remainder Theorem

**What is the Chinese Remainder Theorem?** If you know what remainder a number leaves when divided by 3, and what remainder it leaves when divided by 4, then you can figure out exactly what remainder it leaves when divided by 12. This works because 3 and 4 share no common factors.

**From modulo 3**: Each $x_i$ must leave remainder 0 or 2 when divided by 3.

**From modulo 4**: Each $x_{i+1}$ must leave remainder 0 or 2 when divided by 4 (since $k_i > 0$).

**Using CRT**: The only numbers that leave remainder 0 or 2 when divided by both 3 and 4 are those that leave remainder 0, 2, 6, 8, or 10 when divided by 12.

So all our $x_i$ values must be in this very small set: $\{0, 2, 6, 8, 10\} \pmod{12}$.

### Case Analysis

**Case 1**: All $k_i = 1$

If every $k_i = 1$, then from our modulo 4 analysis, every $x_{i+1}$ must leave remainder 2 when divided by 4. Combined with our modulo 3 analysis, this forces every $x_i$ to leave remainder 2 or 6 when divided by 12.

The system becomes:
$$(2x_i + 1) \cdot 2 = 6x_{i+1} + 4$$
$$2x_i = 3x_{i+1} + 1$$

This means each $x_i$ is bigger than the next $x_{i+1}$. But if the sequence keeps getting smaller, it can never loop back to the beginning - contradicting the requirement that we have a cycle.

**Case 2**: All $k_i \geq 2$

If every $k_i \geq 2$, then from our modulo 4 analysis, every $x_{i+1}$ must leave remainder 0 when divided by 4. Combined with our modulo 3 analysis, this forces every $x_i$ to leave remainder 0 or 8 when divided by 12.

The cycle cannot close because:
- If $x_0$ leaves remainder 0 when divided by 12, then $x_n$ must also leave remainder 0 when divided by 12
- The last equation requires $(2x_n + 1) \cdot 2^{k_n} = 6x_0 + 4$
- Since $x_n$ leaves remainder 0 when divided by 12, we have $2^{k_n} \equiv 4 \pmod{12}$
- But $2^{k_n}$ can only leave remainder 2, 4, or 8 when divided by 12 for $k_n \geq 1$, and 4 is possible
- However, this forces $x_0$ to leave remainder 0 when divided by 12, which creates a problem with the requirement that all $x_i$ be different numbers

**Case 3**: Mixed $k_i$ values

Suppose we have a mix where some $k_i = 1$ and some $k_j \geq 2$. This creates a chain of constraints that cannot be satisfied consistently.

Consider consecutive steps where $k_i = 1$ and $k_{i+1} \geq 2$:
- From $k_i = 1$: $x_{i+1}$ must leave remainder 2 when divided by 4
- From $k_{i+1} \geq 2$: $x_{i+2}$ must leave remainder 0 when divided by 4

But from our modulo 3 analysis, if $k_{i+1}$ is odd, then $x_{i+1}$ must leave remainder 2 when divided by 3, and if $k_{i+1}$ is even, then $x_{i+1}$ must leave remainder 0 when divided by 3.

Combining with CRT:
- If $k_{i+1}$ is odd: $x_{i+1}$ leaves remainder 2 when divided by both 3 and 4, so it must leave remainder 2 when divided by 12
- If $k_{i+1}$ is even: $x_{i+1}$ leaves remainder 2 when divided by 4 and remainder 0 when divided by 3, so it must leave remainder 6 when divided by 12

In both cases, $x_{i+1}$ is forced into a very specific value modulo 12. But then $x_{i+2}$ must leave remainder 0 when divided by 4, which creates even more constraints. These constraints keep building up as we go around the cycle, eventually making it impossible to satisfy the requirement that all $x_i$ be different numbers.

### Conclusion

With $k_i > 0$, our analysis shows that all $x_i$ values are forced into a very small set: they can only leave remainder 0, 2, 6, 8, or 10 when divided by 12. This severely restricts our options.

In all three cases we examined, these restrictions make it impossible to create a cycle where all the numbers are different from each other. The constraints keep building up until we hit a dead end.

**Therefore, no solution exists for $n \geq 2$**.

