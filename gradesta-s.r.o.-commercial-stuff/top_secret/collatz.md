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

### Bounding $k_i$

If we multiply all $N$ equations together, the terms on both sides can be grouped:
$$\prod_{i=0}^{N-1} 2(3x_i + 2) = \prod_{i=0}^{N-1} (2x_{i+1}+1) \cdot 2^{k_i}$$

Since the indices are cyclic, the product $\prod (2x_{i+1}+1)$ is the same as $\prod (2x_i+1)$.
$$2^N \prod_{i=0}^{N-1} (3x_i + 2) = \left(\prod_{i=0}^{N-1} (2x_i+1)\right) \cdot 2^{\sum k_i}$$

**3. The Core Ratio**

Now, we rearrange this into the product of ratios you were analyzing:
$$\prod_{i=0}^{N-1} \frac{3x_i + 2}{2x_i + 1} = \frac{2^{\sum k_i}}{2^N} = 2^{\sum k_i - N}$$

This equation states that the product of $N$ rational numbers must equal a power of 2.

**4. Bounding the Ratio**

Let's analyze the function $f(x) = \frac{3x+2}{2x+1}$.
-   As $x \to \infty$, the limit of $f(x)$ is $\frac{3}{2} = 1.5$.
-   The function is strictly decreasing for positive $x$.
-   Since the loop cannot contain the number 1 (which would lead to the trivial $1 \to 4 \to 2 \to 1$ loop), the smallest possible odd number is 3, which corresponds to $x=1$.
-   The maximum value of the function for any $x \ge 1$ is $f(1) = \frac{3(1)+2}{2(1)+1} = \frac{5}{3} \approx 1.667$.

Therefore, for any $x_i$ in a non-trivial loop:
$$1.5 < \frac{3x_i + 2}{2x_i + 1} \le \frac{5}{3}$$

**5. Bounding the Average Value of *k***

Taking the product of all $N$ terms in the loop, we get:
$$(1.5)^N < \prod_{i=0}^{N-1} \frac{3x_i + 2}{2x_i + 1} \le \left(\frac{5}{3}\right)^N$$

Substituting our result from step 3:
$$(1.5)^N < 2^{\sum k_i - N} \le \left(\frac{5}{3}\right)^N$$

Now, take the base-2 logarithm of the entire inequality:
$$N \cdot \log_2(1.5) < \sum k_i - N \le N \cdot \log_2(5/3)$$

Let's find the average value of $k$, which is $\frac{\sum k_i}{N}$:
$$\log_2(1.5) < \frac{\sum k_i}{N} - 1 \le \log_2(5/3)$$
$$1 + \log_2(1.5) < \frac{\sum k_i}{N} \le 1 + \log_2(5/3)$$

Calculating the numerical values:
-   $\log_2(1.5) \approx 0.585$
-   $\log_2(5/3) \approx 0.737$

This gives us the final, rigorous bounds:
$$1.585 < \text{Average}(k) \le 1.737$$

For any non-trivial cycle to exist, the average number of divisions by 2 following each ```3n+1``` step must be strictly between $\log_2(3) \approx 1.585$ and $1+\log_2(5/3) \approx 1.737$.


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

Simplifying a bit:

$$
x = \frac{3\left(\frac{3\left(\frac{3\left(\frac{3x + 2}{2^{k_0}} - \frac{1}{2}\right) + 2}{2^{k_1}} - \frac{1}{2}\right) + 2}{2^{k_2}} - \frac{1}{2}\right) + 2}{2^{k_3}} - \frac{1}{2}
$$


The general equation for depth n where n>1 is:

$$
x = \frac{2^1 \cdot 3^{n-1} + 3^{n-2} \cdot 2^{k_0-1} + 3^{n-3} \cdot 2^{k_0+k_1-1} + \cdots + 3^1 \cdot 2^{k_0+\cdots+k_{n-3}-1} + 2^{k_0+\cdots+k_{n-2}-1} - 2^{k_0+\cdots+k_{n-1}-1}}{2^{k_0+\cdots+k_{n-1}} - 3^n}
$$

where $k_0, k_1, \ldots, k_{n-1}$ are the exponents of the $n$ consecutive "odd steps" in the Collatz sequence, and $n \geq 2$.

### Explicit Forms for Small Depths

**Depth 1:**
$$x = \frac{2^1 - 2^{k_0-1}}{2^{k_0} - 3^1}$$

**Depth 2:**
$$x = \frac{2^1 \cdot 3^1 + 2^{k_0-1} - 2^{k_0+k_1-1}}{2^{k_0+k_1} - 3^2}$$

**Depth 3:**
$$x = \frac{2^1 \cdot 3^2 + 3^1 \cdot 2^{k_0-1} + 2^{k_0+k_1-1} - 2^{k_0+k_1+k_2-1}}{2^{k_0+k_1+k_2} - 3^3}$$

**Depth 4:**
$$x = \frac{2^1 \cdot 3^3 + 3^2 \cdot 2^{k_0-1} + 3^1 \cdot 2^{k_0+k_1-1} + 2^{k_0+k_1+k_2-1} - 2^{k_0+k_1+k_2+k_3-1}}{2^{k_0+k_1+k_2+k_3} - 3^4}$$

### Turning this into a new conjecture

$$
\begin{matrix}\begin{align}
&2 \cdot 3^{7} \\
&+ 3^{6} \cdot 2^{l_7} \\
&+ 3^{5} \cdot 2^{l_6} \\
&+ 3^{4} \cdot 2^{l_5} \\
&+ 3^{3} \cdot 2^{l_4} \\
&+ 3^{2} \cdot 2^{l_3} \\
&+ 3^{1} \cdot 2^{l_2} \\
&+ 3^{0} \cdot 2^{l_1} \\
&- 2^{l_0}
\end{align}\end{matrix}
$$

$$
N=\sum_{i=1}^{n} 3^{n-i} \cdot 2^{l_i} + 2 \cdot 3^{n} - 2^{l_0}
$$

or, equivalently,

$$
N = 2 \cdot 3^{n} + \sum_{i=1}^{n} 3^{n-i} \cdot 2^{l_i} - 2^{l_0}
$$

where $n$ is any positive integer and $l_0, l_1, \ldots, l_n$ are non-negative integers with $l_i < l_{i+1}$ for all $i$.

Restrictions

$$
l_i < l_{i+1}
$$

$l_i$ are positive integers >= 0.

The above sum is not divisible by $D = 2^{l_n+1}-3^n$ if $\frac{N}{D} > 0$ and $n>1$.