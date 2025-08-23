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

## Modular Arithmetic Analysis

Let's convert this system to modular arithmetic on mod 3 and mod 4 to disprove solvability.

### What is Modular Arithmetic?

Modular arithmetic (also called "clock arithmetic") works with remainders after division. When we say $a \equiv b \pmod{m}$, we mean that $a$ and $b$ leave the same remainder when divided by $m$. For example, $7 \equiv 2 \pmod{5}$ because both 7 and 2 leave remainder 2 when divided by 5.

**Why modular arithmetic holds:** If $a \equiv b \pmod{m}$, then $a - b$ is divisible by $m$. This means we can substitute $a$ with $b$ in equations modulo $m$ without changing the truth of the equation. This is incredibly useful for finding contradictions in systems of equations.

### Analysis Modulo 4

Converting our equation $(2x_i+1) \cdot 2^{k_i} = 6x_{i+1} + 4$ to modulo 4:
$$(2x_i + 1) \cdot 2^{k_i} \equiv 2x_{i+1} \pmod{4}$$

Since $2x_i + 1$ is always odd, and $2^{k_i} \equiv 0 \pmod{4}$ for $k_i \geq 2$, we have:

$$x_{i+1} \equiv \begin{cases}
2x_i + 1 \pmod{4} & \text{if } k_i = 0 \\
2 \pmod{4} & \text{if } k_i = 1 \\
0 \pmod{4} & \text{if } k_i \geq 2
\end{cases}$$

This gives us constraints on the parity of $x_{i+1}$ based on the value of $k_i$.

### Analysis Modulo 3

In modulo 3:

$$\begin{cases}
2 \equiv -1 \pmod{3} & \text{(because 2 and -1 both leave remainder 2 when divided by 3)} \\
6 \equiv 0 \pmod{3} & \text{(because 6 is divisible by 3)} \\
4 \equiv 1 \pmod{3} & \text{(because 4 leaves remainder 1 when divided by 3)}
\end{cases}$$

Converting our equation $(2x_i+1) \cdot 2^{k_i} = 6x_{i+1} + 4$ to modulo 3:
$$(-x_i + 1) \cdot 2^{k_i} \equiv 1 \pmod{3}$$

Since $2 \equiv -1 \pmod{3}$, we have $2^{k_i} \equiv (-1)^{k_i} \pmod{3}$. Therefore:
$$(-x_i + 1) \cdot (-1)^{k_i} \equiv 1 \pmod{3}$$

This means:

$$x_i \equiv \begin{cases}
0 \pmod{3} & \text{if } k_i \text{ is even} \\
2 \pmod{3} & \text{if } k_i \text{ is odd}
\end{cases}$$

### Combining Modulo 3 and Modulo 4 Constraints

Now let's combine our findings from both modular analyses to find contradictions.

**From modulo 3**: Each $x_i$ must be either $0 \pmod{3}$ or $2 \pmod{3}$ (depending on whether $k_{i-1}$ is even or odd).

**From modulo 4**: Each $x_{i+1}$ must satisfy:
$$x_{i+1} \equiv \begin{cases}
2x_i + 1 \pmod{4} & \text{if } k_i = 0 \\
2 \pmod{4} & \text{if } k_i = 1 \\
0 \pmod{4} & \text{if } k_i \geq 2
\end{cases}$$

**Key Insight**: The modulo 4 constraints create a chain reaction. If any $k_i \geq 2$, then $x_{i+1} \equiv 0 \pmod{4}$. But from modulo 3, $x_{i+1}$ must also be either $0 \pmod{3}$ or $2 \pmod{3}$.

**Combined Constraint**: If $k_i \geq 2$, then $x_{i+1} \equiv 0 \pmod{4}$ AND $x_{i+1} \equiv 0 \pmod{3}$ or $2 \pmod{3}$. This means $x_{i+1} \equiv 0 \pmod{12}$ or $x_{i+1} \equiv 8 \pmod{12}$.

## Systematic Analysis Using Chinese Remainder Theorem

### What is the Chinese Remainder Theorem?

The Chinese Remainder Theorem (CRT) is a powerful mathematical tool that tells us how to combine information from different modular arithmetic systems. In simple terms:

**If you know what remainder a number leaves when divided by 3, and what remainder it leaves when divided by 4, then CRT tells you exactly what remainder it leaves when divided by 12.**

This works because 3 and 4 are coprime (they share no common factors other than 1), and 3 × 4 = 12.

