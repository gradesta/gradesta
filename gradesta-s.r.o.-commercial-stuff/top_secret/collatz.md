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

We have two series of variables $x_0, x_1, \ldots, x_n$ where all the $x$'s are distinct non-zero positive integers. And another series $k_0, k_1, \ldots, k_n$ where the $k$'s are positive integers but don't need to be distinct.

The equation we need to prove solvability for is:

$$(2x_0+1) \cdot 2^{k_0} = 6x_1 + 4$$
$$(2x_1+1) \cdot 2^{k_1} = 6x_2 + 4$$
$$(2x_2+1) \cdot 2^{k_2} = 6x_3 + 4$$
$$\vdots$$
$$(2x_{n-1}+1) \cdot 2^{k_{n-1}} = 6x_n + 4$$
$$(2x_n+1) \cdot 2^{k_n} = 6x_0 + 4$$

**Condensed Product Form:**

Multiplying all equations together gives:

$$\prod_{i=0}^{n} \left((2x_i + 1) \cdot 2^{k_i}\right) = \prod_{i=0}^{n} (6x_i + 4)$$

where $x_{n+1} = x_0$ due to the cyclic nature.

This can be expanded as:

$$\prod_{i=0}^{n} (2x_i + 1) \cdot 2^{\sum_{i=0}^{n} k_i} = \prod_{i=0}^{n} (6x_i + 4)$$

Or equivalently:

$$\left(\prod_{i=0}^{n} (2x_i + 1)\right) \cdot 2^{\sum_{i=0}^{n} k_i} = \prod_{i=0}^{n} (6x_i + 4)$$

Dividing both sides by $\prod_{i=0}^{n} (6x_i + 4)$:

$$\frac{\left(\prod_{i=0}^{n} (2x_i + 1)\right) \cdot 2^{\sum_{i=0}^{n} k_i}}{\prod_{i=0}^{n} (6x_i + 4)} = 1$$

Let $K = \sum_{i=0}^{n} k_i$ for easier analysis:

$$\frac{\left(\prod_{i=0}^{n} (2x_i + 1)\right) \cdot 2^K}{\prod_{i=0}^{n} (6x_i + 4)} = 1$$

## Conclusion

If we can prove that this equation has no solutions, then we should have proven that there are no unknown cycles in Collatz.