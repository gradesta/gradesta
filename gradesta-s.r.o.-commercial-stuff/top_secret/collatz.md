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

$$6a + 4 = (2a+1) \cdot 2^k$$

be solved. Because we would need the left-hand side of the graph to point to the right-hand side and the right-hand side to point to the left-hand side.

This is solvable for $a=0$. Interestingly, $k$ then represents the number of → labeled edges in the walk (or if you prefer) the number of even numbers in the non-condensed cyclic walk.

In order for a cycle between walk trees with two edges to exist, we would need:

$$
\left\{
\begin{aligned}
6a + 4 = (2b+1) \cdot 2^k \\
6b + 4 = (2a+1) \cdot 2^l
\end{aligned}
\right.
$$

To hold. That is, such a cycle consists of two distinct walk trees. And the right-hand side of the first walk tree must point to the left-hand side of the second and the right-hand side of the second must point to the left-hand side of the first. This is, however, unsolvable for distinct positive integer values of $a$ and $b$ and positive integer values of $k$ and $l$.

To show this is unsolvable lets first simplify the equations.

$$
\left\{
\begin{aligned}
    6a + 4 &= b \cdot 2^{k+1} + 2^k \\
    6b + 4 &= a \cdot 2^{l+1} + 2^l
\end{aligned}
\right.
$$

If $k=1$ then $b>a$ and if that's true than $l>1$. If $k=2 (or greater)$ then $b<a$ this means that $k\neq l$.


$$
\left\{
\begin{aligned}
    a &= \frac{b \cdot 2^{k} + 2^{k-1} - 2}{3} \\
    b &= \frac{a \cdot 2^{l} + 2^{l-1} -2}{3}
\end{aligned}
\right.
$$

Since a and b are both whole numbers we know that.

$$b \cdot 2^{k} + 2^{k-1} - 2$$

and

$$a \cdot 2^{l} + 2^{l-1} -2$$

are both divisible by 3.

Let us analyze the divisibility by 3 for the expressions in question.

Recall:
- $a = \frac{b \cdot 2^{k} + 2^{k-1} - 2}{3}$
- $b = \frac{a \cdot 2^{l} + 2^{l-1} - 2}{3}$

For $a$ and $b$ to be integers, the numerators must be divisible by 3.

Let us consider $2^n \bmod 3$:
- $2^1 \equiv 2 \pmod{3}$
- $2^2 \equiv 1 \pmod{3}$
- $2^3 \equiv 2 \pmod{3}$
- $2^4 \equiv 1 \pmod{3}$
- and so on, so $2^n \equiv 2$ if $n$ is odd, $1$ if $n$ is even.

Now, analyze $b \cdot 2^k + 2^{k-1} - 2 \bmod 3$:

- If $k$ is even: $2^k \equiv 1$, $2^{k-1} \equiv 2$
- If $k$ is odd: $2^k \equiv 2$, $2^{k-1} \equiv 1$

So, for $k$ even:
- $b \cdot 1 + 2 - 2 \equiv b \pmod{3}$

For $k$ odd:
- $b \cdot 2 + 1 - 2 \equiv 2b - 1 \pmod{3}$

For the numerator to be divisible by 3, we require:
- If $k$ even: $b \equiv 0 \pmod{3}$
- If $k$ odd: $2b - 1 \equiv 0 \pmod{3} \implies 2b \equiv 1 \pmod{3} \implies b \equiv 2 \pmod{3}$

Similarly, for $a \cdot 2^l + 2^{l-1} - 2$:
- If $l$ even: $a \equiv 0 \pmod{3}$
- If $l$ odd: $a \equiv 2 \pmod{3}$

Now, let's consider all possible parity combinations for $k$ and $l$:

---

**Case 1: $k$ and $l$ both even**

From above:
- $b \equiv 0 \pmod{3}$
- $a \equiv 0 \pmod{3}$

Let $a = 3A$, $b = 3B$ for integers $A, B \geq 0$.

Plug $b = 3B$ into the formula for $a$:
- $a = \frac{3B \cdot 2^k + 2^{k-1} - 2}{3} = B \cdot 2^k + \frac{2^{k-1} - 2}{3}$

But $a$ must also be a multiple of 3, so $a = 3A$:
- $3A = B \cdot 2^k + \frac{2^{k-1} - 2}{3}$
- $3A - B \cdot 2^k = \frac{2^{k-1} - 2}{3}$

The right side is an integer only if $2^{k-1} \equiv 2 \pmod{3}$, i.e., $k-1$ is odd, i.e., $k$ is even (which it is in this case). So $2^{k-1} - 2 \equiv 0 \pmod{3}$, so the right side is an integer.

But for $A, B \geq 0$, $3A - B \cdot 2^k$ is only divisible by 3 if $B \cdot 2^k \equiv 0 \pmod{3}$, i.e., $B \equiv 0 \pmod{3}$ or $2^k \equiv 0 \pmod{3}$, but $2^k$ is never $0 \pmod{3}$, so $B \equiv 0 \pmod{3}$, i.e., $b$ is a multiple of 9, and so on. But then, recursively, $a$ and $b$ must be divisible by higher and higher powers of 3, and the only solution is $a = b = 0$.

---

**Case 2: $k$ even, $l$ odd**

- $b \equiv 0 \pmod{3}$
- $a \equiv 2 \pmod{3}$

Let $b = 3B$, $a = 3A + 2$.

Plug $b$ into the formula for $a$:
- $a = B \cdot 2^k + \frac{2^{k-1} - 2}{3}$

But $a \equiv 2 \pmod{3}$, so $B \cdot 2^k + \frac{2^{k-1} - 2}{3} \equiv 2 \pmod{3}$. But $B \cdot 2^k \equiv 0 \pmod{3}$, so $\frac{2^{k-1} - 2}{3} \equiv 2 \pmod{3}$. But as above, $\frac{2^{k-1} - 2}{3}$ is $0$ or $1$ mod 3, never $2$. Contradiction.

---

**Case 3: $k$ odd, $l$ even**

- $b \equiv 2 \pmod{3}$
- $a \equiv 0 \pmod{3}$

Let $b = 3B + 2$, $a = 3A$.

Plug $b$ into the formula for $a$:
- $a = \frac{(3B + 2) \cdot 2^k + 2^{k-1} - 2}{3} = B \cdot 2^{k+1} + \frac{2 \cdot 2^k + 2^{k-1} - 2}{3}$

But $a$ is a multiple of 3, so $B \cdot 2^{k+1}$ is always even, but the remaining term must also be divisible by 3. But $2 \cdot 2^k + 2^{k-1} - 2$ is not generally divisible by 3 for $k$ odd, and checking small values shows no positive integer solutions for $A, B$.

---

**Case 4: $k$ and $l$ both odd**

- $b \equiv 2 \pmod{3}$
- $a \equiv 2 \pmod{3}$

Let $a = 3A + 2$, $b = 3B + 2$.

Plug $b$ into the formula for $a$:
- $a = B \cdot 2^{k+1} + \frac{2 \cdot 2^k + 2^{k-1} - 2}{3}$

But $a \equiv 2 \pmod{3}$, so $B \cdot 2^{k+1}$ is always even, and the remaining term must be congruent to 2 mod 3. But as above, this is not possible for any $k \geq 1$ and integer $B$.

---

In all cases, the only integer solution is $a = b = 0$. For all other positive integer values, the system is unsolvable modulo 3, and thus unsolvable in integers.

This completes the proof for the simple case, with all cases checked explicitly.



## Larger Cycles

We can go onwards to larger cycles like:

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

$$6*0 + 4 = (2*x_1+1)*2^{k_0}$$

Now obviously the only possible value for $x_1$ is 0. There is always only one possible value in this direction.

Lets start with $x_0 = 4$ and go for a walk. $x_0=4$ represents starting anywhere on the walk tree with the odd number 9 in it and walking down the Collatz graph. From this walk tree we can get to exactly one other walk tree.

$$6*4 + 4 = (2*x_1+1)*2^{k_1}$$

$x_1=3$ and $k_1=2$ here clearly. Thus we've gotten to the walk tree with a 7 in it.

$$6*3 + 4 = (2*x_2+1)*2^{k_2}$$

$x_2=5$, $k_2=1$.

$$6*5 + 4 = (2*x_3+1)*2^{k_3}$$

$x_3=8$, $k_3=1$

$$6*8 + 4 = (2*x_4+1)*2^{k_4}$$

$x_4=6$, $k_4=3$

$$6*6 + 4 = (2*x_5+1)*2^{k_5}$$

$x_5=2$, $k_5=3$

$$6*2 + 4 = (2*x_6+1)*2^{k_6}$$

$x_6=0$, $k_6=4$

$$6*0 + 4 = (2*x_7+1)*2^{k_7}$$

$x_7=0$, $k_7=2$

Now obviously at this point we are stuck. No matter how many times we iterate from here we will never get $x_n=4$ which is where we started. Because there simply is no loop in the Collatz graph from 9 to 1. But we want to prove that this equation is unsolvable not just in finite cases that we test, but in all cases for all positive whole number values of x and n.

### Analyzing specific Cases

Before we analyze the possible cases, let's understand why some patterns can never happen in a cycle:

**"All $k_i$ ≥ 2 cannot happen"**: 

Returning to our equation:

$$6x_i + 4 = (2x_{i+1}+1) \cdot 2^{k_i}$$

We can rewrite as

$$6x_i + 4 = x_{i+1} 2^{k_i+1}+2^{k_i} $$

If $k_i$ is 2 then:

$$6x_i + 4 = x_{i+1} 2^3+2^2 $$

$$6x_i + 4 = 8x_{i+1}+4$$

Clearly $x_{i+1} < x_i$ if all $k_i ≥ 2$ but this violates the contstraint $x_0==x_{n+1}$.

**"All $k_i = 1$ cannot happen"**

If $k_i$ is 1 then:

$$6x_i + 4 = x_{i+1} 2+2 $$

In this case $x_{i+1} > x_i$ which violates $x_0==x_{n+1}$.

This insight lets us bound the sum of all $k_i$. Specifically, we've established that $\sum k_i > n$.

We can learn more about this sum by taking the product of the equations:

**Condensed Product Form:**

Multiplying all equations together gives:

$$\prod_{i=0}^{n} (6x_i + 4) = \prod_{i=0}^{n} \left((2x_i + 1) \cdot 2^{k_i}\right)$$

where $x_{n+1} = x_0$ due to the cyclic nature.

Let's expand the right-hand side:

$$\prod_{i=0}^{n} \left((2x_i + 1) \cdot 2^{k_i}\right) = \left(\prod_{i=0}^{n} (2x_i + 1)\right) \cdot \left(\prod_{i=0}^{n} 2^{k_i}\right)$$

Recall that the product of exponents is the exponent of the sum:

$$\prod_{i=0}^{n} 2^{k_i} = 2^{\sum_{i=0}^{n} k_i}$$

So the equation becomes:

$$\prod_{i=0}^{n} (6x_i + 4) = \left(\prod_{i=0}^{n} (2x_i + 1)\right) \cdot 2^{\sum_{i=0}^{n} k_i}$$

Now, to solve for the sum $\sum_{i=0}^{n} k_i$, divide both sides by $\prod_{i=0}^{n} (2x_i + 1)$:

$$\frac{\prod_{i=0}^{n} (6x_i + 4)}{\prod_{i=0}^{n} (2x_i + 1)} = 2^{\sum_{i=0}^{n} k_i}$$

Taking the base-2 logarithm of both sides gives:

$$\log_2\left(\frac{\prod_{i=0}^{n} (6x_i + 4)}{\prod_{i=0}^{n} (2x_i + 1)}\right) = \sum_{i=0}^{n} k_i$$

Thus, the sum of all $k_i$ is:

$$\boxed{\sum_{i=0}^{n} k_i = \log_2\left(\frac{\prod_{i=0}^{n} (6x_i + 4)}{\prod_{i=0}^{n} (2x_i + 1)}\right)}$$

---

**Interpreting the quantity $k_i$**

Because a logarithm turns products into sums, the boxed identity tells us that every term $k_i$ must satisfy

$$
k_i = \log_2\left(\frac{6x_i+4}{2x_i+1}\right)
$$

Hence the problem is equivalent to finding an upper bound for

$$
S = \sum_{i=0}^{n} \log_2\left(\frac{6x_i+4}{2x_i+1}\right)
$$

given the admissible values of the $x_i$.

**Bounding a single term**

Put $f(x) = \frac{6x+4}{2x+1}$, with $x > -\frac{1}{2}$.

A straightforward derivative check shows

$$
f'(x) = \frac{6(2x+1) - 2(6x+4)}{(2x+1)^2} = \frac{-2}{(2x+1)^2} < 0,
$$

so $f(x)$ is strictly decreasing on its domain. Therefore, the largest value of $f(x)$ occurs at the smallest allowed $x$.

Assume throughout that all $x_i \ge 0$ (the usual setting in combinatorial or number-theoretic problems). Then

$$
\begin{align*}
&\text{At } x=0: \quad f(0) = 4. \\
&\text{As } x \to \infty: \quad f(x) \to 3.
\end{align*}
$$

Thus for every $x_i \ge 0$ we have

$$
3 < f(x_i) \le 4,
$$

and consequently

$$
1.585\ldots = \log_2 3 < k_i \le \log_2 4 = 2.
$$

**A universal upper bound (non-negative $x_i$)**

Because each $k_i \le 2$,

$$
\boxed{S \le 2(n+1)}
$$

Equality occurs precisely when every $x_i = 0$.

**Tighter bounds under stronger hypotheses**

If the $x_i$ are additionally bounded below by some integer $m \ge 1$, you can sharpen the estimate using the same monotonicity argument:

$$
k_i \le \log_2\left(\frac{6m+4}{2m+1}\right)
\implies
S \le (n+1)\log_2\left(\frac{6m+4}{2m+1}\right)
$$

Examples:

$$
\begin{align*}
&x_i \ge 1: \quad k_i \le \log_2\left(\frac{10}{3}\right) \approx 1.737 \implies S \le 1.737(n+1). \\
&x_i \ge 2: \quad k_i \le \log_2\left(\frac{16}{5}\right) \approx 1.678 \implies S \le 1.678(n+1).
\end{align*}
$$

**Summary**


For the typical case $x_i \ge 0$, the simple and sharp bound is
$$
\begin{align*}
&\sum_{i=0}^{n} k_i \le 2(n+1). \\
\end{align*}
$$
$$\sum k_i > n$$