**Why this helps us**: We have constraints from modulo 3 and modulo 4. CRT lets us combine them to get stronger constraints modulo 12, which will make our contradiction much clearer.

### Combining Our Modular Constraints

**From modulo 3**: Each $x_i$ must be either $0 \pmod{3}$ or $2 \pmod{3}$.

**From modulo 4**: Each $x_{i+1}$ must satisfy:
$$x_{i+1} \equiv \begin{cases}
2x_i + 1 \pmod{4} & \text{if } k_i = 0 \\
2 \pmod{4} & \text{if } k_i = 1 \\
0 \pmod{4} & \text{if } k_i \geq 2
\end{cases}$$

**Using CRT**: The only values that can satisfy both constraints modulo 12 are:
$$x_i \equiv 0, 2, 6, 8, 10 \pmod{12}$$

This is because:
- Values $\equiv 1, 4, 5, 7, 9, 11 \pmod{12}$ violate the modulo 3 constraint
- Values $\equiv 1, 3, 5, 7, 9, 11 \pmod{12}$ violate the modulo 4 constraint
- Only $0, 2, 6, 8, 10 \pmod{12}$ satisfy both

### Case Analysis

**Case 1**: All $k_i = 0$

If every $k_i = 0$, then the system becomes:
$$2x_i + 1 = 6x_{i+1} + 4$$
$$2x_i = 6x_{i+1} + 3$$
$$x_i = 3x_{i+1} + \frac{3}{2}$$

This requires $x_{i+1}$ to be odd (since $3x_{i+1} + \frac{3}{2}$ must be an integer), and $x_i > x_{i+1}$. This creates a strictly decreasing sequence that cannot close into a cycle.

**Case 2**: All $k_i = 1$

If every $k_i = 1$, then from modulo 4, all $x_{i+1} \equiv 2 \pmod{4}$. Combined with modulo 3, this forces all $x_i$ to be either $2 \pmod{12}$ or $6 \pmod{12}$.

The system becomes:
$$(2x_i + 1) \cdot 2 = 6x_{i+1} + 4$$
$$4x_i + 2 = 6x_{i+1} + 4$$
$$2x_i = 3x_{i+1} + 1$$

This means $x_i > x_{i+1}$ for all $i$, which contradicts the requirement that the cycle closes back to $x_0$.

**Case 3**: All $k_i \geq 2$

If every $k_i \geq 2$, then from modulo 4, all $x_{i+1} \equiv 0 \pmod{4}$. Combined with modulo 3, this forces all $x_i$ to be either $0 \pmod{12}$ or $8 \pmod{12}$.

But then the cycle cannot close because:
- If $x_0 \equiv 0 \pmod{12}$, then $x_n \equiv 0 \pmod{12}$
- The last equation requires $(2x_n + 1) \cdot 2^{k_n} = 6x_0 + 4$
- If $x_n \equiv 0 \pmod{12}$, then $2x_n + 1 \equiv 1 \pmod{12}$
- So $(2x_n + 1) \cdot 2^{k_n} \equiv 2^{k_n} \pmod{12}$
- But $6x_0 + 4 \equiv 4 \pmod{12}$
- Therefore $2^{k_n} \equiv 4 \pmod{12}$, which is impossible since $2^{k_n} \equiv 2, 4, 8 \pmod{12}$ for $k_n \geq 1$

**Case 4**: Mixed $k_i$ values

If we have a mix of different $k_i$ values, the constraints become inconsistent. For example:
- If $k_i = 0$, then $x_{i+1} \equiv 2x_i + 1 \pmod{4}$
- If $k_{i+1} \geq 1$, then $x_{i+2} \equiv 2 \pmod{4}$ or $0 \pmod{4}$

This creates a chain where the parity constraints cannot be satisfied consistently around the entire cycle, leading to contradictions.

### Final Contradiction

The key insight is that **all $x_i$ must be distinct positive integers**, but our CRT analysis shows they can only take values from a very limited set modulo 12: $\{0, 2, 6, 8, 10\}$.

If any $k_i \geq 2$, this forces subsequent $x_j$ into an even smaller set, making it impossible to satisfy the distinctness requirement while closing the cycle.

**Therefore, no solution exists for $n \geq 2$**.

**Conclusion**: No solution exists for $n \geq 2$. The only possible cycle is the trivial $n = 1$ case, which corresponds to the known 1→4→1 cycle in Collatz.

